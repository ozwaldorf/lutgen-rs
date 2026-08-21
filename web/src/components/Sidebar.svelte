<script>
  import { app, LEVEL } from "../lib/state.svelte.js";
  import PalettePicker from "./PalettePicker.svelte";

  // Format a millisecond duration in the smallest unit that keeps it >= 1.
  const UNITS = [
    [1000, "s"],
    [1, "ms"],
    [1e-3, "us"],
    [1e-6, "ns"],
  ];

  function duration(ms) {
    if (!(ms > 0)) return "0";
    const [scale, unit] = UNITS.find(([s]) => ms >= s) || UNITS.at(-1);
    const value = ms / scale;
    // Keep 3 significant digits, dropping trailing zeros.
    const digits = value < 10 ? 2 : value < 100 ? 1 : 0;
    return `${Number(value.toFixed(digits))}${unit}`;
  }

  const HINTS = {
    radius:
      "Gaussian sigma across the LUT. Higher values blend palette colors further into the surrounding color space.",
    lum: "Weights lightness against chroma when finding the nearest palette color. At 0 only chroma is matched.",
    preserve: "Keeps the source lightness, remapping chroma only.",
  };

  // The lutgen invocation matching the current settings.
  const command = $derived.by(() => {
    const flags = [`-l ${LEVEL}`, `-r ${app.radius}`];
    if (app.lum !== 1) flags.push(`--lum ${app.lum}`);
    if (app.preserve) flags.push("-P");
    // Custom colors are trailing positionals after `--`. Palettes use -p.
    return app.palette.id === "custom"
      ? `lutgen apply ${flags.join(" ")} image.jpg -- ${app.palette.colors.join(" ")}`
      : `lutgen apply -p ${app.palette.id} ${flags.join(" ")} image.jpg`;
  });

  const elapsed = $derived(duration(app.elapsed));

  let copied = $state(false);
  let timer;

  // The sidebar scrolls and would clip the hint, so it renders in a fixed
  // layer outside the panel.
  let tip = $state(null);

  // Anchored vertically to the icon and flush with the sidebar's inner edge,
  // keeping the bubble inside the viewport.
  function showTip(event, text) {
    const icon = event.currentTarget.getBoundingClientRect();
    const panel = event.currentTarget.closest("aside").getBoundingClientRect();
    tip = {
      text,
      right: window.innerWidth - panel.right + 16,
      top: icon.bottom + 6,
    };
  }

  async function copyCommand() {
    try {
      await navigator.clipboard.writeText(command);
    } catch {
      // Clipboard access requires a secure context.
      app.error = "Clipboard unavailable outside https or localhost";
      return;
    }
    copied = true;
    clearTimeout(timer);
    timer = setTimeout(() => (copied = false), 1200);
  }
</script>

<!-- Sits outside the <label> so clicking the label never activates it. -->
{#snippet hint(text, label)}
  <button
    type="button"
    class="tip"
    aria-label="What is {label}?"
    onmouseenter={(e) => showTip(e, text)}
    onmouseleave={() => (tip = null)}
    onfocus={(e) => showTip(e, text)}
    onblur={() => (tip = null)}
  >
    ?
  </button>
{/snippet}

<aside>
  <PalettePicker />

  <section>
    <h2>Parameters</h2>

    <div class="field">
      <div class="lbl">
        <label for="radius">Blur Radius</label>
        {@render hint(HINTS.radius, "Blur Radius")}
        <output>{app.radius.toFixed(1)}</output>
      </div>
      <input
        id="radius"
        type="range"
        min="0.5"
        max="24"
        step="0.5"
        bind:value={app.radius}
      />
    </div>

    <div class="field">
      <div class="lbl">
        <label for="lum">Luminosity Weight</label>
        {@render hint(HINTS.lum, "Luminosity Weight")}
        <output>{app.lum.toFixed(2)}</output>
      </div>
      <input id="lum" type="range" min="0" max="2" step="0.05" bind:value={app.lum} />
    </div>

    <div class="lbl">
      <label for="preserve">Preserve Luminosity</label>
      {@render hint(HINTS.preserve, "Preserve Luminosity")}
      <input id="preserve" type="checkbox" bind:checked={app.preserve} />
    </div>
  </section>

  <section>
    <div class="row">
      <h2>Equivalent command</h2>
      <button class="link" onclick={copyCommand}>
        {copied ? "copied" : "copy"}
      </button>
    </div>
    <code>{command}</code>
  </section>

  <section>
    <h2>Debug</h2>
    <dl class="stats">
      <div><dt>Image</dt><dd>{app.imageSize[0]} x {app.imageSize[1]}</dd></div>
      <div><dt>LUT</dt><dd>HALD {LEVEL}</dd></div>
      <div><dt>Colors</dt><dd>{app.palette.colors.length}</dd></div>
      <div><dt>Render</dt><dd>{elapsed}</dd></div>
    </dl>
  </section>
</aside>

{#if tip}
  <div class="tooltip" style:right="{tip.right}px" style:top="{tip.top}px">
    {tip.text}
  </div>
{/if}

<style>
  .row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  /* Hover hint. Rendered in a fixed-position layer because the sidebar's
     overflow would clip it. */
  .tip {
    flex: none;
    padding: 0;
    background: none;
    border: none;
    border-radius: 0;
    color: var(--grey-500);
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1;
    cursor: help;
  }

  .tip:hover {
    background: none;
    color: var(--grey-100);
  }

  .tooltip {
    position: fixed;
    z-index: 50;
    max-width: 16rem;
    padding: 0.4rem 0.55rem;
    background: var(--grey-700);
    border: 1px solid var(--grey-600);
    color: var(--grey-100);
    font-size: 11.5px;
    line-height: 1.45;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.6);
    pointer-events: none;
  }

  /* Label, its help icon, then the value or control pushed to the right edge. */
  .lbl {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 13px;
  }

  .lbl label {
    cursor: pointer;
  }

  .lbl > :last-child {
    margin-left: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  /* Timing readout. Label and value rows with a dotted leader between them. */
  .stats {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .stats > div {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
  }

  .stats dt {
    color: var(--grey-500);
    font-size: 11.5px;
  }

  .stats > div::after {
    content: "";
    flex: 1;
    order: 1;
    height: 0;
    border-bottom: 1px dotted var(--grey-700);
  }

  .stats dd {
    order: 2;
    margin: 0;
    color: var(--grey-100);
    font-family: var(--mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
</style>
