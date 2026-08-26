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
        // oclif resolves `oclif.hooks` to a built module the same way it
        // resolves commands, so the hook needs its own entry too.
        "hooks/command-not-found": "src/hooks/command-not-found.ts",
        // oclif discovers commands as individual modules under dist/commands;
        // each command file is therefore its own build entry (mirrors the
        // canonical ix-cli build).
        "commands/update": "src/commands/update.ts",
        "commands/write": "src/commands/write.ts",
        "commands/review": "src/commands/review.ts",
        "commands/matrix": "src/commands/matrix.ts",
        "commands/to-plan": "src/commands/to-plan.ts",
        "commands/advise": "src/commands/advise.ts",
        "commands/completeness": "src/commands/completeness.ts",
        "commands/assurance": "src/commands/assurance.ts",
        "commands/report": "src/commands/report.ts",
        "commands/validate": "src/commands/validate.ts",
        "commands/measurement/index": "src/commands/measurement/index.ts",
        "commands/measurement/record": "src/commands/measurement/record.ts",
        "commands/config/index": "src/commands/config/index.ts",
        "commands/config/get": "src/commands/config/get.ts",
        "commands/config/set": "src/commands/config/set.ts",
        "commands/config/edit": "src/commands/config/edit.ts",
        "commands/config/doctor": "src/commands/config/doctor.ts",
        "commands/evidence/index": "src/commands/evidence/index.ts",
        "commands/evidence/record": "src/commands/evidence/record.ts",
        "commands/evidence/affirm": "src/commands/evidence/affirm.ts",
        "commands/evidence/gc": "src/commands/evidence/gc.ts",
        "commands/evidence/audit": "src/commands/evidence/audit.ts",
        "commands/evidence/baseline": "src/commands/evidence/baseline.ts",
        "commands/evidence/inspect-mocks":
          "src/commands/evidence/inspect-mocks.ts",
        "commands/catalog/index": "src/commands/catalog/index.ts",
        "commands/catalog/methods": "src/commands/catalog/methods.ts",
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
    // `corpus/` is the qa-corpus submodule: miniature repositories that are
    // DATA, run by `quire`, never by vitest. Its TypeScript fixtures are named
    // `*.test.ts` because that is what a TypeScript evidence file is called and
    // the fixture must look like the thing it stands for — so vitest collected
    // them, executed them, and failed on `trace is not defined`, which is the
    // undeclared marker one of them exists to seed. This is the same class as
    // the ecosystem manifest's `source_exclude`: a fixture tree that carries
    // real-looking evidence has to be excluded by the tools that walk it.
    exclude: ["node_modules/**", "dist/**", "corpus/**"],
    // The command-level tests shell out to `quire coverage`, which needs an
    // installed module declaring a `traceability:` model. Present on a
    // developer machine, absent in CI — see tests/global-setup.ts.
    globalSetup: ["tests/global-setup.ts"],
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
