<script>
  let { openFile, saveImage, saveLut, fileName } = $props();

  let input = $state(null);
</script>

<header>
  <img class="mark" src="{import.meta.env.BASE_URL}lutgen.png" alt="" />
  <h1>Lutgen Web</h1>

  <span class="file-name">{fileName}</span>
  <input
    bind:this={input}
    type="file"
    accept="image/*"
    hidden
    onchange={(e) => openFile(e.currentTarget.files[0])}
  />

  <div class="toolbar">
    <button
      class="action labeled"
      aria-label="Open image"
      onclick={() => input.click()}
      title="Open image"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M1.5 4.5h5l1.5 2h6.5v7h-13z" />
        <path d="M1.5 4.5v-2h4l1.5 2" />
      </svg>
      <span>Open</span>
    </button>
    <button
      class="action labeled"
      aria-label="Export processed image as JPEG"
      onclick={saveImage}
      title="Export JPEG"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M8 1.5v9M4.5 7 8 10.5 11.5 7M2 13.5h12" />
      </svg>
      <span>Save Image</span>
    </button>
    <button
      class="action labeled"
      aria-label="Export HALD CLUT"
      onclick={saveLut}
      title="Export LUT"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M2 2h5v5H2zM9 2h5v5H9zM2 9h5v5H2zM9 9h5v5H9z" />
      </svg>
      <span>Export LUT</span>
    </button>
  </div>
</header>

<style>
  header {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0 0.875rem;
    height: 48px;
    flex: none;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }

  .mark {
    width: 28px;
    height: 28px;
    display: block;
    image-rendering: pixelated;
  }

  h1 {
    position: relative;
    top: -2px;
    font-size: 0.875rem;
    font-weight: 600;
    line-height: 1;
    margin: 0 0.25rem;
    letter-spacing: -0.01em;
  }

  .file-name {
    min-width: 0;
    max-width: min(32vw, 20rem);
    margin: 0 0.25rem;
    padding-left: 0.625rem;
    border-left: 1px solid var(--line);
    overflow: hidden;
    color: var(--grey-400);
    font-family: var(--mono);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toolbar {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.625rem;
  }

  .action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 1px solid transparent;
    background: transparent;
    color: var(--grey-300);
  }

  .action:hover {
    border-color: var(--line);
    background: var(--grey-800);
    color: var(--white);
  }

  .action svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.25;
    stroke-linecap: square;
    stroke-linejoin: miter;
  }

  .action.labeled {
    width: auto;
    gap: 0.35rem;
    padding: 0 0.45rem;
  }

</style>
