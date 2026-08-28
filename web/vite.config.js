import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { parse } from "smol-toml";
import { defineConfig } from "vite";

const PALETTES_TOML = fileURLToPath(
  new URL("../crates/palettes/palettes.toml", import.meta.url),
);
const MODULE_ID = "virtual:palettes";
const RESOLVED_ID = `\0${MODULE_ID}`;
const UPPER = new Set(["oled", "ist", "ii"]);
const HEX = /^#[0-9a-fA-F]{6}$/;

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

      // Parsed rather than pattern matched, so any entry the crate accepts is
      // carried over and anything unexpected fails the build instead of being
      // dropped. Mirrors the validation in crates/palettes/build.rs.
      const entries = Object.entries(parse(readFileSync(PALETTES_TOML, "utf8")))
        .map(([name, definition]) => {
          const sources = definition?.src;
          const colors = definition?.palette;
          if (
            !Array.isArray(sources) ||
            !sources.every((source) => typeof source === "string") ||
            !Array.isArray(colors) ||
            !colors.every((c) => typeof c === "string" && HEX.test(c))
          ) {
            throw new Error(
              `palettes.toml: "${name}" must contain src and palette arrays, with palette containing only "#rrggbb" strings`,
            );
          }
          return {
            id: name.replace(/^_/, "").replace(/_/g, "-"),
            name: label(name),
            src: sources,
            colors,
          };
        });

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
