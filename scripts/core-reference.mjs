#!/usr/bin/env node
/**
 * Thin host-dispatch runner for the TypeScript side of `quoin-difftest`
 * (quoin#375).
 *
 * It does argv, stdin, two writes and an exit, and nothing else. Every
 * decision lives in `src/core/reference.ts`, where it is typed and unit
 * tested. Requires `make build`.
 */
import { readFileSync } from "node:fs";

import { canonicalJson, reference } from "../dist/core/reference.js";

let stdin = "";
try {
  stdin = readFileSync(0, "utf8");
} catch {
  stdin = "";
}

const outcome = reference(process.argv.slice(2), stdin);
if (outcome.payload !== null) process.stdout.write(`${outcome.payload}\n`);
if (outcome.diagnostics.length > 0) {
  process.stderr.write(`${canonicalJson(outcome.diagnostics)}\n`);
}
process.exit(outcome.exitCode);
