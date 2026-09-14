// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

/**
 * Capture the retained TypeScript's answers for quoin#383, ONCE.
 *
 * Run from the repository root, against a plain `tsc` emit of the retained
 * source. The shipped build is a rolldown bundle whose chunk exports are
 * minified, so it cannot answer by name:
 *
 *   pnpm install
 *   pnpm exec tsc -p tsconfig.json --outDir dist-oracle \
 *     --declaration false --declarationMap false --sourceMap false
 *   node rust/crates/quoin-auditor/oracle/capture-auditor.mjs \
 *     > rust/crates/quoin-auditor/tests/goldens/auditor.json
 *   rm -rf dist-oracle
 *
 * The output is committed and is the oracle from then on. **No test spawns
 * node** (FR-101-AC-5): the retained implementation is not a runtime oracle,
 * and re-running this is a deliberate act with a diff to review rather than
 * something a gate does behind your back.
 *
 * Every case is a question about behaviour someone could get wrong, not a
 * sample of production data. The case names say which question. A case the
 * retained code THROWS on is captured as `{ threw: <message> }` rather than
 * omitted — see `DIVERGENCE.md` §1, where that is the whole point.
 */

import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const DIST =
  process.env.QUOIN_TS_DIST ??
  new URL("../../../../dist-oracle/", import.meta.url).pathname;

const { audit, ratchet, delta, scoresFor, MOCK_SUBJECT_FLOOR } = await import(
  join(DIST, "auditor/audit.js")
);
const {
  advise,
  characteristicsOf,
  mintableCharacteristics,
  uncataloguedAuthoredMethods,
} = await import(join(DIST, "advisor/advise.js"));
const { loadMethodCatalog, methodClasses } = await import(
  join(DIST, "method-catalog.js")
);

// ── helpers ────────────────────────────────────────────────────────────────

const HASH = "a".repeat(64);
const OTHER_HASH = "b".repeat(64);
const HEAD = "c".repeat(40);
const OLD = "d".repeat(40);

const obligation = (over = {}) => ({
  source: "spec/fr.md",
  document: "FR-001",
  id: "FR-001-AC-1",
  statement: "The system shall record the result.",
  statement_hash: HASH,
  ...over,
});

const binding = (over = {}) => ({
  obligation: "FR-001-AC-1",
  suite: "unit",
  symbols: ["test_records"],
  commit: HEAD,
  statementHashAtBinding: HASH,
  ...over,
});

const run = (over = {}) => ({
  schemaVersion: 1,
  suite: "unit",
  commit: HEAD,
  timestamp: "2026-01-01T00:00:00Z",
  tool: "jest@30",
  entries: [{ symbol: "test_records", outcome: "pass" }],
  ...over,
});

const scan = (over = {}) => ({
  schemaVersion: 1,
  suite: "scan",
  commit: HEAD,
  timestamp: "2026-01-01T00:00:00Z",
  tool: "semgrep@1",
  findings: [],
  ...over,
});

const CATALOG = {
  methods: [
    {
      id: "unit-testing",
      name: "Unit testing",
      class: "Test",
      definition: "Exercise the unit.",
      evidenceKind: "unit",
      applicability: { characteristics: ["state-machine"] },
      tooling: ["jest"],
      moduleName: "core",
    },
    {
      id: "inspection",
      name: "Inspection",
      class: "Inspection",
      definition: "Read it.",
      applicability: { archetypes: ["StR"] },
      tooling: [],
      moduleName: "core",
    },
    {
      id: "mutation-testing",
      name: "Mutation testing",
      class: "Test",
      definition: "Kill the mutants.",
      evidenceKind: "mutation",
      applicability: {
        characteristics: ["high-criticality", "fault-detection-failed"],
      },
      tooling: ["cargo-mutants"],
      moduleName: "extra",
    },
  ],
  duplicates: [],
  unreadable: [],
};

// ── audit cases ────────────────────────────────────────────────────────────

/** @type {Array<{name: string, input: unknown}>} */
const AUDIT_CASES = [
  {
    name: "an-obligation-with-no-binding-is-undischarged",
    input: { obligations: [obligation()], bindings: [], runs: [] },
  },
  {
    name: "a-clean-obligation-is-healthy-and-silent",
    input: {
      obligations: [obligation()],
      bindings: [binding()],
      runs: [run()],
    },
  },
  {
    name: "unknown-method-fires-before-the-binding-guard",
    // Both findings must appear: an unbound obligation with an uncatalogued
    // method is BOTH undischarged AND unknown-method, and hiding either behind
    // the other costs a reader a fact (agent-ix/quoin#165).
    input: {
      obligations: [obligation({ method: "Vibes" })],
      bindings: [],
      runs: [],
      catalog: CATALOG,
    },
  },
  {
    name: "a-catalogued-class-is-not-an-unknown-method",
    input: {
      obligations: [obligation({ method: "Inspection" })],
      bindings: [binding()],
      runs: [run()],
      catalog: CATALOG,
    },
  },
  {
    name: "a-reworded-statement-makes-one-binding-suspect",
    input: {
      obligations: [obligation()],
      bindings: [
        binding({ suite: "unit", statementHashAtBinding: OTHER_HASH }),
        binding({ suite: "zebra" }),
      ],
      runs: [run(), run({ suite: "zebra" })],
    },
  },
  {
    name: "a-binding-with-no-record-at-all-is-stale",
    input: { obligations: [obligation()], bindings: [binding()], runs: [] },
  },
  {
    name: "a-scan-backed-binding-is-not-stale",
    input: {
      obligations: [obligation()],
      bindings: [binding({ suite: "scan", symbols: [] })],
      runs: [],
      scans: [scan()],
      vacuousScanSuites: [],
    },
  },
  {
    name: "a-scan-that-evaluated-no-rules-is-vacuous",
    input: {
      obligations: [obligation()],
      bindings: [binding({ suite: "scan", symbols: [] })],
      runs: [],
      scans: [scan({ rulesEvaluated: 0 })],
      vacuousScanSuites: ["scan"],
    },
  },
  {
    name: "an-uninspected-run-suite-is-unevaluated-not-healthy",
    input: {
      obligations: [obligation()],
      bindings: [binding()],
      runs: [run()],
      injections: [],
      mockInspectionSuites: [],
    },
  },
  {
    name: "every-binding-mocked-is-a-finding",
    input: {
      obligations: [obligation()],
      bindings: [binding()],
      runs: [run()],
      injections: [
        {
          suite: "unit",
          symbol: "test_records",
          injects: ["record", "result", "system"],
        },
      ],
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "one-real-suite-beside-a-mocked-one-is-ordinary-test-design",
    input: {
      obligations: [obligation()],
      bindings: [
        binding(),
        binding({ suite: "integration", symbols: ["it_records"] }),
      ],
      runs: [
        run(),
        run({
          suite: "integration",
          entries: [{ symbol: "it_records", outcome: "pass" }],
        }),
      ],
      injections: [
        {
          suite: "unit",
          symbol: "test_records",
          injects: ["record", "result", "system"],
        },
      ],
      mockInspectionSuites: ["unit", "integration"],
    },
  },
  {
    name: "every-bound-symbol-skipped-is-vacuous",
    input: {
      obligations: [obligation()],
      bindings: [binding()],
      runs: [run({ entries: [{ symbol: "test_records", outcome: "skip" }] })],
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "one-skipped-symbol-among-two-is-not-vacuous",
    input: {
      obligations: [obligation()],
      bindings: [binding({ symbols: ["test_records", "test_other"] })],
      runs: [
        run({
          entries: [
            { symbol: "test_records", outcome: "skip" },
            { symbol: "test_other", outcome: "pass" },
          ],
        }),
      ],
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "a-run-behind-head-is-reported-and-does-not-stop-the-ladder",
    input: {
      obligations: [obligation()],
      bindings: [binding()],
      runs: [run({ commit: OLD })],
      headCommit: HEAD,
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "a-scan-only-binding-beside-a-run-throws-in-the-retained-code",
    // DIVERGENCE §1. `runs` is built from `runBindings` and then indexed with
    // an index into `bindings`; with one scan-backed binding sorting first the
    // last index walks past the end and `runs[i].commit` throws. Captured, not
    // avoided: the divergence has to be provable from the committed corpus.
    input: {
      obligations: [obligation()],
      bindings: [
        binding({ suite: "aaa-scan", symbols: [] }),
        binding({ suite: "unit" }),
      ],
      runs: [run({ commit: OLD })],
      scans: [scan({ suite: "aaa-scan" })],
      vacuousScanSuites: [],
      headCommit: HEAD,
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "a-combinatorial-statement-reports-the-gaps-it-never-ran",
    input: {
      obligations: [
        obligation({
          statement: "2-way over os(linux|mac) arch(x64|arm64)",
        }),
      ],
      bindings: [binding()],
      runs: [
        run({
          entries: [
            {
              symbol: "test_records",
              outcome: "pass",
              config: { os: "linux", arch: "x64" },
            },
          ],
        }),
      ],
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "a-full-covering-array-raises-no-combinatorial-gap",
    input: {
      obligations: [
        obligation({
          statement: "2-way over os(linux|mac) arch(x64|arm64)",
        }),
      ],
      bindings: [binding({ symbols: ["a", "b", "c", "d"] })],
      runs: [
        run({
          entries: [
            {
              symbol: "a",
              outcome: "pass",
              config: { os: "linux", arch: "x64" },
            },
            {
              symbol: "b",
              outcome: "pass",
              config: { os: "linux", arch: "arm64" },
            },
            {
              symbol: "c",
              outcome: "pass",
              config: { os: "mac", arch: "x64" },
            },
            {
              symbol: "d",
              outcome: "pass",
              config: { os: "mac", arch: "arm64" },
            },
          ],
        }),
      ],
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "an-evidence-kind-that-contradicts-the-method-is-a-conformance-finding",
    input: {
      obligations: [obligation({ method: "unit-testing" })],
      bindings: [binding()],
      runs: [run({ evidenceKind: "mutation" })],
      catalog: CATALOG,
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "an-undeclared-evidence-kind-makes-conformance-unanswerable-not-failed",
    input: {
      obligations: [obligation({ method: "Inspection" })],
      bindings: [binding()],
      runs: [run({ evidenceKind: "unit" })],
      catalog: CATALOG,
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "one-method-where-criticality-demands-two-is-insufficient-multiplicity",
    input: {
      obligations: [obligation({ criticality: "P0", method: "unit-testing" })],
      bindings: [binding()],
      runs: [run({ evidenceKind: "unit" })],
      catalog: CATALOG,
      multiplicityRequires: ["P0"],
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "a-mutation-score-below-the-floor-is-a-finding",
    input: {
      obligations: [obligation({ criticality: "P0" })],
      bindings: [binding()],
      runs: [
        run({
          entries: [
            {
              symbol: "test_records",
              outcome: "pass",
              score: 0.4,
              metric: "mutation-score",
            },
          ],
        }),
      ],
      mutationFloor: { P0: 0.8 },
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "a-score-without-the-mutation-metric-is-not-a-mutation-score",
    input: {
      obligations: [obligation({ criticality: "P0" })],
      bindings: [binding()],
      runs: [
        run({
          entries: [
            {
              symbol: "test_records",
              outcome: "pass",
              score: 0.4,
              metric: "coverage",
            },
          ],
        }),
      ],
      mutationFloor: { P0: 0.8 },
      mockInspectionSuites: ["unit"],
    },
  },
  {
    name: "insufficient-independence-stops-the-ladder-and-is-reported",
    input: {
      obligations: [obligation()],
      bindings: [binding()],
      runs: [run()],
      mockInspectionSuites: ["unit"],
      independencePolicy: {
        schemaVersion: 1,
        profile: "AP-001",
        requirements: [
          {
            id: "IR-1",
            obligation: "FR-001-AC-1",
            dimensions: ["actor", "technique"],
            rationale: "two lines",
          },
        ],
      },
      independence: [
        {
          profile: "AP-001",
          requirement: "IR-1",
          obligation: "FR-001-AC-1",
          status: "insufficient",
          dimensions: [
            { dimension: "actor", values: ["a"], missingSuites: [] },
            { dimension: "technique", values: ["t"], missingSuites: [] },
          ],
          summary: "no pair of suites differs on every requested dimension",
        },
      ],
    },
  },
  {
    name: "a-requirement-no-obligation-loop-reached-still-appears-in-the-report",
    input: {
      obligations: [],
      bindings: [],
      runs: [],
      independencePolicy: {
        schemaVersion: 1,
        profile: "AP-001",
        requirements: [
          {
            id: "IR-1",
            obligation: "FR-999-AC-1",
            dimensions: ["actor"],
            rationale: "two lines",
          },
        ],
      },
      independence: [
        {
          profile: "AP-001",
          requirement: "IR-1",
          obligation: "FR-999-AC-1",
          status: "satisfied",
          dimensions: [
            { dimension: "actor", values: ["a", "b"], missingSuites: [] },
          ],
          satisfiedBy: ["unit", "integration"],
          summary: "unit and integration differ on actor",
        },
      ],
    },
  },
  {
    name: "obligations-and-findings-sort-by-code-unit-not-by-locale",
    // U+FFFD is one scalar BELOW U+10000 and two code units ABOVE it, so
    // `Array.prototype.sort` and Rust's `str: Ord` disagree here.
    input: {
      obligations: [
        obligation({ id: "\u{10000}" }),
        obligation({ id: "�" }),
        obligation({ id: "FR-001-AC-1" }),
      ],
      bindings: [],
      runs: [],
    },
  },
];

// ── advise cases ───────────────────────────────────────────────────────────

const ADVISE_CASES = [
  {
    name: "nothing-matches-and-the-advisor-says-so",
    facts: { id: "FR-001-AC-1", statement: "The system shall exist." },
  },
  {
    name: "a-characteristic-recommends-the-method-that-declares-it",
    facts: {
      id: "FR-001-AC-2",
      statement: "When the door opens the lock shall transition to unlocked.",
    },
  },
  {
    name: "a-hyphen-compound-does-not-match-the-bare-word",
    facts: {
      id: "FR-001-AC-3",
      statement: "The state-machine-free path shall hold.",
    },
  },
  {
    name: "an-archetype-alone-recommends-inspection",
    facts: {
      id: "StR-001-AC-1",
      statement: "The stakeholder wants it.",
      archetype: "StR",
    },
  },
  {
    name: "high-criticality-is-read-from-the-field-not-from-the-prose",
    facts: {
      id: "FR-002-AC-1",
      statement: "The system shall exist.",
      criticality: "P0",
    },
  },
  {
    name: "an-authored-method-outside-the-recommendations-is-a-mismatch",
    facts: {
      id: "FR-001-AC-4",
      statement: "When the door opens the lock shall transition to unlocked.",
      authoredMethod: "Inspection",
    },
  },
  {
    name: "an-uncatalogued-authored-method-is-never-a-mismatch",
    facts: {
      id: "FR-001-AC-5",
      statement: "When the door opens the lock shall transition to unlocked.",
      authoredMethod: "Vibes",
      uncataloguedMethod: true,
    },
  },
  {
    name: "an-unmeasured-fault-detection-is-not-a-failed-one",
    facts: {
      id: "FR-003-AC-1",
      statement: "The system shall exist.",
      evidence: { bound: true, faultDetectionScores: [] },
    },
  },
  {
    name: "a-failing-fault-detection-score-mints-the-failed-characteristic",
    facts: {
      id: "FR-003-AC-2",
      statement: "The system shall exist.",
      evidence: { bound: true, faultDetectionScores: [0.2, 0.9] },
    },
  },
  {
    name: "structured-parameters-are-read-and-not-re-derived-from-prose",
    facts: {
      id: "NFR-001-M-1",
      statement: "The system shall be fast.",
      parameters: { target: "< 4 min", threshold: "< 5 min" },
    },
  },
  {
    name: "two-matching-rules-outrank-one-and-the-id-breaks-a-tie",
    facts: {
      id: "FR-006-AC-1",
      statement: "When the door opens the lock shall transition to unlocked.",
      archetype: "FR",
    },
  },
  {
    name: "an-object-type-is-an-axis-of-its-own",
    facts: {
      id: "FR-004-AC-1",
      statement: "The system shall exist.",
      objectTypes: ["Order"],
    },
  },
  {
    name: "a-property-shape-is-an-axis-of-its-own",
    facts: {
      id: "FR-005-AC-1",
      statement: "The system shall exist.",
      propertyShape: "round-trip",
    },
  },
];

const AXIS_CATALOG = {
  ...CATALOG,
  methods: [
    ...CATALOG.methods,
    {
      id: "type-inspection",
      name: "Type inspection",
      class: "Inspection",
      definition: "Read the types.",
      applicability: {
        object_types: ["Order"],
        property_shapes: ["round-trip"],
      },
      tooling: [],
      moduleName: "extra",
    },
    {
      id: "dual-axis",
      name: "Two axes at once",
      class: "Analysis",
      definition: "Matches on two rules, so it outranks a one-rule match.",
      applicability: { characteristics: ["state-machine"], archetypes: ["FR"] },
      tooling: [],
      moduleName: "extra",
    },
    {
      id: "unobservable-axis",
      name: "Unobservable axis",
      class: "Analysis",
      definition: "Names an axis the advisor cannot observe.",
      applicability: { phase_of_the_moon: ["waxing"] },
      tooling: [],
      moduleName: "extra",
    },
  ],
};

// ── characteristicsOf cases ────────────────────────────────────────────────

const CHARACTERISTIC_CASES = [
  { name: "plain-prose-matches-nothing", statement: "The system shall exist." },
  {
    name: "a-markdown-link-target-is-not-prose",
    statement: "See [the guide](docs/invariant-and-user-rules.md).",
  },
  {
    name: "a-hyphen-compound-on-the-left-does-not-count",
    statement: "The non-user path shall hold.",
  },
  {
    name: "a-hyphen-compound-on-the-right-does-not-count",
    statement: "The user-free path shall hold.",
  },
  {
    name: "a-non-ascii-neighbour-is-not-a-word-character-to-a-unicode-less-regexp",
    // `\w` without the `u` flag is ASCII in JavaScript, so `é-user` is NOT a
    // compound and the bare word matches. A Rust `\w` is Unicode by default
    // and would disagree; this case is what holds the `(?i-u)` prefix in place.
    statement: "The é-user path shall hold.",
  },
  {
    name: "latency-mixes-unicode-whitespace-with-ascii-word-boundaries",
    statement: "Response latency shall be ≤ 200 ms.",
  },
  {
    name: "a-quantified-threshold-is-recognised-with-a-unicode-comparator",
    statement: "Throughput shall be ≥ 1000 requests per second.",
  },
  {
    name: "degradation-is-recognised-across-components",
    statement: "On partial failure the service shall degrade gracefully.",
  },
  {
    name: "a-configuration-space-mints-configuration-matrix",
    statement: "2-way over os(linux|mac) arch(x64|arm64)",
  },
  {
    name: "criticality-is-taken-verbatim-and-case-insensitively",
    statement: "The system shall exist.",
    criticality: "HIGH",
  },
  {
    name: "a-criticality-outside-the-three-words-mints-nothing",
    statement: "The system shall exist.",
    criticality: "P1",
  },
  {
    name: "an-unbound-obligation-mints-no-fault-detection-characteristic",
    statement: "The system shall exist.",
    evidence: { bound: false, faultDetectionScores: [] },
  },
  {
    name: "a-nan-score-propagates-through-the-minimum",
    statement: "The system shall exist.",
    evidence: { bound: true, faultDetectionScores: [0.9, 0.95] },
  },
  {
    name: "a-threshold-parameter-alone-is-enough",
    statement: "The system shall exist.",
    parameters: { threshold: "< 5 min" },
  },
];

// ── ratchet and delta cases ────────────────────────────────────────────────

const REPORT_A = {
  findings: [
    {
      kind: "undischarged",
      obligation: "FR-001-AC-1",
      severity: "medium",
      summary: "a",
    },
    {
      kind: "suspect-link",
      obligation: "FR-002-AC-1",
      severity: "high",
      summary: "b",
    },
  ],
  healthy: [],
  unevaluated: [],
};
const REPORT_B = {
  findings: [
    {
      kind: "suspect-link",
      obligation: "FR-002-AC-1",
      severity: "high",
      summary: "b",
    },
    {
      kind: "stale-evidence",
      obligation: "FR-003-AC-1",
      severity: "high",
      summary: "c",
    },
  ],
  healthy: [],
  unevaluated: [],
};

const RATCHET_CASES = [
  { name: "an-empty-baseline-accepts-nothing", report: REPORT_A, accepted: [] },
  {
    name: "a-baselined-key-is-filtered",
    report: REPORT_A,
    accepted: ["undischarged:FR-001-AC-1"],
  },
  {
    name: "a-baseline-key-for-another-kind-does-not-filter",
    report: REPORT_A,
    accepted: ["stale-evidence:FR-001-AC-1"],
  },
  {
    name: "every-key-baselined-leaves-nothing",
    report: REPORT_A,
    accepted: ["undischarged:FR-001-AC-1", "suspect-link:FR-002-AC-1"],
  },
];

// ── module catalog cases ───────────────────────────────────────────────────

const MANIFEST = `name: core
verification_catalog:
  unit-testing:
    name: Unit testing
    class: Test
    definition: Exercise the unit.
    evidence_kind: unit
    applicability:
      characteristics: [state-transition]
      broken_rule: not-a-list
    tooling: [jest, 1]
`;

const SECOND = `name: extra
verification_catalog:
  unit-testing:
    name: Unit testing, again
    class: Analysis
    definition: A collision.
  inspection:
    name: Inspection
    class: Inspection
    definition: Read it.
`;

const COERCING = `name: 1.0
verification_catalog:
  odd:
    name: true
    class: 2
    definition:
    evidence_kind: 7
    tooling: [1, 2]
`;

const NO_CATALOG = `name: empty
`;

const MALFORMED = `name: broken
verification_catalog: [\n`;

const CATALOG_CASES = [
  {
    name: "one-module-is-read-and-coerced",
    modules: { "mod-core": MANIFEST },
    roots: ["mod-core"],
  },
  {
    name: "first-module-wins-and-the-collision-is-reported",
    modules: { "mod-core": MANIFEST, "mod-extra": SECOND },
    roots: ["mod-core", "mod-extra"],
  },
  {
    name: "root-order-decides-the-winner",
    modules: { "mod-core": MANIFEST, "mod-extra": SECOND },
    roots: ["mod-extra", "mod-core"],
  },
  {
    name: "a-repeated-root-is-read-once",
    modules: { "mod-core": MANIFEST },
    roots: ["mod-core", "mod-core"],
  },
  {
    name: "every-field-is-string-coerced-the-way-javascript-does",
    modules: { "mod-odd": COERCING },
    roots: ["mod-odd"],
  },
  {
    name: "a-manifest-without-a-catalog-contributes-nothing",
    modules: { "mod-empty": NO_CATALOG },
    roots: ["mod-empty"],
  },
  {
    name: "a-malformed-manifest-is-reported-not-thrown",
    modules: { "mod-broken": MALFORMED },
    roots: ["mod-broken"],
  },
  {
    name: "a-root-that-does-not-exist-is-skipped",
    modules: {},
    roots: ["mod-absent"],
  },
  {
    name: "a-parent-directory-resolves-to-its-one-module-child",
    modules: { "parent/mod-core": MANIFEST },
    roots: ["parent"],
  },
];

// ── run ────────────────────────────────────────────────────────────────────

const PLACEHOLDER = "/modules";
const home = mkdtempSync(join(tmpdir(), "quoin-383-"));

const catalogCases = CATALOG_CASES.map(({ name, modules, roots }) => {
  const base = mkdtempSync(join(home, "case-"));
  for (const [rel, text] of Object.entries(modules)) {
    const dir = join(base, rel);
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "manifest.yaml"), text, "utf8");
  }
  const catalog = loadMethodCatalog(roots.map((rel) => join(base, rel)));
  const rewrite = (value) =>
    JSON.parse(JSON.stringify(value).split(base).join(PLACEHOLDER));
  return {
    name,
    modules,
    roots,
    catalog: rewrite(catalog),
    classes: methodClasses(catalog),
  };
});

const auditCases = AUDIT_CASES.map(({ name, input }) => {
  try {
    return { name, input, report: audit(input), threw: null };
  } catch (cause) {
    return {
      name,
      input,
      report: null,
      threw: cause instanceof Error ? cause.message : String(cause),
    };
  }
});

const adviseCases = ADVISE_CASES.map(({ name, facts }) => ({
  name,
  facts,
  advice: advise(AXIS_CATALOG, facts),
}));

const characteristicCases = CHARACTERISTIC_CASES.map(
  ({ name, statement, criticality, evidence, parameters }) => ({
    name,
    statement,
    criticality: criticality ?? null,
    evidence: evidence ?? null,
    parameters: parameters ?? null,
    characteristics: characteristicsOf(
      statement,
      criticality,
      evidence,
      parameters,
    ),
  }),
);

const ratchetCases = RATCHET_CASES.map(({ name, report, accepted }) => ({
  name,
  report,
  accepted,
  remaining: ratchet(report, { accepted }),
}));

const deltaResult = delta(REPORT_A, REPORT_B);

const uncataloguedCases = [
  { name: "no-diagnostics", diagnostics: [] },
  {
    name: "a-valued-diagnostic-lands-in-the-set",
    diagnostics: [
      { reason: "uncatalogued-verification-method", value: "Vibes" },
    ],
  },
  {
    name: "a-valueless-diagnostic-degrades",
    diagnostics: [{ reason: "uncatalogued-verification-method" }],
  },
  {
    name: "another-reason-is-ignored",
    diagnostics: [{ reason: "something-else", value: "Vibes" }],
  },
].map(({ name, diagnostics }) => {
  const result = uncataloguedAuthoredMethods(diagnostics);
  return {
    name,
    diagnostics,
    values: [...result.values].sort(),
    degraded: result.degraded,
  };
});

const scoresCases = [
  {
    name: "a-skipped-entry-contributes-no-score",
    bindings: [binding()],
    runs: [
      run({
        entries: [
          {
            symbol: "test_records",
            outcome: "skip",
            score: 0.9,
            metric: "mutation-score",
          },
        ],
      }),
    ],
  },
  {
    name: "only-the-mutation-metric-counts",
    bindings: [binding({ symbols: ["a", "b"] })],
    runs: [
      run({
        entries: [
          {
            symbol: "a",
            outcome: "pass",
            score: 0.5,
            metric: "mutation-score",
          },
          { symbol: "b", outcome: "pass", score: 0.9, metric: "coverage" },
        ],
      }),
    ],
  },
].map(({ name, bindings, runs }) => ({
  name,
  bindings,
  runs,
  scores: scoresFor(bindings, runs),
}));

process.stdout.write(
  `${JSON.stringify(
    {
      mockSubjectFloor: MOCK_SUBJECT_FLOOR,
      mintableCharacteristics: [...mintableCharacteristics()].sort(),
      catalogCases,
      auditCases,
      adviseCatalog: AXIS_CATALOG,
      adviseCases,
      characteristicCases,
      ratchetCases,
      delta: {
        before: REPORT_A,
        after: REPORT_B,
        added: deltaResult.added,
        resolved: deltaResult.resolved,
      },
      uncataloguedCases,
      scoresCases,
    },
    null,
    2,
  )}\n`,
);

rmSync(home, { recursive: true, force: true });
