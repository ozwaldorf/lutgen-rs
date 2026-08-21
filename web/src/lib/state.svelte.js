import { BY_ID, PALETTES } from "virtual:palettes";

export const LEVEL = 12;
const DEFAULT_PALETTE = "catppuccin-mocha";
const SELECTION_KEY = "lutgen.palette";
const RECENTS_KEY = "lutgen.recents";
const MAX_RECENTS = 6;

function read(key) {
  try {
    return JSON.parse(localStorage.getItem(key));
  } catch {
    return null;
  }
}

function write(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Private browsing or a full quota. The selection is not persisted.
  }
}

// Resolves the stored selection into a palette, falling back to the default.
function initialPalette() {
  const saved = read(SELECTION_KEY);
  if (saved && Array.isArray(saved.colors)) {
    const colors = saved.colors.filter((c) => /^#[0-9a-fA-F]{6}$/.test(c));
    if (colors.length) {
      return { id: "custom", name: `Custom (${colors.length})`, colors };
    }
  }
  return BY_ID.get(saved?.id) || BY_ID.get(DEFAULT_PALETTE) || PALETTES[0];
}

function initialRecents() {
  const saved = read(RECENTS_KEY);
  return Array.isArray(saved) ? saved.filter((id) => BY_ID.has(id)) : [];
}

/** Reactive app state, shared by every component. */
export const app = $state({
  palette: initialPalette(),
  recents: initialRecents(),
  radius: 12,
  lum: 0.7,
  preserve: true,
  // Fraction of the image showing the original.
  split: 0.33,
  imageSize: [0, 0],
  elapsed: 0,
  error: "",
});

export function selectPalette(palette) {
  app.palette = palette;
  if (palette.id === "custom") {
    write(SELECTION_KEY, { colors: palette.colors });
  } else {
    write(SELECTION_KEY, { id: palette.id });
    app.recents = [palette.id, ...app.recents.filter((r) => r !== palette.id)]
      .slice(0, MAX_RECENTS);
    write(RECENTS_KEY, app.recents);
  }
}

export function setCustomColors(colors) {
  selectPalette({
    id: "custom",
    name: `Custom (${colors.length})`,
    colors,
  });
}
