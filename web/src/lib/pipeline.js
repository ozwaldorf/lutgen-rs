// Pure-GPU gaussian blur LUT generation pipeline (WebGL2).
//
// Stages:
//   1. The identity 3D LUT is implicit: a texel's normalized coordinate is its
//      own color, derived in the remap shader from gl_FragCoord and the layer.
//   2. Nearest neighbor remap against the palette, evaluated in OKLab, written to
//      a RGBA16F TEXTURE_3D one layer at a time via framebufferTextureLayer.
//   3. Separable gaussian blur: three ping-pong passes along R, G, B. Strided 3D
//      reads carry no cache penalty on the GPU, so the axes need no transposes.
//   4. Final LUT is bound as a sampler3D with LINEAR filtering, giving hardware
//      trilinear interpolation when correcting the image.

const QUAD_VS = `#version 300 es
in vec2 a_pos;
void main() { gl_Position = vec4(a_pos, 0.0, 1.0); }
`;

// sRGB <-> OKLab. Constants match the `oklab` crate so GPU and CPU output agree.
const OKLAB = `
float srgbToLinear(float c) {
  return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);
}
float linearToSrgb(float c) {
  return c <= 0.0031308 ? c * 12.92 : 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}
vec3 srgbToOklab(vec3 c) {
  vec3 lin = vec3(srgbToLinear(c.r), srgbToLinear(c.g), srgbToLinear(c.b));
  float l = 0.4122214708 * lin.r + 0.5363325363 * lin.g + 0.0514459929 * lin.b;
  float m = 0.2119034982 * lin.r + 0.6806995451 * lin.g + 0.1073969566 * lin.b;
  float s = 0.0883024619 * lin.r + 0.2817188376 * lin.g + 0.6299787005 * lin.b;
  float l_ = pow(l, 1.0 / 3.0);
  float m_ = pow(m, 1.0 / 3.0);
  float s_ = pow(s, 1.0 / 3.0);
  return vec3(
    0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
    1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
    0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_
  );
}
vec3 oklabToSrgb(vec3 lab) {
  float l_ = lab.x + 0.3963377774 * lab.y + 0.2158037573 * lab.z;
  float m_ = lab.x - 0.1055613458 * lab.y - 0.0638541728 * lab.z;
  float s_ = lab.x - 0.0894841775 * lab.y - 1.2914855480 * lab.z;
  float l = l_ * l_ * l_;
  float m = m_ * m_ * m_;
  float s = s_ * s_ * s_;
  vec3 lin = vec3(
     4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
  );
  return clamp(
    vec3(linearToSrgb(lin.r), linearToSrgb(lin.g), linearToSrgb(lin.b)),
    0.0, 1.0
  );
}
`;

// Stage 2: nearest neighbor remap of the implicit identity LUT.
// The palette is an RGBA8 texture one texel tall. OKLab conversion and
// luminosity weighting happen here, so the CPU uploads only sRGB bytes.
const REMAP_FS = `#version 300 es
precision highp float;
precision highp sampler2D;
${OKLAB}
uniform sampler2D u_palette;
uniform int u_paletteSize;
uniform int u_size;
uniform int u_layer;
uniform float u_lumFactor;
out vec4 fragColor;

void main() {
  // This texel's grid position mapped to [0,1] is its own identity color.
  // The 8-bit round trip matches the CPU's u8 quantization of cell centers.
  float denom = float(u_size - 1);
  vec3 cell = vec3(floor(gl_FragCoord.xy), float(u_layer));
  vec3 rgb = floor(cell / denom * 255.0 + 0.5) / 255.0;

  vec3 lab = srgbToOklab(rgb);
  vec3 weighted = vec3(lab.x * u_lumFactor, lab.y, lab.z);

  int best = 0;
  float bestDist = 1e30;
  for (int i = 0; i < u_paletteSize; i++) {
    vec3 target = srgbToOklab(texelFetch(u_palette, ivec2(i, 0), 0).rgb);
    target.x *= u_lumFactor;
    vec3 d = weighted - target;
    float dist = dot(d, d);
    if (dist < bestDist) { bestDist = dist; best = i; }
  }

  vec3 target = srgbToOklab(texelFetch(u_palette, ivec2(best, 0), 0).rgb);
  // The LUT always carries complete OKLab values. Preserve mode substitutes
  // the source luminance at apply time and reuses these a/b channels.
  fragColor = vec4(target, 1.0);
}
`;

// Stage 3: one axis of the separable blur. u_axis selects R/G/B as a unit offset.
const BLUR_FS = `#version 300 es
precision highp float;
precision highp sampler3D;
uniform sampler3D u_src;
uniform sampler2D u_kernel;
uniform ivec3 u_axis;
uniform int u_half;
uniform int u_size;
uniform int u_layer;
out vec4 fragColor;

void main() {
  ivec3 coord = ivec3(ivec2(gl_FragCoord.xy), u_layer);
  int maxIdx = u_size - 1;
  vec3 sum = vec3(0.0);
  for (int k = 0; k <= u_half * 2; k++) {
    ivec3 offset = u_axis * (k - u_half);
    ivec3 s = clamp(coord + offset, ivec3(0), ivec3(maxIdx));
    sum += texelFetch(u_src, s, 0).rgb * texelFetch(u_kernel, ivec2(k, 0), 0).r;
  }
  fragColor = vec4(sum, 1.0);
}
`;

// Stage 4: apply the finished LUT to the source image with hardware trilinear.
const APPLY_FS = `#version 300 es
precision highp float;
precision highp sampler3D;
precision highp sampler2D;
${OKLAB}
uniform sampler2D u_image;
uniform sampler3D u_lut;
uniform vec2 u_resolution;
uniform int u_size;
uniform bool u_preserve;
out vec4 fragColor;

void main() {
  vec2 uv = gl_FragCoord.xy / u_resolution;
  uv.y = 1.0 - uv.y;
  vec4 src = texture(u_image, uv);

  // Shrink to texel centers so trilinear never samples outside the cube.
  float scale = float(u_size - 1) / float(u_size);
  float offset = 1.0 / (2.0 * float(u_size));
  vec3 lab = texture(u_lut, src.rgb * scale + offset).rgb;

  fragColor = vec4(
    u_preserve
      ? oklabToSrgb(vec3(srgbToOklab(src.rgb).x, lab.y, lab.z))
      : oklabToSrgb(lab),
    src.a
  );
}
`;

// Renders the 3D OKLab LUT into lutgen's 2D HALD layout. An RGBA8 target
// performs the final quantization before a single byte readback.
const HALD_FS = `#version 300 es
precision highp float;
precision highp int;
precision highp sampler3D;
${OKLAB}
uniform sampler3D u_lut;
uniform int u_size;
uniform int u_level;
uniform bool u_preserve;
out vec4 fragColor;

void main() {
  int x = int(gl_FragCoord.x);
  int y = int(gl_FragCoord.y);
  int r = x % u_size;
  int g = x / u_size + (y % u_level) * u_level;
  int b = y / u_level;

  ivec3 coord = ivec3(r, g, b);
  vec3 lab = texelFetch(u_lut, coord, 0).rgb;
  if (u_preserve) {
    float denom = float(u_size - 1);
    vec3 rgb = floor(vec3(coord) / denom * 255.0 + 0.5) / 255.0;
    lab.x = srgbToOklab(rgb).x;
  }
  fragColor = vec4(oklabToSrgb(lab), 1.0);
}
`;

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(`shader compile failed: ${log}`);
  }
  return shader;
}

function link(gl, fsSource) {
  const program = gl.createProgram();
  const vs = compile(gl, gl.VERTEX_SHADER, QUAD_VS);
  const fs = compile(gl, gl.FRAGMENT_SHADER, fsSource);
  gl.attachShader(program, vs);
  gl.attachShader(program, fs);
  gl.bindAttribLocation(program, 0, "a_pos");
  gl.linkProgram(program);
  gl.deleteShader(vs);
  gl.deleteShader(fs);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const log = gl.getProgramInfoLog(program);
    gl.deleteProgram(program);
    throw new Error(`program link failed: ${log}`);
  }
  return program;
}

function uniforms(gl, program) {
  const map = {};
  const count = gl.getProgramParameter(program, gl.ACTIVE_UNIFORMS);
  for (let i = 0; i < count; i++) {
    const { name } = gl.getActiveUniform(program, i);
    map[name] = gl.getUniformLocation(program, name);
  }
  return map;
}

export function hexToRgb(hex) {
  const v = parseInt(hex.replace("#", ""), 16);
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
}

// Kernel weights match GaussianBlurRemapper::build_kernel.
export function buildKernel(radius) {
  const half = Math.ceil(radius * 3.0);
  const weights = [];
  let sum = 0;
  for (let i = -half; i <= half; i++) {
    const weight = Math.exp(-(i * i) / (2.0 * radius * radius));
    weights.push(weight);
    sum += weight;
  }
  return {
    half,
    weights: Float32Array.from(weights, (weight) => weight / sum),
  };
}

export class LutPipeline {
  constructor(canvas) {
    const gl = canvas.getContext("webgl2", {
      antialias: false,
      preserveDrawingBuffer: true,
      premultipliedAlpha: false,
    });
    if (!gl) throw new Error("WebGL2 is not available in this browser");

    // Required to render into half-float 3D textures. RGBA16F linear filtering
    // is core WebGL2 and does not require OES_texture_float_linear.
    if (!gl.getExtension("EXT_color_buffer_float")) {
      throw new Error("EXT_color_buffer_float is required but unavailable");
    }

    this.gl = gl;
    this.canvas = canvas;
    this.programs = {
      remap: link(gl, REMAP_FS),
      blur: link(gl, BLUR_FS),
      apply: link(gl, APPLY_FS),
      hald: link(gl, HALD_FS),
    };
    this.uniforms = {
      remap: uniforms(gl, this.programs.remap),
      blur: uniforms(gl, this.programs.blur),
      apply: uniforms(gl, this.programs.apply),
      hald: uniforms(gl, this.programs.hald),
    };

    this.quad = gl.createVertexArray();
    gl.bindVertexArray(this.quad);
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 3, -1, -1, 3]),
      gl.STATIC_DRAW,
    );
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);

    this.fbo = gl.createFramebuffer();
    this.lutTextures = [null, null];
    this.lutSize = 0;
    this.paletteTexture = null;
    this.kernelTexture = null;
    this.imageTexture = null;
    this.imageSize = [0, 0];
    // Wall-clock time for the whole pipeline. GL commands queue
    // asynchronously, so a single finish at the end covers every stage.
    this.elapsed = 0;
  }

  // Times one full generate and apply cycle, flushing once at the end.
  timeRender(fn) {
    const start = performance.now();
    fn();
    this.gl.finish();
    this.elapsed = performance.now() - start;
  }

  // A 3D texture sized size^3, RGBA16F, linear-filtered for the final apply pass.
  createLutTexture(size) {
    const gl = this.gl;
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_3D, tex);
    gl.texStorage3D(gl.TEXTURE_3D, 1, gl.RGBA16F, size, size, size);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_WRAP_R, gl.CLAMP_TO_EDGE);
    return tex;
  }

  ensureLutTextures(size) {
    if (this.lutSize === size && this.lutTextures[0]) return;
    for (const tex of this.lutTextures) if (tex) this.gl.deleteTexture(tex);
    this.lutTextures = [
      this.createLutTexture(size),
      this.createLutTexture(size),
    ];
    this.lutSize = size;
  }

  // Palette colors upload verbatim. The remap shader owns color conversion.
  uploadPalette(colors) {
    const gl = this.gl;
    if (this.paletteTexture) gl.deleteTexture(this.paletteTexture);
    const data = new Uint8Array(colors.length * 4);
    colors.forEach((hex, i) => {
      data.set([...hexToRgb(hex), 255], i * 4);
    });
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texStorage2D(gl.TEXTURE_2D, 1, gl.RGBA8, colors.length, 1);
    gl.texSubImage2D(
      gl.TEXTURE_2D,
      0,
      0,
      0,
      colors.length,
      1,
      gl.RGBA,
      gl.UNSIGNED_BYTE,
      data,
    );
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    this.paletteTexture = tex;
    this.paletteSize = colors.length;
  }

  uploadKernel(weights) {
    const gl = this.gl;
    if (this.kernelTexture) gl.deleteTexture(this.kernelTexture);
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texStorage2D(gl.TEXTURE_2D, 1, gl.R32F, weights.length, 1);
    gl.texSubImage2D(
      gl.TEXTURE_2D,
      0,
      0,
      0,
      weights.length,
      1,
      gl.RED,
      gl.FLOAT,
      weights,
    );
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    this.kernelTexture = tex;
  }

  uploadImage(source) {
    const gl = this.gl;
    if (this.imageTexture) gl.deleteTexture(this.imageTexture);
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, source);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.imageTexture = tex;
    this.imageSize = [
      source.naturalWidth || source.width,
      source.naturalHeight || source.height,
    ];
  }

  // Binds one Z-slice of a 3D texture as the color attachment and draws the quad.
  drawLayer(target, layer, size) {
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbo);
    gl.framebufferTextureLayer(
      gl.FRAMEBUFFER,
      gl.COLOR_ATTACHMENT0,
      target,
      0,
      layer,
    );
    gl.viewport(0, 0, size, size);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  // Stages 1 through 3. Returns the texture holding the blurred LUT.
  generateLut({ level, radius, lumFactor }) {
    const gl = this.gl;
    const size = level * level;
    this.ensureLutTextures(size);
    gl.bindVertexArray(this.quad);
    gl.disable(gl.BLEND);
    gl.disable(gl.DEPTH_TEST);

    // Stages 1 and 2: the implicit identity remaps straight into slot 0.
    const ru = this.uniforms.remap;
    gl.useProgram(this.programs.remap);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.paletteTexture);
    gl.uniform1i(ru.u_palette, 0);
    gl.uniform1i(ru.u_paletteSize, this.paletteSize);
    gl.uniform1i(ru.u_size, size);
    gl.uniform1f(ru.u_lumFactor, lumFactor);
    for (let layer = 0; layer < size; layer++) {
      gl.uniform1i(ru.u_layer, layer);
      this.drawLayer(this.lutTextures[0], layer, size);
    }

    // Stage 3: three separable passes, ping-ponging between the two textures.
    const { half, weights } = buildKernel(radius);
    this.uploadKernel(weights);

    const bu = this.uniforms.blur;
    gl.useProgram(this.programs.blur);
    gl.uniform1i(bu.u_src, 0);
    gl.uniform1i(bu.u_kernel, 1);
    gl.uniform1i(bu.u_half, half);
    gl.uniform1i(bu.u_size, size);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.kernelTexture);

    let src = 0;
    for (
      const axis of [
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
      ]
    ) {
      gl.uniform3i(bu.u_axis, axis[0], axis[1], axis[2]);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_3D, this.lutTextures[src]);
      const dst = 1 - src;
      for (let layer = 0; layer < size; layer++) {
        gl.uniform1i(bu.u_layer, layer);
        this.drawLayer(this.lutTextures[dst], layer, size);
      }
      src = dst;
    }

    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    return this.lutTextures[src];
  }

  // Stage 4: sample the LUT through a sampler3D and draw to the canvas.
  applyLut(lut, { preserve }) {
    const gl = this.gl;
    const [width, height] = this.imageSize;
    this.canvas.width = width;
    this.canvas.height = height;

    const au = this.uniforms.apply;
    gl.useProgram(this.programs.apply);
    gl.bindVertexArray(this.quad);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.imageTexture);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_3D, lut);
    gl.uniform1i(au.u_image, 0);
    gl.uniform1i(au.u_lut, 1);
    gl.uniform2f(au.u_resolution, width, height);
    gl.uniform1i(au.u_size, this.lutSize);
    gl.uniform1i(au.u_preserve, preserve ? 1 : 0);

    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, width, height);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  // Renders the LUT as an RGBA8 HALD image and reads back the finished bytes.
  readHaldClut(lut, level, preserve) {
    const gl = this.gl;
    const size = level * level;
    const dim = level * level * level;
    const out = new Uint8Array(dim * dim * 4);

    const target = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, target);
    gl.texStorage2D(gl.TEXTURE_2D, 1, gl.RGBA8, dim, dim);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbo);
    gl.framebufferTexture2D(
      gl.FRAMEBUFFER,
      gl.COLOR_ATTACHMENT0,
      gl.TEXTURE_2D,
      target,
      0,
    );

    const hu = this.uniforms.hald;
    gl.useProgram(this.programs.hald);
    gl.bindVertexArray(this.quad);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_3D, lut);
    gl.uniform1i(hu.u_lut, 0);
    gl.uniform1i(hu.u_size, size);
    gl.uniform1i(hu.u_level, level);
    gl.uniform1i(hu.u_preserve, preserve ? 1 : 0);
    gl.viewport(0, 0, dim, dim);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.readPixels(0, 0, dim, dim, gl.RGBA, gl.UNSIGNED_BYTE, out);

    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.deleteTexture(target);
    return new ImageData(new Uint8ClampedArray(out.buffer), dim, dim);
  }
}
