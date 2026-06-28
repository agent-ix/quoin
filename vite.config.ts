/// <reference types="vitest" />
import { execSync } from "node:child_process";
import { builtinModules } from "node:module";

import { defineConfig } from "vite";
import dts from "vite-plugin-dts";

// Truthful version baked into the bundle at build time. `git describe` yields the
// bare tag when HEAD is exactly a release tag (e.g. v0.5.4) and a commits-ahead
// string otherwise (e.g. v0.5.3-2-gabc123); we strip the leading `v`. We do NOT
// pass `--dirty`: the release pipeline stamps package.json before building, which
// would dirty the tree and mislabel a clean release as `-dirty`. Only computed for
// `vite build` — under vitest (serve) it stays "" so packageVersion() exercises its
// package.json fallback. Empty on a no-git build (e.g. tarball) too.
function gitVersion(): string {
  try {
    return execSync("git describe --tags --always", {
      encoding: "utf8",
    })
      .trim()
      .replace(/^v/, "");
  } catch {
    return "";
  }
}

const external = [
  ...builtinModules,
  ...builtinModules.map((name) => `node:${name}`),
  /^@agent-ix\//,
  /^@napi-rs\/keyring/,
  "@clack/prompts",
  "@oclif/core",
  "age-encryption",
  "react",
  "yaml",
  "zod",
];

export default defineConfig(({ command }) => ({
  define: {
    __QUOIN_VERSION__: JSON.stringify(command === "build" ? gitVersion() : ""),
  },
  plugins: [dts({ rollupTypes: true, include: ["src"] })],
  build: {
    lib: {
      entry: {
        index: "src/index.ts",
        cli: "src/cli.ts",
        // oclif discovers commands as individual modules under dist/commands;
        // each command file is therefore its own build entry (mirrors the
        // canonical ix-cli build).
        "commands/update": "src/commands/update.ts",
        "commands/write": "src/commands/write.ts",
        "commands/review": "src/commands/review.ts",
        "commands/matrix": "src/commands/matrix.ts",
        "commands/to-plan": "src/commands/to-plan.ts",
        "commands/catalog/index": "src/commands/catalog/index.ts",
        "commands/catalog/list": "src/commands/catalog/list.ts",
        "commands/catalog/show": "src/commands/catalog/show.ts",
        "commands/catalog/validate": "src/commands/catalog/validate.ts",
        "commands/module/index": "src/commands/module/index.ts",
        "commands/module/list": "src/commands/module/list.ts",
        "commands/module/install": "src/commands/module/install.ts",
        "commands/module/remove": "src/commands/module/remove.ts",
        "commands/module/ensure-defaults":
          "src/commands/module/ensure-defaults.ts",
        "commands/plugin/index": "src/commands/plugin/index.ts",
        "commands/plugin/list": "src/commands/plugin/list.ts",
        "commands/plugin/install": "src/commands/plugin/install.ts",
        "commands/plugin/remove": "src/commands/plugin/remove.ts",
        "commands/plugin/ensure-defaults":
          "src/commands/plugin/ensure-defaults.ts",
      },
      name: "Ixspec",
      fileName: (format, entryName) => `${entryName}.js`,
      formats: ["es"],
    },
    target: "node18",
    rollupOptions: {
      external,
    },
  },
  test: {
    globals: true,
    environment: "node",
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,js}"],
      thresholds: {
        branches: 100,
        functions: 100,
        lines: 100,
        statements: 100,
      },
    },
  },
}));
