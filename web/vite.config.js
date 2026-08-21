import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

const PALETTES_TOML = fileURLToPath(
  new URL("../crates/palettes/palettes.toml", import.meta.url),
);
const MODULE_ID = "virtual:palettes";
const RESOLVED_ID = `\0${MODULE_ID}`;
const UPPER = new Set(["oled", "ist", "ii"]);

function label(name) {
  return name
    .replace(/^_/, "")
    .split("_")
    .map((
      p,
    ) => (UPPER.has(p) ? p.toUpperCase() : p[0].toUpperCase() + p.slice(1)))
    .join(" ");
}

/** Compiles crates/palettes/palettes.toml into `virtual:palettes` at build time. */
function palettes() {
  return {
    name: "lutgen-palettes",

    resolveId: (id) => (id === MODULE_ID ? RESOLVED_ID : null),

    load(id) {
      if (id !== RESOLVED_ID) return null;
      // Watching the toml makes edits trigger a reload in dev.
      this.addWatchFile(PALETTES_TOML);

      const toml = readFileSync(PALETTES_TOML, "utf8");
      const entries = [];
      for (
        const [, name, body] of toml.matchAll(/^([a-z_0-9]+) = \[([^\]]*)\]/gm)
      ) {
        const colors = [...body.matchAll(/"(#[0-9a-fA-F]{6})"/g)].map((m) =>
          m[1]
        );
        if (colors.length) {
          entries.push({
            id: name.replace(/^_/, "").replace(/_/g, "-"),
            name: label(name),
            colors,
          });
        }
      }

      return [
        `export const PALETTES = ${JSON.stringify(entries)};`,
        "export const BY_ID = new Map(PALETTES.map((p) => [p.id, p]));",
      ].join("\n");
    },
  };
}

export default defineConfig({
  plugins: [svelte(), palettes()],
  base: "./",
});
