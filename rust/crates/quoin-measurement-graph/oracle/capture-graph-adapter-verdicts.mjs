// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//
// One-time capture of `src/measurement/graph-adapters.ts`'s verdicts, for
// `tests/tc_475_parity.rs` (quoin#475).
//
//   node --experimental-strip-types \
//     rust/crates/quoin-measurement-graph/oracle/capture-graph-adapter-verdicts.mjs
//
// It imports the retained module rather than reimplementing it, writes
// `tests/goldens/graph-adapter-verdicts.json`, and is then finished: FR-101
// AC-5 forbids a Rust test spawning node, so the golden is committed and read.
// This script is deleted by the W12 cutover commit, along with the TypeScript
// it captures.

import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const crateRoot = join(here, "..");
const repoRoot = join(crateRoot, "..", "..", "..");
const modulePath = join(repoRoot, "src", "measurement", "graph-adapters.ts");

const {
  selectGraphAdapter,
  adaptQuireAssurance,
  adaptGraphQualityObservation,
  graphQualityObservationId,
} = await import(modulePath);
const { canonicalJson } = await import(
  join(repoRoot, "src", "store", "canonical.ts")
);

const clone = (value) => structuredClone(value);
const sha256 = (bytes) =>
  `sha256:${createHash("sha256").update(bytes).digest("hex")}`;

// ---------------------------------------------------------------- assurance

const bases = JSON.parse(
  readFileSync(
    join(
      crateRoot,
      "..",
      "quoin-quire",
      "tests",
      "goldens",
      "assurance-bases.json",
    ),
    "utf8",
  ),
).bases;
const assuranceBase = bases[0].document;
const acceptedPremises = {
  format: "quire-assurance",
  formatVersion: 1,
  source: clone(assuranceBase.source),
  modules: clone(assuranceBase.modules),
};

// ----------------------------------------------------------- graph quality

const scorerBytes = new Uint8Array(
  Buffer.from('{"scores":[1,2,3],"note":"complete scorer output"}', "utf8"),
);
const revision = "a".repeat(40);

const censusOf = (rows) => ({
  languages: rows,
  node_kinds: [],
  relation_kinds: [],
  resolver_tiers: [],
});

const measuredResults = {
  confusion_matrices: ["overall", "language", "node_kind", "relation_kind"].map(
    (dimension, index) => ({
      dimension,
      key: `k${index}`,
      true_positive: 10 + index,
      false_positive: index,
      false_negative: 1,
      true_negative: 100,
    }),
  ),
  unresolved: [{ dimension: "overall", key: "unresolved", count: 3 }],
  ambiguous: [{ dimension: "resolver_tier", key: "tier-2", count: 1 }],
  recall: [
    {
      dimension: "overall",
      key: "all",
      recovered: 9,
      expected: 10,
      ratio: 0.9,
    },
  ],
};

const producer = {
  extractor_revision: "v1.2.3",
  producer_contract_version: 1,
  parser_grammars: [
    { language: "rust", grammar: "tree-sitter-rust", revision: "v0.21.0" },
  ],
  configuration_digest: sha256("configuration"),
  source_revision: revision,
  corpus_revision: "b".repeat(40),
  scorer_version: "v0.4.0",
};

const withId = (record) => ({
  ...record,
  observation_id: graphQualityObservationId(record),
});

const measuredRecord = withId({
  schema_version: 1,
  record_type: "graph_quality_observation",
  observation_id: sha256("placeholder"),
  producer,
  measurement_plan: {
    ref: "ix://agent-ix/quire-code-rs/MP-001",
    definition_version: "quire-code.graph-quality-v1",
  },
  population: {
    state: "measured",
    files_seen: 12,
    supported_files: 10,
    unreadable_files: 0,
    unsupported_files: 2,
    census: censusOf([
      { key: "rust", count: 8 },
      { key: "typescript", count: 2 },
    ]),
  },
  results: measuredResults,
  raw_scorer_output: { path: "out/scores.json", digest: sha256(scorerBytes) },
});

const emptyRecord = withId({
  schema_version: 1,
  record_type: "graph_quality_observation",
  observation_id: sha256("placeholder"),
  producer,
  measurement_plan: {
    ref: "ix://agent-ix/quire-code-rs/MP-001",
    definition_version: "quire-code.graph-quality-v1",
  },
  population: {
    state: "empty",
    files_seen: 0,
    supported_files: 0,
    unreadable_files: 0,
    unsupported_files: 0,
    census: censusOf([]),
  },
  raw_scorer_output: { path: "out/scores.json", digest: sha256(scorerBytes) },
});

const unsupportedRecord = withId({
  schema_version: 1,
  record_type: "graph_quality_observation",
  observation_id: sha256("placeholder"),
  producer,
  measurement_plan: {
    ref: "ix://agent-ix/quire-code-rs/MP-001",
    definition_version: "quire-code.graph-quality-v1",
  },
  population: {
    state: "unsupported",
    files_seen: 4,
    supported_files: 0,
    unreadable_files: 0,
    unsupported_files: 4,
    census: censusOf([{ key: "cobol", count: 4 }]),
  },
  raw_scorer_output: { path: "out/scores.json", digest: sha256(scorerBytes) },
});

const attestation = {
  subject: "agent-ix/quire-code-rs",
  scope: { repositories: ["agent-ix/quoin"] },
  timestamp: "2026-01-02T03:04:05.000Z",
  environment: { CI: "true" },
  verificationStack: {
    schemaVersion: "verification-stack-attestation-v1",
    lockDigest: sha256("lock"),
    executableDigest: sha256("executable"),
    buildProfile: "release",
    toolchains: { node: "22.15.0", rust: "1.98.1", python: "3.13.1" },
    sources: {
      "agent-ix/quire-code-rs": {
        revision,
        sourceState: "clean",
        remote: "git@github.com:agent-ix/quire-code-rs.git",
      },
    },
    capabilities: ["graph_quality"],
    artifacts: { "scores.json": sha256(scorerBytes) },
  },
};

const plan = {
  id: "MP-001",
  title: "Graph quality",
  status: "active",
  stage: "observe",
  metric: "graph_quality",
  definitionVersion: "quire-code.graph-quality-v1",
  path: "spec/assurance/MP-001-graph-quality.md",
};
const otherPlan = {
  ...plan,
  id: "MP-900",
  metric: "coverage",
  stage: "observe",
};

const qualityInput = (overrides = {}) => ({
  record: clone(measuredRecord),
  scorerBytes,
  scorerMediaType: "application/json",
  attestation: clone(attestation),
  plans: [clone(plan), clone(otherPlan)],
  ...overrides,
});

// ---------------------------------------------------------------- the cases

const cases = [];
const add = (name, fn, input) => cases.push({ name, fn, input });

for (const name of ["quire-assurance-v1", "quire-code-graph-quality-v1"])
  add(`select/${name}`, "selectGraphAdapter", { name });
for (const name of ["", "quire-assurance-v2", "QUIRE-ASSURANCE-V1"])
  add(`select/refused/${name || "empty"}`, "selectGraphAdapter", { name });

add("identity/measured-record", "graphQualityObservationId", {
  value: clone(measuredRecord),
});
add("identity/empty-record", "graphQualityObservationId", {
  value: clone(emptyRecord),
});
add("identity/non-object", "graphQualityObservationId", { value: 7 });
add("identity/array", "graphQualityObservationId", { value: [1, "two"] });
add("identity/nested-sorted", "graphQualityObservationId", {
  value: {
    b: { d: 1, c: [{ z: 1, a: 2 }] },
    a: "x",
    observation_id: "dropped",
  },
});

const assuranceCase = (name, document, premises = acceptedPremises) =>
  add(`assurance/${name}`, "adaptQuireAssurance", {
    document,
    accepted: clone(premises),
  });

assuranceCase("base", clone(assuranceBase));
for (const key of Object.keys(assuranceBase)) {
  const mutated = clone(assuranceBase);
  delete mutated[key];
  assuranceCase(`missing/${key}`, mutated);
}
{
  const mutated = clone(assuranceBase);
  mutated.unexpected = 1;
  assuranceCase("unknown-key", mutated);
}
{
  const mutated = clone(assuranceBase);
  mutated.source = { ...mutated.source, revision: "c".repeat(40) };
  assuranceCase("source-revision-disagrees", mutated);
}
{
  const mutated = clone(assuranceBase);
  mutated.modules = [];
  assuranceCase("modules-disagree", mutated);
}
{
  const mutated = clone(assuranceBase);
  mutated.source = { ...mutated.source, revision: "not-a-revision" };
  assuranceCase("source-revision-malformed", mutated);
}
if (assuranceBase.artifacts.length > 0) {
  for (const [name, path] of [
    ["traversal", "../escape.md"],
    ["rooted", "/etc/passwd"],
    ["inner-dots", "a..b.md"],
  ]) {
    const mutated = clone(assuranceBase);
    mutated.artifacts[0].locator.path = path;
    assuranceCase(`locator-path/${name}`, mutated);
  }
  const uuidCases = [
    ["nil", "00000000-0000-0000-0000-000000000000"],
    ["max", "ffffffff-ffff-ffff-ffff-ffffffffffff"],
    ["v4", "3f2504e0-4f89-41d3-9a0c-0305e82c3301"],
    ["v8", "3f2504e0-4f89-81d3-9a0c-0305e82c3301"],
    ["v9", "3f2504e0-4f89-91d3-9a0c-0305e82c3301"],
    ["variant-c", "3f2504e0-4f89-41d3-ca0c-0305e82c3301"],
    ["uppercase", "3F2504E0-4F89-41D3-9A0C-0305E82C3301"],
    ["truncated", "3f2504e0-4f89-41d3-9a0c-0305e82c330"],
  ];
  for (const [name, uuid] of uuidCases) {
    const mutated = clone(assuranceBase);
    mutated.artifacts[0].uuid = uuid;
    assuranceCase(`uuid/${name}`, mutated);
  }
  const mutated = clone(assuranceBase);
  mutated.artifacts[0].uuid = null;
  assuranceCase("uuid/null", mutated);
}

add("quality/measured", "adaptGraphQualityObservation", qualityInput());
add("quality/empty-population", "adaptGraphQualityObservation", {
  ...qualityInput(),
  record: clone(emptyRecord),
});
add("quality/unsupported-population", "adaptGraphQualityObservation", {
  ...qualityInput(),
  record: clone(unsupportedRecord),
});
add("quality/no-scope", "adaptGraphQualityObservation", {
  ...qualityInput(),
  attestation: (() => {
    const without = clone(attestation);
    delete without.scope;
    return without;
  })(),
});
{
  const broken = clone(measuredRecord);
  broken.observation_id = sha256("wrong");
  add("quality/observation-id-disagrees", "adaptGraphQualityObservation", {
    ...qualityInput(),
    record: broken,
  });
}
{
  const broken = clone(measuredRecord);
  broken.population.supported_files = 0;
  add(
    "quality/measured-without-supported-files",
    "adaptGraphQualityObservation",
    {
      ...qualityInput(),
      record: broken,
    },
  );
}
{
  const broken = clone(emptyRecord);
  broken.results = clone(measuredResults);
  add("quality/empty-with-results", "adaptGraphQualityObservation", {
    ...qualityInput(),
    record: broken,
  });
}
{
  const broken = clone(measuredRecord);
  delete broken.results;
  add("quality/measured-without-results", "adaptGraphQualityObservation", {
    ...qualityInput(),
    record: broken,
  });
}
{
  const broken = clone(measuredRecord);
  broken.unexpected = true;
  add("quality/unknown-member", "adaptGraphQualityObservation", {
    ...qualityInput(),
    record: broken,
  });
}
{
  const broken = clone(measuredRecord);
  broken.results.confusion_matrices = broken.results.confusion_matrices.slice(
    0,
    3,
  );
  add("quality/too-few-matrices", "adaptGraphQualityObservation", {
    ...qualityInput(),
    record: broken,
  });
}
{
  const broken = clone(measuredRecord);
  broken.results.recall[0].ratio = 1.5;
  add("quality/ratio-out-of-range", "adaptGraphQualityObservation", {
    ...qualityInput(),
    record: broken,
  });
}
{
  const broken = clone(measuredRecord);
  broken.population.census.languages = [
    { key: "rust", count: 1 },
    { key: "rust", count: 2 },
  ];
  add("quality/duplicate-partition", "adaptGraphQualityObservation", {
    ...qualityInput(),
    record: withId(broken),
  });
}
add("quality/no-media-type", "adaptGraphQualityObservation", {
  ...qualityInput(),
  scorerMediaType: "",
});
add("quality/digest-mismatch", "adaptGraphQualityObservation", {
  ...qualityInput(),
  scorerBytes: new Uint8Array(Buffer.from("different bytes", "utf8")),
});
for (const [name, plans] of [
  ["none", []],
  ["only-other-metric", [clone(otherPlan)]],
  ["proposed", [{ ...clone(plan), status: "proposed" }]],
  ["two-active", [clone(plan), { ...clone(plan), id: "MP-002" }]],
  [
    "wrong-definition",
    [{ ...clone(plan), definitionVersion: "quire-code.graph-quality-v2" }],
  ],
])
  add(`quality/plans/${name}`, "adaptGraphQualityObservation", {
    ...qualityInput(),
    plans,
  });
for (const [name, timestamp] of [
  ["rfc3339", "2026-01-02T03:04:05.000Z"],
  ["rfc3339-offset", "2026-01-02T03:04:05+01:00"],
  ["date-only", "2026-01-02"],
  ["v8-heuristic-slashes", "2026/01/02 10:00"],
  ["v8-heuristic-words", "Jan 1 2026"],
  ["not-an-instant", "whenever"],
  ["empty", ""],
]) {
  const mutated = clone(attestation);
  mutated.timestamp = timestamp;
  add(`quality/timestamp/${name}`, "adaptGraphQualityObservation", {
    ...qualityInput(),
    attestation: mutated,
  });
}
{
  const mutated = clone(attestation);
  delete mutated.verificationStack.toolchains;
  add("quality/attestation/no-toolchains", "adaptGraphQualityObservation", {
    ...qualityInput(),
    attestation: mutated,
  });
}
{
  const mutated = clone(attestation);
  mutated.verificationStack.buildProfile = "debug";
  add("quality/attestation/debug-build", "adaptGraphQualityObservation", {
    ...qualityInput(),
    attestation: mutated,
  });
}
{
  const mutated = clone(attestation);
  mutated.subject = "";
  add("quality/attestation/empty-subject", "adaptGraphQualityObservation", {
    ...qualityInput(),
    attestation: mutated,
  });
}

for (const [name, text] of [
  ["empty", ""],
  ["ascii", "hello"],
  ["one-pad", "hell"],
  ["two-pad", "hel"],
  ["all-bytes", null],
  ["utf8", "héllo wörld ☃"],
])
  add(`base64/encode/${name}`, "base64Encode", {
    bytes:
      text === null
        ? Array.from({ length: 256 }, (_, index) => index)
        : Array.from(Buffer.from(text, "utf8")),
  });
for (const text of [
  "aGVsbG8=",
  "aGVsbG8",
  "aGV sbG8=",
  "aGVsbG8=extra",
  "a GVsbG8=",
  "aGVsbG8==",
  "!!!!",
  "aGVs*bG8=",
  "=aGVsbG8",
  "aGVsbG9=",
  "AA==",
  "AA=",
  "A",
  "",
  "aGVsbG8\n",
  "aGVs\nbG8=",
  "-_8=",
])
  add(`base64/decode/${JSON.stringify(text)}`, "base64Decode", { text });

// --------------------------------------------------------------- the capture

const run = (entry) => {
  const { fn, input } = entry;
  switch (fn) {
    case "selectGraphAdapter":
      return { output: selectGraphAdapter(input.name) };
    case "graphQualityObservationId":
      return { output: graphQualityObservationId(clone(input.value)) };
    case "adaptQuireAssurance":
      return {
        output: canonicalJson(
          adaptQuireAssurance(input.document, input.accepted),
        ),
      };
    case "adaptGraphQualityObservation":
      return {
        output: canonicalJson(adaptGraphQualityObservation(input)),
      };
    case "base64Encode":
      return {
        output: Buffer.from(new Uint8Array(input.bytes)).toString("base64"),
      };
    case "base64Decode":
      return {
        output: Array.from(Buffer.from(input.text, "base64")),
      };
    default:
      throw new Error(`unhandled function ${fn}`);
  }
};

const captured = cases.map((entry) => {
  const serializable = {
    ...entry,
    input: JSON.parse(
      JSON.stringify(entry.input, (_key, value) =>
        value instanceof Uint8Array ? Array.from(value) : value,
      ),
    ),
  };
  try {
    const { output } = run(entry);
    return { ...serializable, verdict: "accepted", output };
  } catch (error) {
    return {
      ...serializable,
      verdict: "refused",
      code: error.code ?? null,
      message: error.message,
    };
  }
});

const golden = {
  provenance: {
    captured_by:
      "rust/crates/quoin-measurement-graph/oracle/capture-graph-adapter-verdicts.mjs",
    module: "src/measurement/graph-adapters.ts",
    module_digest: sha256(readFileSync(modulePath)),
    node: process.version,
    zod: JSON.parse(
      readFileSync(
        createRequire(import.meta.url).resolve("zod/package.json"),
        "utf8",
      ),
    ).version,
    quoin_revision: process.env.QUOIN_REVISION ?? "unrecorded",
    note: "Captured once. FR-101 AC-5: no Rust test runs node. Deleted by the W12 cutover.",
  },
  cases: captured,
};
const out = join(crateRoot, "tests", "goldens", "graph-adapter-verdicts.json");
writeFileSync(out, `${JSON.stringify(golden, null, 2)}\n`);
const accepted = captured.filter(
  (entry) => entry.verdict === "accepted",
).length;
console.log(
  `${captured.length} cases -> ${out} (${accepted} accepted, ${captured.length - accepted} refused)`,
);
