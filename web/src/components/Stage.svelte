<script>
  import { app } from "../lib/state.svelte.js";

  let { canvas = $bindable(), originalSrc, openFile } = $props();

  let stage = $state(null);
  let wrap = $state(null);
  let availableSize = $state([0, 0]);
  // dragenter and dragleave also fire when crossing child elements, so the
  // counter tracks depth instead of toggling.
  let depth = $state(0);

  const dragging = $derived(depth > 0);
  const clip = $derived(`inset(0 0 0 ${app.split * 100}%)`);
  const frameSize = $derived.by(() => {
    const [availableWidth, availableHeight] = availableSize;
    const [imageWidth, imageHeight] = app.imageSize;
    if (!availableWidth || !availableHeight || !imageWidth || !imageHeight) {
      return [0, 0];
    }

    const scale = Math.min(
      availableWidth / imageWidth,
      availableHeight / imageHeight,
    );
    return [imageWidth * scale, imageHeight * scale];
  });

  $effect(() => {
    if (!stage) return;

    const observer = new ResizeObserver(([entry]) => {
      const size = entry.contentBoxSize?.[0];
      availableSize = size
        ? [size.inlineSize, size.blockSize]
        : [entry.contentRect.width, entry.contentRect.height];
    });
    observer.observe(stage);
    return () => observer.disconnect();
  });

  function track(event) {
    const rect = canvas.getBoundingClientRect();
    app.split = Math.min(
      1,
      Math.max(0, (event.clientX - rect.left) / rect.width),
    );
  }

  function down(event) {
    wrap.setPointerCapture(event.pointerId);
    track(event);
  }

  function move(event) {
    if (wrap.hasPointerCapture(event.pointerId)) track(event);
  }

  const hasFiles = (event) =>
    Array.from(event.dataTransfer?.types || []).includes("Files");
</script>

<div
  class="stage"
  bind:this={stage}
  class:dragging
  role="region"
  aria-label="Image preview"
  ondragenter={(e) => {
    if (hasFiles(e)) {
      e.preventDefault();
      depth += 1;
    }
  }}
  ondragover={(e) => hasFiles(e) && e.preventDefault()}
  ondragleave={(e) => hasFiles(e) && (depth = Math.max(0, depth - 1))}
  ondrop={(e) => {
    e.preventDefault();
    depth = 0;
    openFile(e.dataTransfer.files[0]);
  }}
>
  <!-- The frame carries the source aspect ratio. Both layers fill it, with the
       canvas overlaying the original and clipped at the split. -->
  <div
    class="canvas-wrap"
    bind:this={wrap}
    style:width="{frameSize[0]}px"
    style:height="{frameSize[1]}px"
    role="slider"
    tabindex="0"
    aria-label="Comparison split"
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.round(app.split * 100)}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={(e) => wrap.releasePointerCapture(e.pointerId)}
    ondragstart={(e) => e.preventDefault()}
    onkeydown={(e) => {
      if (e.key === "ArrowLeft") app.split = Math.max(0, app.split - 0.02);
      if (e.key === "ArrowRight") app.split = Math.min(1, app.split + 0.02);
    }}
  >
    <img class="original" src={originalSrc} alt="" draggable="false" />
    <canvas bind:this={canvas} style:clip-path={clip}></canvas>

    <div class="divider" style:left="{app.split * 100}%">
      <span class="handle"></span>
    </div>
    <div class="tag tag-before">original</div>
    <div class="tag tag-after">lutgen</div>
  </div>

  <div class="drop">drop an image</div>

  {#if app.error}
    <div class="error">{app.error}</div>
  {/if}
</div>

<style>
  .stage {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    /* Quantized cell lattice, echoing the LUT grid in the logo. Dots rather
       than hairlines, because a 1px gradient stop lands on fractional device
       pixels under display scaling and antialiases to uneven thicknesses. */
    background-color: var(--bg);
    background-image: radial-gradient(var(--grey-700) 1px, transparent 1px);
    background-size: 22px 22px;
    background-position: 11px 11px;
  }

  .canvas-wrap {
    position: relative;
    flex: none;
    line-height: 0;
    box-shadow: 0 1px 0 var(--grey-700), 0 16px 48px rgba(0, 0, 0, 0.6);
    cursor: ew-resize;
    /* Prevents a native image drag or page selection while dragging the split. */
    user-select: none;
    -webkit-user-select: none;
  }

  .canvas-wrap img {
    -webkit-user-drag: none;
  }

  /* The frame already carries the source aspect ratio. Contain is a safeguard
     against distortion when intrinsic dimensions arrive late. */
  .original {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  canvas {
    display: block;
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  /* Compare divider --------------------------------------------------------- */

  .divider {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    pointer-events: none;
  }

  .divider::before {
    content: "";
    position: absolute;
    inset: 0;
    background: var(--white);
    mix-blend-mode: difference;
  }

  .handle {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 26px;
    height: 26px;
    transform: translate(-50%, -50%);
    border: 1px solid var(--white);
    background: var(--black);
  }

  .handle::before,
  .handle::after {
    content: "";
    position: absolute;
    top: 50%;
    width: 0;
    height: 0;
    border-block: 4px solid transparent;
    transform: translateY(-50%);
  }

  .handle::before {
    left: 5px;
    border-right: 5px solid var(--white);
  }

  .handle::after {
    right: 5px;
    border-left: 5px solid var(--white);
  }

  .tag {
    position: absolute;
    top: 0.5rem;
    font-family: var(--mono);
    font-size: 10px;
    /* The wrapper zeroes line-height to remove the inline gap under the
       canvas. Without restoring it, the background collapses behind the text. */
    line-height: 1.4;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    padding: 0.2rem 0.45rem;
    background: var(--black);
    color: var(--white);
    pointer-events: none;
    opacity: 0.9;
  }

  .tag-before {
    left: 0.5rem;
  }

  .tag-after {
    right: 0.5rem;
  }

  .drop {
    position: absolute;
    inset: 0.75rem;
    display: none;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--blue);
    background: color-mix(in srgb, var(--blue) 8%, transparent);
    color: var(--blue-lt);
    font-family: var(--mono);
    font-size: 12px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    pointer-events: none;
  }

  .stage.dragging .drop {
    display: flex;
  }

  .error {
    position: absolute;
    top: 1rem;
    left: 50%;
    transform: translateX(-50%);
    max-width: min(92%, 40rem);
    padding: 0.5rem 0.75rem;
    background: var(--grey-900);
    border-left: 2px solid var(--red);
    color: var(--red);
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
  }

  @media (max-width: 860px) {
    .stage {
      padding: 0.75rem;
    }
  }
</style>
