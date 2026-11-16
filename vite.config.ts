import { defineConfig } from "vite-plus";

const root = "crates/app/ui";

export default defineConfig({
  root,
  clearScreen: false,
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/crates/*/src/**", "**/target/**"] },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    // Keep `/*!` license notices (Lucide's, in `strip.ts`) in the minified JS.
    // Vite drops them when minifying, and an explicit `comments` object
    // replaces its defaults, so the other two are restated as Vite sets them.
    rolldownOptions: {
      output: {
        comments: {
          legal: true,
          annotation: !!process.env.TAURI_ENV_DEBUG,
          jsdoc: !!process.env.TAURI_ENV_DEBUG,
        },
      },
    },
  },
  fmt: {
    ignorePatterns: [
      "**/*.md",
      ".release-please-manifest.json",
      "release-please-config.json",
      "crates/app/capabilities/**",
      "crates/app/tauri.conf.json",
      // Written by the image-embeddings plan's genre_vectors.py.
      "crates/core/genres/**",
      // Written by the image-embeddings plan's rank_prior.py.
      "crates/core/ranking/**",
      // gpui-kit's theme files, vendored unchanged.
      "crates/native/themes/**",
      "target/**",
      "crates/app/ui/dist/**",
      ".claude/**",
    ],
  },
  lint: {
    ignorePatterns: ["crates/app/ui/dist/**", ".claude/**"],
    options: { typeAware: true, typeCheck: true },
  },
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
  },
});
