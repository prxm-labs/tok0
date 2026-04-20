import { defineConfig } from "astro/config";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import react from "@astrojs/react";
import mdx from "@astrojs/mdx";
import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";

const here = dirname(fileURLToPath(import.meta.url));
// `apps/cli/Cargo.toml` is the canonical version source for the homepage
// hero + status panel (see src/lib/version.ts). It lives outside this
// package's root, so Vite's strict fs guard needs an explicit allow-entry.
const repoRoot = resolve(here, "../..");

// https://astro.build/config
export default defineConfig({
  site: "https://tok0.dev",
  integrations: [react(), mdx(), sitemap()],
  markdown: {
    shikiConfig: {
      theme: {
        name: "tok0-spec",
        type: "dark",
        bg: "#0a0a0a",
        fg: "#f4f1ea",
        colors: {
          "editor.background": "#0a0a0a",
          "editor.foreground": "#f4f1ea",
        },
        settings: [
          { settings: { background: "#0a0a0a", foreground: "#f4f1ea" } },
          // muted neutrals for structural punctuation
          {
            scope: ["punctuation", "meta.brace", "meta.delimiter"],
            settings: { foreground: "#8b8a83" },
          },
          // comments — quiet
          {
            scope: [
              "comment",
              "punctuation.definition.comment",
              "string.comment",
            ],
            settings: { foreground: "#6a6a66", fontStyle: "italic" },
          },
          // strings — accent orange (the project signature)
          {
            scope: [
              "string",
              "string.quoted",
              "string.template",
              "punctuation.definition.string",
            ],
            settings: { foreground: "#ff4a1c" },
          },
          // numbers / booleans — cyan accent
          {
            scope: [
              "constant.numeric",
              "constant.language",
              "constant.language.boolean",
            ],
            settings: { foreground: "#5bb8e5" },
          },
          // keywords — paper, bold
          {
            scope: [
              "keyword",
              "keyword.control",
              "keyword.operator",
              "storage.modifier",
            ],
            settings: { foreground: "#f4f1ea", fontStyle: "bold" },
          },
          // storage / types — italic paper
          {
            scope: ["storage.type", "support.type", "entity.name.type"],
            settings: { foreground: "#ddd8cb", fontStyle: "italic" },
          },
          // functions — bright paper
          {
            scope: [
              "entity.name.function",
              "support.function",
              "meta.function-call",
            ],
            settings: { foreground: "#f4f1ea" },
          },
          // variables / parameters
          {
            scope: ["variable", "support.variable", "variable.parameter"],
            settings: { foreground: "#e9e5db" },
          },
          // tags (HTML/XML/JSX)
          {
            scope: ["entity.name.tag"],
            settings: { foreground: "#f4f1ea", fontStyle: "bold" },
          },
          // attribute names
          {
            scope: ["entity.other.attribute-name"],
            settings: { foreground: "#ff4a1c" },
          },
          // TOML / JSON / YAML keys
          {
            scope: [
              "support.type.property-name",
              "entity.name.tag.toml",
              "entity.name.tag.yaml",
              "meta.structure.dictionary.key",
            ],
            settings: { foreground: "#f4f1ea", fontStyle: "bold" },
          },
          // section headers in TOML, e.g. [filter]
          {
            scope: [
              "entity.other.attribute-name.table.toml",
              "punctuation.definition.table.toml",
            ],
            settings: { foreground: "#ff4a1c", fontStyle: "bold" },
          },
          // shell builtins / commands
          {
            scope: ["support.function.builtin.shell", "meta.function-call.shell"],
            settings: { foreground: "#f4f1ea", fontStyle: "bold" },
          },
          // shell options / flags
          {
            scope: [
              "constant.other.option.shell",
              "variable.parameter.option.shell",
            ],
            settings: { foreground: "#5bb8e5" },
          },
          // diff
          {
            scope: ["markup.inserted"],
            settings: { foreground: "#5bb8e5" },
          },
          {
            scope: ["markup.deleted"],
            settings: { foreground: "#ff4a1c" },
          },
          // headings (markdown)
          {
            scope: ["markup.heading"],
            settings: { foreground: "#f4f1ea", fontStyle: "bold" },
          },
        ],
      },
      wrap: true,
    },
  },
  vite: {
    plugins: [tailwindcss()],
    server: {
      fs: {
        allow: [here, repoRoot],
      },
    },
  },
});

