<script>
  import { BY_ID, PALETTES } from "virtual:palettes";
  import { app, selectPalette, setCustomColors } from "../lib/state.svelte.js";

  let open = $state(false);
  let query = $state("");
  let cursor = $state(0);
  let search = $state(null);
  let list = $state(null);
  let activeColor = $state(null);
  let hexDraft = $state("");

  // Recents lead the unfiltered list. A query ranks prefix matches first.
  const results = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) {
      const pinned = app.recents.map((id) => BY_ID.get(id)).filter(Boolean);
      return [...pinned, ...PALETTES.filter((p) => !app.recents.includes(p.id))];
    }
    return PALETTES.map((p) => {
      const hay = `${p.id} ${p.name.toLowerCase()}`;
      const at = hay.indexOf(q);
      return at < 0 ? null : { p, rank: at === 0 || hay[at - 1] === " " ? 0 : 1 };
    })
      .filter(Boolean)
      .sort((a, b) => a.rank - b.rank)
      .map((s) => s.p);
  });

  // Each palette renders as a single gradient rather than one element per color.
  function strip(palette) {
    const n = palette.colors.length;
    const stops = palette.colors.map(
      (c, i) => `${c} ${(i / n) * 100}% ${((i + 1) / n) * 100}%`,
    );
    return `linear-gradient(90deg, ${stops.join(",")})`;
  }

  function choose(palette) {
    selectPalette(palette);
    activeColor = null;
    hexDraft = "";
    open = false;
  }

  function move(delta) {
    cursor = Math.min(results.length - 1, Math.max(0, cursor + delta));
    list?.querySelectorAll(".picker-row")[cursor]?.scrollIntoView({
      block: "nearest",
    });
  }

  function keydown(event) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      move(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      move(-1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (results[cursor]) choose(results[cursor]);
    } else if (event.key === "Escape") {
      open = false;
    }
  }

  function toggle() {
    open = !open;
    if (open) {
      query = "";
      cursor = 0;
      queueMicrotask(() => search?.focus());
    }
  }

  const normalizeHex = (color) => {
    const match = color.trim().match(/^#?([0-9a-fA-F]{6})$/);
    return match ? `#${match[1]}` : null;
  };

  const activeHsv = $derived(
    activeColor === null
      ? null
      : hexToHsv(normalizeHex(hexDraft) || app.palette.colors[activeColor]),
  );

  function hexToHsv(hex) {
    const value = parseInt(hex.slice(1), 16);
    const r = ((value >> 16) & 255) / 255;
    const g = ((value >> 8) & 255) / 255;
    const b = (value & 255) / 255;
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const delta = max - min;
    let h = 0;

    if (delta) {
      if (max === r) h = 60 * (((g - b) / delta) % 6);
      else if (max === g) h = 60 * ((b - r) / delta + 2);
      else h = 60 * ((r - g) / delta + 4);
    }

    return {
      h: h < 0 ? h + 360 : h,
      s: max ? delta / max : 0,
      v: max,
    };
  }

  function hsvToHex({ h, s, v }) {
    const c = v * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = v - c;
    const [r, g, b] =
      h < 60
        ? [c, x, 0]
        : h < 120
          ? [x, c, 0]
          : h < 180
            ? [0, c, x]
            : h < 240
              ? [0, x, c]
              : h < 300
                ? [x, 0, c]
                : [c, 0, x];
    return `#${[r, g, b]
      .map((channel) => Math.round((channel + m) * 255).toString(16).padStart(2, "0"))
      .join("")}`;
  }

  function setHsv(next) {
    if (activeColor !== null) updateColor(activeColor, hsvToHex(next));
  }

  function updateSaturationValue(event) {
    if (activeHsv === null) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const s = Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width));
    const v = Math.min(1, Math.max(0, 1 - (event.clientY - rect.top) / rect.height));
    setHsv({ ...activeHsv, s, v });
  }

  function startSaturationValue(event) {
    event.currentTarget.setPointerCapture(event.pointerId);
    updateSaturationValue(event);
  }

  function moveSaturationValue(event) {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      updateSaturationValue(event);
    }
  }

  function keySaturationValue(event) {
    if (!activeHsv || !event.key.startsWith("Arrow")) return;
    event.preventDefault();
    const step = event.shiftKey ? 0.1 : 0.01;
    const next = { ...activeHsv };
    if (event.key === "ArrowLeft") next.s = Math.max(0, next.s - step);
    if (event.key === "ArrowRight") next.s = Math.min(1, next.s + step);
    if (event.key === "ArrowDown") next.v = Math.max(0, next.v - step);
    if (event.key === "ArrowUp") next.v = Math.min(1, next.v + step);
    setHsv(next);
  }

  function editColor(index) {
    activeColor = activeColor === index ? null : index;
    hexDraft = activeColor === null ? "" : app.palette.colors[index];
  }

  function updateColor(index, color) {
    const colors = [...app.palette.colors];
    colors[index] = color;
    app.error = "";
    setCustomColors(colors);
    hexDraft = color;
  }

  function updateHex(value) {
    hexDraft = value;
    const color = normalizeHex(value);
    if (color && activeColor !== null) updateColor(activeColor, color);
  }

  function addColor() {
    const color = app.palette.colors.at(-1) || "#f4f4f4";
    const index = app.palette.colors.length;
    setCustomColors([...app.palette.colors, color]);
    activeColor = index;
    hexDraft = color;
  }

</script>

<svelte:window
  onpointerdown={(e) => {
    if (open && !e.target.closest(".picker")) open = false;
  }}
/>

<section>
  <div class="row">
    <h2>Palette</h2>
    <div class="row-actions">
      <button
        class="link"
        onclick={() => choose(PALETTES[Math.floor(Math.random() * PALETTES.length)])}
      >
        shuffle
      </button>
    </div>
  </div>

  <div class="picker">
    <button class="picker-trigger" onclick={toggle} aria-expanded={open}>
      <span class="picker-name">{app.palette.name}</span>
      <span class="picker-caret"></span>
    </button>

    {#if open}
      <div class="picker-panel">
        <input
          bind:this={search}
          bind:value={query}
          type="search"
          placeholder="Search {PALETTES.length} palettes"
          spellcheck="false"
          autocomplete="off"
          oninput={() => (cursor = 0)}
          onkeydown={keydown}
        />
        <div class="picker-list" bind:this={list} role="listbox" tabindex="-1">
          {#each results as palette, i (palette.id)}
            <button
              class="picker-row"
              class:active={i === cursor}
              class:current={palette.id === app.palette.id}
              class:recent={!query.trim() && i < app.recents.length}
              role="option"
              aria-selected={i === cursor}
              onclick={() => choose(palette)}
            >
              <span class="picker-row-name">{palette.name}</span>
              <span class="picker-row-strip" style:background-image={strip(palette)}
              ></span>
            </button>
          {:else}
            <div class="picker-empty">No palettes match</div>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <div class="swatches">
    {#each app.palette.colors as color, i}
      <button
        class="swatch"
        class:active={activeColor === i}
        type="button"
        style:background={color}
        title="Edit {color}"
        aria-label="Edit color {i + 1}: {color}"
        aria-expanded={activeColor === i}
        onclick={() => editColor(i)}
      ></button>
    {/each}
    <button
      class="swatch add-color"
      type="button"
      title="Add color"
      aria-label="Add color"
      onclick={addColor}
    >+</button>
  </div>

  {#if activeColor !== null && activeHsv}
    <div class="color-picker">
      <div class="picker-meta">
        <span>Color {activeColor + 1}</span>
        <input
          class:invalid={!normalizeHex(hexDraft)}
          type="text"
          value={hexDraft}
          maxlength="7"
          spellcheck="false"
          aria-label="Hex value for color {activeColor + 1}"
          oninput={(e) => updateHex(e.currentTarget.value)}
        />
      </div>
      <div
        class="saturation-value"
        style:--hue={activeHsv.h}
        role="slider"
        tabindex="0"
        aria-label="Saturation and brightness"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(activeHsv.s * 100)}
        aria-valuetext="{Math.round(activeHsv.s * 100)}% saturation, {Math.round(activeHsv.v * 100)}% brightness"
        onpointerdown={startSaturationValue}
        onpointermove={moveSaturationValue}
        onkeydown={keySaturationValue}
      >
        <span
          class="color-cursor"
          style:left="{activeHsv.s * 100}%"
          style:top="{(1 - activeHsv.v) * 100}%"
        ></span>
      </div>
      <input
        class="hue"
        type="range"
        min="0"
        max="359"
        step="1"
        value={activeHsv.h}
        aria-label="Hue"
        oninput={(e) => setHsv({ ...activeHsv, h: Number(e.currentTarget.value) })}
      />
    </div>
  {/if}
</section>

<style>
  .row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  .row-actions {
    display: flex;
    gap: 0.625rem;
  }

  .picker {
    position: relative;
  }

  .picker-trigger {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    background: var(--grey-800);
    border: 1px solid var(--line);
    padding: 0.4rem 0.5rem;
    font-size: 13px;
    text-align: left;
  }

  .picker-trigger:hover {
    background: var(--grey-800);
    border-color: var(--grey-500);
  }

  .picker-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .picker-caret {
    flex: none;
    width: 0;
    height: 0;
    border-inline: 4px solid transparent;
    border-top: 5px solid var(--grey-300);
  }

  .picker-panel {
    position: absolute;
    top: calc(100% + 3px);
    left: 0;
    right: 0;
    z-index: 20;
    background: var(--grey-900);
    border: 1px solid var(--grey-600);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.65);
    display: flex;
    flex-direction: column;
  }

  .picker-panel input[type="search"] {
    width: 100%;
    background: var(--grey-800);
    color: var(--text);
    border: none;
    border-bottom: 1px solid var(--line);
    border-radius: 0;
    padding: 0.5rem;
    font: inherit;
    font-size: 12.5px;
    appearance: none;
  }

  .picker-panel input[type="search"]::-webkit-search-cancel-button {
    appearance: none;
  }

  .picker-list {
    overflow-y: auto;
    max-height: 46vh;
    overscroll-behavior: contain;
  }

  .picker-row {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr 84px;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    border-left: 2px solid transparent;
    padding: 0.3rem 0.5rem;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }

  .picker-row:hover {
    background: var(--grey-800);
  }

  .picker-row.active {
    background: var(--grey-700);
    border-left-color: var(--accent);
  }

  .picker-row.current .picker-row-name {
    color: var(--accent);
  }

  .picker-row.recent .picker-row-name::after {
    content: "recent";
    margin-left: 0.4rem;
    font-family: var(--mono);
    font-size: 9px;
    letter-spacing: 0.04em;
    color: var(--grey-500);
    text-transform: uppercase;
  }

  .picker-row-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--grey-100);
  }

  .picker-row-strip {
    height: 12px;
    border: 1px solid var(--grey-800);
  }

  .picker-empty {
    padding: 0.5rem;
    color: var(--grey-500);
    font-size: 11.5px;
    text-align: center;
  }

  /* Palette ----------------------------------------------------------------- */

  .swatches {
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    grid-auto-rows: 20px;
    gap: 2px;
  }

  .swatch {
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 0;
    border: 1px solid transparent;
  }

  .swatch:hover,
  .swatch.active {
    border-color: var(--white);
    box-shadow: inset 0 0 0 1px var(--black);
  }

  .add-color {
    background: var(--grey-800);
    border-color: var(--grey-600);
    color: var(--grey-300);
    font-family: var(--mono);
    font-size: 14px;
    line-height: 1;
  }

  .add-color:hover {
    background: var(--grey-700);
    color: var(--white);
  }

  .color-picker {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.5rem;
    border: 1px solid var(--grey-600);
    background: var(--grey-800);
  }

  .picker-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    color: var(--grey-400);
    font-family: var(--mono);
    font-size: 9px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .picker-meta input {
    width: 5.25rem;
    padding: 0.15rem 0.3rem;
    border: 1px solid var(--grey-600);
    outline: none;
    background: var(--grey-900);
    color: var(--grey-200);
    font: inherit;
    text-align: right;
    text-transform: uppercase;
  }

  .picker-meta input:focus {
    border-color: var(--accent);
  }

  .picker-meta input.invalid {
    border-color: var(--red);
    color: var(--red);
  }

  .saturation-value {
    position: relative;
    height: 7rem;
    overflow: hidden;
    background:
      linear-gradient(to top, var(--black), transparent),
      linear-gradient(to right, var(--white), hsl(var(--hue) 100% 50%));
    cursor: crosshair;
    touch-action: none;
  }

  .color-cursor {
    position: absolute;
    width: 10px;
    height: 10px;
    border: 1px solid var(--white);
    border-radius: 50%;
    box-shadow: 0 0 0 1px var(--black);
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .hue {
    appearance: none;
    width: 100%;
    height: 12px;
    margin: 0;
    background: linear-gradient(
      to right,
      #f00,
      #ff0,
      #0f0,
      #0ff,
      #00f,
      #f0f,
      #f00
    );
  }

  .hue::-webkit-slider-runnable-track {
    height: 12px;
    background: transparent;
  }

  .hue::-moz-range-track {
    height: 12px;
    background: transparent;
  }

  .hue::-webkit-slider-thumb {
    appearance: none;
    width: 4px;
    height: 16px;
    margin-top: -2px;
    border: 1px solid var(--black);
    background: var(--white);
  }

  .hue::-moz-range-thumb {
    width: 4px;
    height: 16px;
    border: 1px solid var(--black);
    border-radius: 0;
    background: var(--white);
  }

</style>
