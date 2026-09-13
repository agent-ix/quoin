// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The config that runs the quoin#447 oracle capture, and the only one that
// does. The root `vite.config.ts` excludes `rust/**`, so the capture is
// unreachable from `pnpm test`; naming this file on the command line is the
// deliberate act that runs it. See `generate-oracle.mts` for the two commands.
//
// Paths are repository-relative and the command is run from the repository
// root: the capture imports `src/assurance/index.ts` directly, so it has to
// resolve modules the way the repository does.
import { defineConfig } from "vite";

export default defineConfig({
  // The same substitution `vite.config.ts` makes under vitest (serve): empty,
  // so `packageVersion()` exercises its package.json fallback. Without it
  // `src/version.ts` throws on an undefined global before any verdict is read.
  define: { __QUOIN_VERSION__: JSON.stringify("") },
  test: {
    include: ["rust/crates/quoin-assurance/tools/generate-oracle.mts"],
    environment: "node",
    // The same setup the rest of the suite uses, so the capture reads the
    // retained source and not whatever `dist/` happens to hold.
    setupFiles: ["tests/setup.ts"],
  },
});
