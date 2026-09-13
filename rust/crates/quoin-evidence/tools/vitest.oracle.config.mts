// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The config that runs the quoin#456 adapter-oracle capture, and the only one
// that does. The root `vite.config.ts` excludes `rust/**`, so the capture is
// unreachable from `pnpm test`; naming this file on the command line is the
// deliberate act that runs it. See `generate-adapter-oracle.mts`.
//
// Paths are repository-relative and the command is run from the repository
// root: the capture imports `src/evidence/adapters/`, so it has to resolve
// modules the way the repository does.
import { defineConfig } from "vite";

export default defineConfig({
  // The same substitution `vite.config.ts` makes under vitest (serve).
  define: { __QUOIN_VERSION__: JSON.stringify("") },
  test: {
    include: ["rust/crates/quoin-evidence/tools/generate-adapter-oracle.mts"],
    environment: "node",
  },
});
