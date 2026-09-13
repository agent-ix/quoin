// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The config that runs the quoin#456 store-oracle capture, and the only one
// that does. The root `vite.config.ts` excludes `rust/**`, so the capture is
// unreachable from `pnpm test`; naming this file on the command line is the
// deliberate act that runs it. See `generate-store-oracle.mts`.
import { defineConfig } from "vite";

export default defineConfig({
  define: { __QUOIN_VERSION__: JSON.stringify("") },
  test: {
    include: ["rust/crates/quoin-evidence/tools/generate-store-oracle.mts"],
    environment: "node",
  },
});
