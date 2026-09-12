// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// Build a change-assurance store with the *oracle's own writers*.
//
//   node --loader ts-node/esm oracle/build-change-assurance-fixture.mjs <dir>
//
// Why this exists: the replay found that **no reachable repository in the
// ecosystem contains a change-assurance record or a proof attestation**. The
// evidence stores that exist hold runs, scans, bindings, baselines,
// measurements and interventions — all of them written in the pretty form.
// So the highest-risk path in this crate, the RFC 8785 on-disk form and the
// digest that *names* a record, had nothing real to replay against.
//
// This writes one through `sealChangeRecord` / `writeChangeRecord` /
// `sealAttestation` / `intakeAttestation` — the shipped TypeScript, not a
// re-implementation — so the replay has a store whose bytes, digests, filenames
// and attestation pairing were all produced by the oracle. It is a fixture, and
// the report must say so; it is not a substitute for a real store, it is the
// only instance of that shape that exists.
//
// The record shape is the one `tests/change-assurance.test.ts` uses. The store
// it writes is committed at `tests/fixtures/change-assurance-store/` so
// `tc_change_assurance_store.rs` can replay it with no TypeScript in the loop.
//
// `QUOIN_SRC_ROOT` overrides where the TypeScript is read from.

import { mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(process.env.QUOIN_SRC_ROOT ?? join(here, "..", "..", ".."));
const load = (path) => import(pathToFileURL(join(repoRoot, "src", path)).href);

const {
  blake3Hex,
  intakeAttestation,
  sealAttestation,
  sealChangeRecord,
  writeChangeRecord,
} = await load("change-assurance/index.js");

const target = resolve(process.argv[2] ?? "");
if (!process.argv[2]) {
  process.stderr.write("usage: build-change-assurance-fixture.mjs <dir>\n");
  process.exit(2);
}
mkdirSync(join(target, "spec", "evidence"), { recursive: true });

const HEX_A = "a".repeat(64);
const HEX_B = "b".repeat(64);

const recordInput = (revision) => ({
  schema_version: 1,
  record_type: "change_assurance",
  record_id: "change-1",
  revision,
  parent_digest: revision === 1 ? null : HEX_A,
  subject: {
    repository: "agent-ix/quoin",
    base_revision: "abc123",
    // Non-ASCII and an astral character, so the fixture exercises the UTF-16
    // ordering and the literal-UTF-8 escaping rules, not just ASCII.
    scope: ["src/evidence", "docs/é", "docs/\u{10000}"],
  },
  source_connections: [
    { source_id: "FR-063", kind: "requirement", revision: "1", digest: HEX_A },
  ],
  impact_snapshot: {
    identity: "impact-1",
    revision: "1",
    digest: HEX_B,
    completeness: "complete",
    truncated: false,
    gaps: [],
  },
  definition: {
    requirements: [
      { id: "FR-063", statement: "seal it", source_ids: ["FR-063"] },
    ],
    preservation_constraints: [],
    proof_obligations: [
      {
        proof_id: "proof-1",
        statement: "tests pass",
        obligation_ids: ["FR-063-AC-1"],
        evidence_kind: "Unit",
        command: { argv: ["pnpm", "test"], working_directory: "." },
        tool_identity: "vitest",
        configuration_digest: HEX_B,
      },
    ],
    unknowns: [],
  },
  review_workflow: {
    run_id: "run-1",
    decision_event_kind: "change_assurance.review_decided",
  },
});

const records = [];
for (const revision of [1, 2, 3]) {
  const sealed = sealChangeRecord(recordInput(revision));
  writeChangeRecord(target, sealed);
  records.push(sealed);
}

// Four outputs whose bytes are deliberately awkward: empty, a lone NUL, a large
// binary run, and UTF-8 that is not JSON. A retained output is opaque bytes; the
// raw-bytes digest domain must not care what is in it.
const outputs = [
  new Uint8Array(0),
  new Uint8Array([0]),
  new Uint8Array(Array.from({ length: 4096 }, (_, index) => index % 251)),
  new TextEncoder().encode("not json: \u{1F600}\0\u{FFFD}"),
];

let attestations = 0;
for (const [index, output] of outputs.entries()) {
  const sealed = sealAttestation({
    schema_version: 1,
    record_type: "proof_attestation",
    attestation_id: `att-${index + 1}`,
    record_digest: records[0].digest,
    candidate_revision: `candidate-${index + 1}`,
    proof_id: "proof-1",
    command: { argv: ["pnpm", "test"], working_directory: "." },
    tool: { identity: "vitest", version: "4.1.10", configuration_digest: HEX_B },
    environment: { os: "test" },
    observed_at: "2026-08-31T00:00:00Z",
    result: "passed",
    retained_output: {
      media_type: "application/octet-stream",
      digest: blake3Hex(output),
      size_bytes: output.byteLength,
    },
  });
  const encoder = new TextEncoder();
  intakeAttestation(target, encoder.encode(JSON.stringify(sealed)), output);
  attestations += 1;
}

process.stderr.write(
  `change-assurance fixture: ${records.length} records, ${attestations} attestations -> ${target}\n`,
);
