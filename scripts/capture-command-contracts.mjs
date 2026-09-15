// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

/**
 * One-time Stage 8 capture of retained oclif shell contracts (quoin#520).
 *
 * Run after `pnpm build`, while `bin/quoin.js` is still the published shell:
 *
 *   node scripts/capture-command-contracts.mjs
 *
 * The resulting fixture is replayed by a Rust integration test. This script
 * is capture-time tooling only and must leave with the retained shell at the
 * Stage 9 deletion cutover; no Rust test invokes it or Node.
 */

import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(
  root,
  process.argv[2] ??
    "rust/crates/quoin-cli/tests/fixtures/retained-command-help.json",
);

// This is deliberately a frozen route census rather than discovery through
// oclif. Discovery could silently lose a command before capture; the Rust
// replay test separately rejects any capture below this population.
const ROUTES = [
  [],
  ["advise"],
  ["assurance"],
  ["catalog"],
  ["catalog", "list"],
  ["catalog", "methods"],
  ["catalog", "show"],
  ["catalog", "validate"],
  ["change-assurance"],
  ["change-assurance", "intake"],
  ["change-assurance", "receipt"],
  ["change-assurance", "recover"],
  ["change-assurance", "schema"],
  ["change-assurance", "seal-attestation"],
  ["change-assurance", "seal-record"],
  ["change-assurance", "verify-receipt"],
  ["completeness"],
  ["config"],
  ["config", "doctor"],
  ["config", "edit"],
  ["config", "get"],
  ["config", "set"],
  ["discharge"],
  ["evidence"],
  ["evidence", "affirm"],
  ["evidence", "audit"],
  ["evidence", "baseline"],
  ["evidence", "gc"],
  ["evidence", "inspect-mocks"],
  ["evidence", "record"],
  ["evidence", "record-experiment"],
  ["evidence", "record-operational"],
  ["evidence", "trust"],
  ["graph"],
  ["graph", "change-impact"],
  ["graph", "churn"],
  ["graph", "fan-out"],
  ["matrix"],
  ["measurement"],
  ["measurement", "intervention"],
  ["measurement", "operational-release"],
  ["measurement", "record"],
  ["module"],
  ["module", "ensure-defaults"],
  ["module", "install"],
  ["module", "list"],
  ["module", "remove"],
  ["plugin"],
  ["plugin", "ensure-defaults"],
  ["plugin", "install"],
  ["plugin", "list"],
  ["plugin", "remove"],
  ["report"],
  ["review"],
  ["semantic"],
  ["semantic", "sweep"],
  ["sync"],
  ["to-plan"],
  ["update"],
  ["validate"],
  ["write"],
];

// These are deterministic, offline shell boundaries that complement the help
// census. They deliberately avoid commands that read project configuration,
// run a core subprocess, or contact an update source.
const SHELL_CASES = [
  ["--version"],
  ["not-a-command"],
  ["catalog", "not-a-command", "--format", "json"],
  ["graph"],
  ["graph", "fan-out"],
  ["graph", "churn"],
  ["graph", "change-impact"],
  ["write"],
];

function gitRevision() {
  const result = spawnSync("git", ["rev-parse", "HEAD"], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.status !== 0) throw new Error(result.stderr || "git rev-parse failed");
  return result.stdout.trim();
}

function normalize(text) {
  return text
    .replace(/@agent-ix\/quoin\/[^\s]+ [^\n]+/g, "@agent-ix/quoin/<VERSION>")
    .replace(/@agent-ix\/quoin\/[^\s]+/g, "@agent-ix/quoin/<VERSION>")
    .replace(/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/gm, "<VERSION>");
}

function capture(argv, kind) {
  const result = spawnSync(process.execPath, ["bin/quoin.js", ...argv], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.error || result.status === null) {
    throw new Error(`${argv.join(" ") || "<root>"}: ${result.error?.message ?? "terminated"}`);
  }
  return {
    kind,
    argv,
    exit: result.status,
    stdout: normalize(result.stdout),
    stderr: normalize(result.stderr),
  };
}

const cases = [
  ...ROUTES.map((route) => capture([...route, "--help"], "help")),
  ...SHELL_CASES.map((argv) => capture(argv, "shell")),
];

const fixture = {
  schema_version: 1,
  captured_by: "scripts/capture-command-contracts.mjs",
  captured_revision: gitRevision(),
  normalization: "@agent-ix/quoin/<VERSION> replaces retained build/platform text; <VERSION> replaces bare SemVer output",
  cases,
};
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, `${JSON.stringify(fixture, null, 2)}\n`);
console.log(`captured ${cases.length} retained command help contracts to ${output}`);
