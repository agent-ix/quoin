// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// The TypeScript half of the W2 report gate (quoin#469, FR-100-AC-4).
//
//   node --loader ts-node/esm oracle/capture-report-oracle.mjs \
//     --out tests/fixtures/report-oracle.json
//
// Runs `buildInterventionReport`/`renderInterventionReport` and
// `buildOperationalReport`/`renderOperationalReport` over a fixed case list and
// writes what THEY produce. Nothing is asserted here; the Rust side compares.
//
// The output is committed and frozen. Per FR-101-AC-11 the capture records the
// producing implementation and revision, so the fixture is evidence rather than
// a file someone once generated. A live TypeScript runtime is never an oracle
// at test time — `tc_469_reports.rs` reads these bytes.
//
// `QUOIN_SRC_ROOT` overrides where the TypeScript is read from. It must be a
// checkout with `node_modules` installed.

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(
  process.env.QUOIN_SRC_ROOT ?? join(here, "..", "..", "..", ".."),
);
const load = (path) =>
  import(pathToFileURL(join(repoRoot, "src", path)).href);

const { buildInterventionReport, renderInterventionReport } = await load(
  "measurement/intervention-report.js",
);
const { buildOperationalReport, renderOperationalReport } = await load(
  "measurement/operational-report.js",
);

const argv = process.argv.slice(2);
let out = null;
for (let index = 0; index < argv.length; index += 1) {
  if (argv[index] === "--out") {
    out = argv[index + 1];
    index += 1;
  }
}
if (out === null) {
  throw new Error("usage: capture-report-oracle.mjs --out <file>");
}

const producer = {
  tool_identity: "quoin",
  tool_version: "0.9.0",
  configuration_digest: "sha256:aa",
  source_revision: "rev-1",
  environment: { ci: true, node: "22.11.0", workers: 4, seed: null },
  definition_version: "1",
};
const subject = { id: "quoin", revision: "rev-1" };
const design = {
  kind: "repeated",
  repetitions: 3,
  assignment: { method: "deterministic", seed: "7" },
  sampling_conditions: ["cold cache"],
};
const arm = (id) => ({
  id,
  population: "corpus",
  sample_size: 12,
  configuration: { flag: false, nested: { n: [1, 2] } },
});

/** An intervention record with every list populated. */
const interventionFull = {
  schema_version: 1,
  record_type: "intervention_experiment",
  record_id: "p-beta",
  observed_at: "2026-02-01T00:00:00.000Z",
  subject,
  producer,
  design,
  baseline: arm("baseline"),
  treatments: [arm("t2"), arm("t1")],
  changed_variables: [
    {
      name: "model",
      treatment_id: "t1",
      baseline_value: "small",
      treatment_value: "large",
    },
  ],
  held_constant: [{ name: "corpus", value: { revision: "rev-1" } }],
  // Deliberately out of sorted order: the builder sorts by
  // (treatment_id, metric) and the fixture must be able to see that.
  measured_effects: [
    {
      treatment_id: "t2",
      metric: "latency_ms",
      baseline_value: 120,
      treatment_value: 95.5,
      effect: -24.5,
      unit: "ms",
    },
    {
      treatment_id: "t1",
      metric: "pass_rate",
      baseline_value: 0.5,
      treatment_value: 1,
      effect: 0.5,
      unit: "ratio",
    },
    {
      treatment_id: "t1",
      metric: "flaky",
      baseline_value: true,
      treatment_value: null,
      effect: null,
      unit: "boolean",
    },
    {
      treatment_id: "t1",
      metric: "budget",
      baseline_value: 1e21,
      treatment_value: 1e-7,
      effect: "unquantified",
      unit: "usd",
    },
  ],
  interactions: [
    { description: "cache warms across arms", disposition: "uncontrolled" },
    { description: "declared controlled", disposition: "controlled" },
    { description: "unknown coupling", disposition: "unknown" },
  ],
  confounders: [
    { description: "machine load", disposition: "unknown" },
    { description: "pinned seed", disposition: "not_applicable" },
  ],
  status: "completed",
  conclusion: {
    kind: "causal_effect_established",
    statement: "the larger model lowers latency",
    attribution_confidence: "moderate",
  },
  gaps: ["no third arm"],
  owner: "qa",
  actions: ["repeat at scale"],
  raw_evidence: [
    {
      path: "b/second.json",
      media_type: "application/json",
      size_bytes: 2,
      digest: "sha256:bb",
    },
    {
      path: "a/first.json",
      media_type: "application/json",
      size_bytes: 1,
      digest: "sha256:aa",
    },
  ],
};

/** The same shape with every optional list empty — the placeholder paths. */
const interventionBare = {
  ...interventionFull,
  record_id: "p-alpha",
  observed_at: "2026-02-01T00:00:00.000Z",
  measured_effects: [],
  interactions: [],
  confounders: [],
  status: "inconclusive",
  conclusion: {
    kind: "cause_not_established",
    statement: "nothing was established",
    attribution_confidence: "none",
  },
  gaps: [],
  actions: [],
  raw_evidence: [],
};

/** Sorts ahead of both on `observed_at`, so ordering is observable. */
const interventionEarly = {
  ...interventionBare,
  record_id: "p-zulu",
  observed_at: "2026-01-01T00:00:00.000Z",
  status: "failed",
};

const operationalBase = {
  schema_version: 1,
  record_type: "operational_evidence",
  observed_at: "2026-03-01T00:00:00.000Z",
  control_kind: "release",
  subject,
  producer,
  scope: { service: "quoin", environment: "prod", population: "all" },
  configuration: {
    version_pins: [
      {
        kind: "model",
        identity: "opus",
        revision: "5",
        digest: "sha256:cc",
      },
    ],
  },
  owner: "release",
  gaps: ["one declared gap"],
  actions: ["review"],
  raw_evidence: [
    {
      path: "z/run.json",
      media_type: "application/json",
      size_bytes: 9,
      digest: "sha256:zz",
    },
    {
      path: "a/workflow.yaml",
      media_type: "application/yaml",
      size_bytes: 3,
      digest: "sha256:aa",
    },
  ],
};

const capability = (status, extra = {}) => ({
  ...operationalBase,
  record_shape: "standing_capability",
  capability: {
    control_id: "release-gate",
    status,
    surface: "gh workflow",
    authorized_roles: ["maintainer"],
    coverage: "all releases",
    limitations: ["no rollback"],
    supported_transitions: ["draft->published"],
    clock_support: {
      supported: true,
      start_event: "workflow_run",
      completion_event: "release",
      deadline_seconds: 600,
    },
  },
  ...extra,
});

const exercise = (outcome, clock, extra = {}) => ({
  ...operationalBase,
  record_shape: "exercise",
  exercise: {
    control_id: "release-gate",
    capability_record_id: "cap-1",
    mode: "actual",
    started_at: "2026-03-01T00:00:00.000Z",
    completed_at: "2026-03-01T00:05:00.000Z",
    actor: "ci",
    trigger: "push",
    outcome,
    state_before: { published: false },
    state_after: { published: true },
    observations: ["clean"],
    clock,
  },
  ...extra,
});

const clockMet = {
  applicability: "operational_with_clock",
  started_at: "2026-03-01T00:00:00.000Z",
  deadline_at: "2026-03-01T00:10:00.000Z",
  completed_at: "2026-03-01T00:05:00.000Z",
  status: "met",
};
const clockOpen = { ...clockMet, status: "open", completed_at: undefined };
const clockMissed = { ...clockMet, status: "missed" };
const clockNotApplicable = {
  applicability: "not_applicable",
  status: "not_applicable",
};

const interventionCases = [
  { name: "empty", records: [] },
  { name: "single_full", records: [interventionFull] },
  { name: "single_bare", records: [interventionBare] },
  {
    name: "ordering",
    records: [interventionFull, interventionBare, interventionEarly],
  },
];

const operationalCases = [
  { name: "empty", records: [] },
  {
    name: "capability_available",
    records: [capability("available", { record_id: "op-available" })],
  },
  {
    name: "capability_unknown",
    records: [capability("unknown", { record_id: "op-unknown" })],
  },
  {
    name: "capability_unavailable",
    records: [capability("unavailable", { record_id: "op-unavailable" })],
  },
  {
    name: "capability_not_applicable",
    records: [
      capability("not_applicable", { record_id: "op-not-applicable" }),
    ],
  },
  {
    name: "exercise_succeeded_clock_met",
    records: [exercise("succeeded", clockMet, { record_id: "op-met" })],
  },
  {
    name: "exercise_succeeded_clock_not_applicable",
    records: [
      exercise("succeeded", clockNotApplicable, { record_id: "op-na" }),
    ],
  },
  {
    name: "exercise_open",
    records: [exercise("partial", clockOpen, { record_id: "op-open" })],
  },
  {
    name: "exercise_missed",
    records: [exercise("failed", clockMissed, { record_id: "op-missed" })],
  },
  {
    name: "exercise_succeeded_but_clock_missed",
    records: [
      exercise("succeeded", clockMissed, { record_id: "op-succeeded-missed" }),
    ],
  },
  {
    name: "ordering",
    records: [
      exercise("succeeded", clockMet, {
        record_id: "op-b",
        observed_at: "2026-03-02T00:00:00.000Z",
      }),
      capability("available", {
        record_id: "op-a",
        observed_at: "2026-03-02T00:00:00.000Z",
      }),
      exercise("failed", clockMissed, {
        record_id: "op-c",
        observed_at: "2026-03-01T00:00:00.000Z",
      }),
    ],
  },
];

const revision = execFileSync("git", ["rev-parse", "HEAD"], {
  cwd: repoRoot,
  encoding: "utf8",
}).trim();

const capture = {
  produced_by: "src/measurement/intervention-report.ts, src/measurement/operational-report.ts",
  produced_from_revision: revision,
  capture_script: "rust/crates/quoin-measurement/oracle/capture-report-oracle.mjs",
  intervention: interventionCases.map(({ name, records }) => {
    const entries = buildInterventionReport(records);
    return { name, records, entries, rendered: renderInterventionReport(entries) };
  }),
  operational: operationalCases.map(({ name, records }) => {
    const entries = buildOperationalReport(records);
    return { name, records, entries, rendered: renderOperationalReport(entries) };
  }),
};

mkdirSync(dirname(resolve(out)), { recursive: true });
writeFileSync(resolve(out), `${JSON.stringify(capture, null, 2)}\n`);
