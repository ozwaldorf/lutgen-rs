<script>
  import Header from "./components/Header.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Stage from "./components/Stage.svelte";
  import { LutPipeline } from "./lib/pipeline.js";
  import { app, LEVEL } from "./lib/state.svelte.js";

  let canvas = $state(null);
  const SAMPLE = {
    src: `${import.meta.env.BASE_URL}sample.jpg`,
    name: "Seattle_Sunset",
    fileName: "Seattle_Sunset.jpg",
  };

  let originalSrc = $state(SAMPLE.src);
  // Basename of the loaded image, used to name exports.
  let imageName = $state(SAMPLE.name);
  let fileName = $state(SAMPLE.fileName);
  let pipeline = null;
  let lut = null;
  let lutDirty = true;
  let queued = false;

  // Palette bytes only cross the CPU/GPU boundary when the colors change.
  $effect(() => {
    const colors = app.palette.colors;
    if (!pipeline) return;
    pipeline.uploadPalette(colors);
    lutDirty = true;
    schedule();
  });

  // Luminance weighting and blur are shader inputs. Changing them neither
  // transforms nor re-uploads palette colors on the CPU.
  $effect(() => {
    void app.lum;
    void app.radius;
    if (!pipeline) return;
    lutDirty = true;
    schedule();
  });

  // Preserve mode only changes how the completed LUT is applied, so the
  // image repaints without regenerating it.
  $effect(() => {
    void app.preserve;
    if (!pipeline) return;
    schedule();
  });

  function loadImage(src, revoke = false) {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      app.error = "";
      pipeline?.uploadImage(img);
      app.imageSize = pipeline?.imageSize ?? [0, 0];
      // The same source backs the comparison layer beneath the canvas and
      // must stay alive until that <img> has decoded.
      originalSrc = src;
      schedule();
    };
    img.onerror = () => {
      if (revoke) URL.revokeObjectURL(src);
      app.error = `Could not load image: ${src}`;
    };
    img.src = src;
  }

  // Coalesce input into one render per animation frame.
  function schedule() {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      render();
    });
  }

  function render() {
    if (!pipeline?.imageTexture) return;
    try {
      pipeline.timeRender(() => {
        if (lutDirty || !lut) {
          lut = pipeline.generateLut({
            level: LEVEL,
            radius: app.radius,
            lumFactor: app.lum,
          });
          lutDirty = false;
        }
        pipeline.applyLut(lut, { preserve: app.preserve });
      });
      app.error = "";
      app.elapsed = pipeline.elapsed;
    } catch (err) {
      app.error = String(err.message || err);
    }
  }

  // Creates the GL context once the canvas exists, then loads the sample.
  $effect(() => {
    if (!canvas || pipeline) return;
    try {
      pipeline = new LutPipeline(canvas);
    } catch (err) {
      app.error = String(err.message || err);
      return;
    }
    pipeline.uploadPalette(app.palette.colors);
    loadSample();
  });

  function loadSample() {
    imageName = SAMPLE.name;
    fileName = SAMPLE.fileName;
    loadImage(SAMPLE.src);
  }

  function openFile(file) {
    if (!file?.type.startsWith("image/")) return;
    imageName = file.name.replace(/\.[^.]+$/, "");
    fileName = file.name;
    loadImage(URL.createObjectURL(file), true);
  }

  function saveImage() {
    canvas.toBlob((blob) =>
      downloadBlob(blob, `${imageName}-${app.palette.id}.jpg`),
      "image/jpeg",
      0.95,
    );
  }

  function saveLut() {
    const data = pipeline.readHaldClut(lut, LEVEL, app.preserve);
    const out = document.createElement("canvas");
    out.width = data.width;
    out.height = data.height;
    out.getContext("2d").putImageData(data, 0, 0);
    out.toBlob((blob) => downloadBlob(blob, `${app.palette.id}-hald-${LEVEL}.png`));
  }

  function downloadBlob(blob, filename) {
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    link.click();
    URL.revokeObjectURL(url);
  }
</script>

<Header {openFile} {saveImage} {saveLut} {fileName} />

<main>
  <Stage bind:canvas {originalSrc} {openFile} />
  <Sidebar />
</main>
