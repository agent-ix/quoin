// Declarative eval scenarios (TC-EV-001..TC-EV-015 from spec/evals.md).
//
// Each scenario: { id, useCase, prompt, expect, setup?(ctx), env?(ctx) }.
//  - prompt:  task text for the agent (string or (ctx)=>string). The EV "Prompt" column.
//  - setup:   optional pre-seed of brownfield/broken/plugin/dev-module fixtures into ctx.
//             May stash data on ctx.data for prompt(ctx)/env(ctx).
//  - env:     optional (ctx)=>extra env for BOTH the agent run and the ground-truth
//             assertion (e.g. QUOIN_MODULE_PATHS).
//  - expect:  ground-truth success checks (see lib/assert.mjs):
//             files, absentFiles, validate, artifacts, flow, cliRejects, plugin,
//             resolvesTo, sentinel.
//
// TC-EV-021..TC-EV-025 are the artifact-completeness set: they drive `/specify` for
// request shapes (new project / add US / edit FR / add US+FR / backport) and use
// the `artifacts` check to assert the EXACT artifact types were authored as
// discrete files (an FR written only as a row in spec.md's table fails).
//
// The two canaries (TC-EV-001 greenfield, TC-EV-008 repair loop) are the cheapest, highest
// -signal scenarios; `--canary` runs only those.

import {
  copySkeleton,
  removeSection,
  makeLocalPlugin,
  makeDevModule,
  makeRepos,
  removeSeededModule,
  writeModuleFile,
  writeRepoFile,
} from "../lib/fixtures.mjs";

// --- Shared fixture for the gap-analysis scenarios (TC-EV-030..TC-EV-033) -----------
// Seeds a PLAN-001 bundle + Test Matrix + code/tests with configurable gaps so each
// scenario exercises a different happy/sad branch of the gap-analysis skill:
//   taskDone  - TASK-002 status: true=done, false=not_started          (Step 2 gap)
//   tc2       - matrix TC-002 backing test:
//                 "real"   = tagged test with a real assertion          (no gap)
//                 "hollow" = tagged test that asserts nothing — Step 3 passes on the
//                            tag, only the optional Step 5 (semantic) catches it
//                 "none"   = no tagged test                             (Step 3 gap)
//   untraced  - extra source fn with no owning requirement             (Step 4 gap):
//                 false | "purge" (destructive→high) | "listCodes" (read-only→medium)
function seedGapBundle(ctx, { taskDone, tc2, untraced }) {
  // A real FR whose intent matches the code/tests, so the only gaps are the ones
  // each scenario injects (a generic skeleton FR would itself read as a spec↔code
  // mismatch and pollute the happy path).
  writeRepoFile(
    ctx,
    "spec/functional/FR-001.md",
    [
      "---",
      "id: FR-001",
      'title: "Shorten and resolve URLs"',
      "type: FR",
      "relationships:",
      "  - target: ix://agent-ix/eval/US-001",
      "    type: implements",
      "---",
      "# FR-001: Shorten and resolve URLs",
      "",
      "## Description",
      "",
      "The system SHALL generate a short code for a long URL and resolve a short " +
        "code back to the original URL.",
      "",
      "## Acceptance Criteria",
      "",
      "| ID | Criteria | Verification |",
      "| --- | --- | --- |",
      "| FR-001-AC-1 | shorten(url) returns a 6-character code and stores the mapping | Test |",
      "| FR-001-AC-2 | resolve(code) returns the original URL for a stored code | Test |",
      "",
    ].join("\n"),
  );
  writeRepoFile(
    ctx,
    "spec/spec.md",
    [
      "---",
      "id: SPEC-001",
      'title: "URL shortener"',
      "type: master-requirements",
      "org: agent-ix",
      "name: eval",
      "---",
      "# URL shortener",
      "",
      "A small service that shortens URLs and resolves them back.",
      "",
    ].join("\n"),
  );
  // Matrix claims BOTH TCs complete (✅); whether that is true depends on `tc2`.
  writeRepoFile(
    ctx,
    "spec/matrix.md",
    [
      "---",
      "id: TM-001",
      'title: "URL shortener Test Matrix"',
      "type: TestMatrix",
      "---",
      "# URL shortener Test Matrix",
      "",
      "## Test Case Summary",
      "",
      "| Test ID | Title | Type | Priority | Traces To | Status |",
      "| ------- | ----- | ---- | -------- | --------- | ------ |",
      "| TC-001 | shorten returns a code | Unit | P0 | FR-001-AC-1 | ✅ |",
      "| TC-002 | resolve round-trips a code | Unit | P0 | FR-001-AC-2 | ✅ |",
      "",
    ].join("\n"),
  );
  writeRepoFile(
    ctx,
    "plan/PLAN-001-core/plan.md",
    [
      "---",
      "id: PLAN-001",
      'title: "Core plan"',
      "type: Plan",
      "status: active",
      "relationships:",
      "  - target: ix://agent-ix/eval/FR-001",
      "    type: references",
      "---",
      "# PLAN-001: Core plan",
      "",
      "## Scope",
      "Implement and test the URL shortener (FR-001).",
      "",
    ].join("\n"),
  );
  writeRepoFile(
    ctx,
    "plan/PLAN-001-core/index.md",
    [
      "---",
      "type: index",
      'title: "PLAN-001 — Core plan"',
      'description: "Contents of the PLAN-001 bundle."',
      'okf_version: "0.1"',
      "---",
      "# PLAN-001 — Core plan",
      "",
      "## Contents",
      "",
      "* [PLAN-001: Core plan](./plan.md) - Plan overview.",
      "* [TASK-001](./tasks/TASK-001-impl.md) - shorten/resolve impl.",
      "* [TASK-002](./tasks/TASK-002-tests.md) - tests.",
      "",
    ].join("\n"),
  );
  writeRepoFile(
    ctx,
    "plan/PLAN-001-core/log.md",
    [
      "---",
      "type: log",
      'title: "PLAN-001 — Update Log"',
      'description: "Change log for the PLAN-001 bundle."',
      "---",
      "# PLAN-001 — Update Log",
      "",
      "## History",
      "",
      "* **2026-06-21** — Plan created covering FR-001.",
      "",
    ].join("\n"),
  );
  const task = (id, title, status, tc) =>
    [
      "---",
      `id: ${id}`,
      `title: "${title}"`,
      "type: Task",
      `status: ${status}`,
      "track: A",
      "priority: P0",
      "relationships:",
      "  - target: ix://agent-ix/eval/FR-001",
      "    type: references",
      `  - target: ix://agent-ix/eval/${tc}`,
      "    type: verifies",
      "---",
      `# ${id}: ${title}`,
      "",
      "## Scope",
      `${title}.`,
      "",
    ].join("\n");
  writeRepoFile(
    ctx,
    "plan/PLAN-001-core/tasks/TASK-001-impl.md",
    task("TASK-001", "shorten/resolve implementation", "done", "TC-001"),
  );
  writeRepoFile(
    ctx,
    "plan/PLAN-001-core/tasks/TASK-002-tests.md",
    task(
      "TASK-002",
      "shorten/resolve tests",
      taskDone ? "done" : "not_started",
      "TC-002",
    ),
  );
  const src = [
    "export function shorten(url, store) {",
    "  let h = 0;",
    "  for (const c of url) h = (h * 31 + c.charCodeAt(0)) | 0;",
    "  const code = Math.abs(h).toString(36).slice(0, 6);",
    "  store.set(code, url);",
    "  return code;",
    "}",
    "export function resolve(code, store) {",
    "  if (!store.has(code)) throw new Error('unknown code');",
    "  return store.get(code);",
    "}",
  ];
  if (untraced === "purge") {
    src.push(
      "// Not covered by any requirement (FR-001 says nothing about deletion).",
      "export function purge(store) {",
      "  store.clear();",
      "}",
    );
  } else if (untraced === "listCodes") {
    src.push(
      "// Not covered by any requirement (read-only enumeration API).",
      "export function listCodes(store) {",
      "  return [...store.keys()];",
      "}",
    );
  }
  src.push("");
  writeRepoFile(ctx, "src/shorten.mjs", src.join("\n"));
  const tests = [
    "// Trace: FR-001-AC-1 / TC-001",
    "import { shorten, resolve } from '../src/shorten.mjs';",
    "test('shorten returns a 6-char code', () => {",
    "  const store = new Map();",
    "  expect(shorten('https://x.test', store)).toHaveLength(6);",
    "});",
  ];
  if (tc2 === "real") {
    tests.push(
      "// Trace: FR-001-AC-2 / TC-002",
      "test('resolve round-trips a stored code', () => {",
      "  const store = new Map();",
      "  const code = shorten('https://y.test', store);",
      "  expect(resolve(code, store)).toBe('https://y.test');",
      "});",
    );
  } else if (tc2 === "hollow") {
    tests.push(
      "// Trace: FR-001-AC-2 / TC-002  (hollow: tagged but asserts nothing about resolve)",
      "test('resolve works', () => {",
      "  expect(true).toBe(true);",
      "});",
    );
  }
  tests.push("");
  writeRepoFile(ctx, "tests/shorten.test.mjs", tests.join("\n"));
}

// --- Shared fixtures for the spec-correctness scenarios (TC-EV-050..TC-EV-053) ------
//
// FR-028's skill consumes `quire properties --json`, so each scenario needs a
// repo whose criteria land in known classification buckets, plus a source file
// the clauses can be grounded against. The fixture is a tiny "codes" library:
// `normalizeCode` (idempotent, total) and `parseCodeList` (throws on unknown),
// which between them give one criterion per lane the skill routes on.

/** A JS library the criteria below can be grounded against. */
function seedCorrectnessSource(ctx) {
  writeRepoFile(
    ctx,
    "src/codes.js",
    [
      "const KNOWN = new Set(['alpha', 'bravo', 'charlie']);",
      "",
      "/** Lowercase and trim a code. Idempotent. */",
      "export function normalizeCode(code) {",
      "  return String(code).trim().toLowerCase();",
      "}",
      "",
      "/** Split a comma list into known codes, sorted. Throws on an unknown one. */",
      "export function parseCodeList(value) {",
      "  const codes = String(value)",
      "    .split(',')",
      "    .map(normalizeCode)",
      "    .filter(Boolean);",
      "  for (const code of codes) {",
      "    if (!KNOWN.has(code)) {",
      "      throw new Error(`unknown code: ${code}`);",
      "    }",
      "  }",
      "  return [...codes].sort();",
      "}",
    ].join("\n"),
  );
}

/**
 * An FR whose acceptance criteria span the classification buckets.
 *
 * `withGroundable` false swaps the groundable criteria for ones the skill must
 * refuse — an adjectival oracle and a symbol that does not exist — which is what
 * TC-EV-053 exercises.
 */
function correctnessFr(withGroundable) {
  const criteria = withGroundable
    ? [
        "| FR-001-AC-1 | A normalized code normalizes to itself | Test |",
        "| FR-001-AC-2 | A parsed code list is returned in sorted order | Test |",
        "| FR-001-AC-3 | An unknown code raises an error naming the code | Test |",
        "| FR-001-AC-4 | `parseCodeList('alpha')` returns `['alpha']` | Test |",
      ]
    : [
        "| FR-001-AC-1 | An error message is actionable and clear to the reader | Inspection |",
        "| FR-001-AC-2 | A retired code is rejected by `parseRetiredCode` | Test |",
        "| FR-001-AC-3 | `parseCodeList('alpha')` returns `['alpha']` | Test |",
      ];
  return [
    "---",
    "id: FR-001",
    'title: "Normalize and parse codes"',
    "type: FR",
    "---",
    "# [FR-001] Normalize and parse codes",
    "",
    "## Description",
    "",
    "The library shall normalize a code by trimming and lowercasing it.",
    "",
    "The library shall reject a code list containing an unknown code.",
    "",
    "## Inputs",
    "",
    "- A code string, and a comma-separated code list.",
    "",
    "## Outputs",
    "",
    "- The normalized code, or the sorted list of known codes, or a raised error",
    "  naming the offending code.",
    "",
    "## Behavior",
    "",
    "- The library shall lowercase and trim a code, so normalizing an already",
    "  normalized code returns it unchanged.",
    "- The library shall return a parsed code list in sorted order.",
    "- Where a code list contains a code outside the known set, the library shall",
    "  raise an error naming that code.",
    "",
    "## Acceptance Criteria",
    "",
    "| ID | Criteria | Verification |",
    "|----|----------|--------------|",
    ...criteria,
    "",
    "## Dependencies",
    "",
    "- **Upstream**: none",
    "- **Downstream**: none",
  ].join("\n");
}

/** A package.json, with fast-check present or absent. */
function seedCorrectnessPackage(ctx, { fastCheck }) {
  writeRepoFile(
    ctx,
    "package.json",
    JSON.stringify(
      {
        name: "codes",
        version: "0.1.0",
        type: "module",
        scripts: { test: "vitest run" },
        devDependencies: fastCheck
          ? { "fast-check": "^4.9.0", vitest: "^4.1.8" }
          : { vitest: "^4.1.8" },
      },
      null,
      2,
    ),
  );
}

function seedCorrectness(ctx, { fastCheck = true, groundable = true } = {}) {
  seedCorrectnessSource(ctx);
  seedCorrectnessPackage(ctx, { fastCheck });
  writeRepoFile(ctx, "spec/functional/FR-001.md", correctnessFr(groundable));
}

/** The skill's own guardrail: it must read the classification, not guess. */
const CORRECTNESS_AGENT_RAN = [
  {
    pattern: "quire\\s+properties\\b[\\s\\S]*--json",
    desc: "read the classification (quire properties --json)",
  },
];

const correctnessPrompt =
  "Use the spec-correctness skill on this repository. Classify the acceptance " +
  "criteria in spec/ with `quire properties`, ground each one against src/, and " +
  "emit what it settles. Record everything you could not ground in the review " +
  "artifact the skill specifies.";

// --- Shared fixtures for the spec-fuzz scenarios (TC-EV-054..TC-EV-057) ------------
//
// FR-038's skill consumes `quoin advise --json` and selects obligations whose
// verification method carries `evidence_kind: Fuzz`. The fixture is the same
// tiny "codes" library the spec-correctness scenarios use — it already has a
// real parser (`parseCodeList`) that throws on bad input, which is exactly the
// shape a fuzz target needs to be groundable against.

/**
 * An FR whose criteria are about an input surface, so the advisor reaches
 * `fuzzing` through the `parser` and `untrusted-input` characteristics.
 *
 * `groundable: false` swaps the surface for one no function in `src/` provides,
 * which is the refusal TC-EV-055 exercises.
 */
function fuzzFr(groundable) {
  const criteria = groundable
    ? [
        "| FR-002-AC-1 | The code-list parser never panics on untrusted input | Test |",
        "| FR-002-AC-2 | Malformed input is rejected rather than crashing the parser | Test |",
      ]
    : [
        "| FR-002-AC-1 | The binary frame decoder never panics on untrusted input | Test |",
      ];
  return [
    "---",
    "id: FR-002",
    'title: "Parse untrusted code lists"',
    "type: FR",
    "---",
    "",
    "# FR-002: Parse untrusted code lists",
    "",
    "## Description",
    "",
    "The parser SHALL reject malformed input rather than crashing.",
    "",
    "## Inputs",
    "",
    groundable
      ? "- A comma-separated code list, from an untrusted caller, read by `parseCodeList`."
      : "- A length-prefixed binary frame, from an untrusted caller.",
    "",
    "## Acceptance Criteria",
    "",
    "| ID | Criteria | Verification |",
    "|----|----------|--------------|",
    ...criteria,
    "",
  ].join("\n");
}

/**
 * A module renaming the catalog's fuzz method.
 *
 * FR-038-CON-1: the skill selects on `evidence_kind: Fuzz`, never on the name
 * `fuzzing`. A skill that pattern-matched the name would pass every other
 * assertion in TC-EV-054 and fail only here, which is the point of renaming it.
 */
function fuzzCatalogModule(ctx) {
  writeModuleFile(
    ctx,
    "fuzzcat/manifest.yaml",
    [
      "manifest_version: 1",
      "name: fuzzcat",
      "version: 0.0.0",
      "verification_catalog:",
      "  robustness-search:",
      "    name: Robustness search",
      "    class: Test",
      "    definition: >-",
      "      Execute the input surface against generated or mutated data, looking",
      "      for crashes and hangs rather than wrong answers.",
      "    evidence_kind: Fuzz",
      "    applicability:",
      "      characteristics: [untrusted-input, parser]",
      "    tooling: [fast-check]",
      "",
    ].join("\n"),
  );
}

function seedFuzz(ctx, { harness = true, groundable = true } = {}) {
  seedCorrectnessSource(ctx);
  seedCorrectnessPackage(ctx, { fastCheck: harness });
  fuzzCatalogModule(ctx);
  writeRepoFile(ctx, "spec/functional/FR-002.md", fuzzFr(groundable));
}

const FUZZ_AGENT_RAN = [
  {
    pattern: "quoin\\s+advise\\b[\\s\\S]*--json",
    desc: "read the obligations and their methods (quoin advise --json)",
  },
];

const fuzzPrompt =
  "Use the spec-fuzz skill on this repository. " +
  "Select the obligations whose verification method is fuzz-kind, ground each " +
  "one's entry point in src/, and emit what you can. Record everything you " +
  "could not serve in the review artifact the skill specifies.";

function specFuzzScenarios() {
  const REVIEW_ARTIFACT = {
    artifacts: { require: { SpecReview: { min: 1, dir: "reviews" } } },
    validate: { globs: ["reviews/*.md"], shouldPass: true },
  };
  // Nothing that would convert the repository into a fuzzing repository
  // (FR-038-CON-2). Each of these is a decision belonging to its owner.
  const NO_INSTALL = [
    "fuzz/Cargo.toml",
    "rust-toolchain.toml",
    "fuzz/corpus/*",
  ];

  return [
    {
      // TC-EV-054 — the generation lane. The catalog method is RENAMED, so a
      // skill matching on the name `fuzzing` selects nothing and fails here
      // while passing every other scenario.
      id: "TC-EV-054",
      useCase: "US-012",
      setup(ctx) {
        seedFuzz(ctx);
      },
      prompt: fuzzPrompt,
      expect: {
        agentRan: FUZZ_AGENT_RAN,
        files: ["reviews/*.md"],
        absentFiles: NO_INSTALL,
        ...REVIEW_ARTIFACT,
        // Both trace carriers plus the provenance line, and an entry point that
        // exists: `parseCodeList` is in src/codes.js.
        fileContains: [
          {
            glob: "**/*fuzz*.js",
            includes: ["FR-002-AC-", "spec-fuzz:", "parseCodeList"],
          },
        ],
      },
    },
    {
      // TC-EV-055 — the two refusals, which are NOT the same refusal. No
      // harness is a decision for the repo owner; an ungroundable entry point
      // is a gap in the spec or the code. Collapsing both into "skipped" loses
      // the remedy, so the report must name each.
      id: "TC-EV-055",
      useCase: "US-012",
      setup(ctx) {
        seedFuzz(ctx, { harness: false, groundable: false });
      },
      prompt: fuzzPrompt,
      expect: {
        agentRan: FUZZ_AGENT_RAN,
        files: ["reviews/*.md"],
        // No target written, and nothing installed to make one possible.
        absentFiles: [...NO_INSTALL, "**/*fuzz*.js"],
        ...REVIEW_ARTIFACT,
        fileContains: [{ glob: "reviews/*.md", includes: ["FR-002-AC-1"] }],
      },
    },
    {
      // TC-EV-056 — idempotent re-run, and harness from the MANIFEST. The
      // requirement's Inputs section says "binary frame", which reads Rust-ish;
      // package.json says JavaScript, and the manifest is what decides
      // (FR-038-AC-7). No framework name may reach spec/ (CON-4).
      id: "TC-EV-056",
      useCase: "US-012",
      setup(ctx) {
        seedFuzz(ctx);
      },
      prompt:
        fuzzPrompt +
        " Then run the skill a second time over the same repository without " +
        "changing anything, and report what it rewrote.",
      expect: {
        agentRan: FUZZ_AGENT_RAN,
        files: ["reviews/*.md"],
        absentFiles: NO_INSTALL,
        ...REVIEW_ARTIFACT,
        // The spec must not have acquired a framework name (FR-028-CON-2,
        // inherited as FR-038-CON-4). `excludes` on `fileContains` is the
        // harness's spelling — an invented `fileOmits` key is accepted by the
        // scenario loader and asserted by nothing, which is the silent-no-op
        // shape this whole program exists to catch.
        fileContains: [
          {
            glob: "spec/functional/FR-002.md",
            excludes: ["fast-check", "cargo-fuzz", "atheris"],
          },
        ],
      },
    },
    {
      // TC-EV-057 — the one that matters most. A generated target that has
      // never run discharges nothing, and the failure mode is a matrix row
      // reading as covered because a file exists.
      id: "TC-EV-057",
      useCase: "US-012",
      setup(ctx) {
        seedFuzz(ctx);
      },
      prompt:
        fuzzPrompt +
        " Then state plainly in your report what these targets do and do not " +
        "prove about the obligations they name.",
      expect: {
        agentRan: FUZZ_AGENT_RAN,
        files: ["reviews/*.md"],
        absentFiles: NO_INSTALL,
        ...REVIEW_ARTIFACT,
        fileContains: [{ glob: "reviews/*.md", includes: ["undischarged"] }],
      },
    },
  ];
}

function specCorrectnessScenarios() {
  // The review artifact is a `SpecReview` at reviews/, exactly as TC-EV-026 and
  // TC-EV-030 assert for gap-analysis — not an ad-hoc file in the test tree. The
  // earlier version of these scenarios required `tests/props/QUEUE.md`, so the
  // suite would have failed an agent that behaved correctly (agent-ix/quoin#63).
  const REVIEW_ARTIFACT = {
    artifacts: { require: { SpecReview: { min: 1, dir: "reviews" } } },
    validate: { globs: ["reviews/*.md"], shouldPass: true },
  };
  // Nothing the skill writes may be an invented format (FR-028-CON-1), and no
  // emitted test may be disabled in the tree (FR-028-AC-6).
  const NO_INVENTED_OUTPUT = [
    "tests/props/QUEUE.md",
    "tests/props/_review/*",
    "tests/props/*.queue.md",
  ];

  return [
    {
      // TC-EV-050 — the settled lane. fast-check is present, so the criteria the
      // classifier settled as `extractable` become real property tests under
      // tests/props/, each tagged with its row_id. The `example` criterion
      // (AC-4, a single witness) must NOT become a property test; it is a
      // finding in the review artifact.
      id: "TC-EV-050",
      useCase: "US-011",
      setup(ctx) {
        seedCorrectness(ctx, { fastCheck: true });
      },
      prompt: correctnessPrompt,
      expect: {
        agentRan: CORRECTNESS_AGENT_RAN,
        files: ["tests/props/*.test.*", "reviews/*.md"],
        absentFiles: NO_INVENTED_OUTPUT,
        ...REVIEW_ARTIFACT,
        // Every emitted tag names a row_id that exists, in a form gap-analysis
        // greps for.
        fileContains: [
          {
            glob: "tests/props/*.test.*",
            includes: ["Trace: FR-001-AC-", "spec-correctness: row=FR-001-AC-"],
            excludes: ["FR-001-AC-9", "FR-002-AC-"],
          },
          {
            glob: "reviews/*.md",
            includes: ["FR-001-AC-4"],
          },
        ],
      },
    },
    {
      // TC-EV-051 — the review record. What the skill cannot settle unattended is
      // a *finding*, not a disabled test: the artifact validates as a
      // SpecReview, and nothing in the test tree is skipped or ignored. A
      // generated test is reviewed in the pull request it arrives in.
      id: "TC-EV-051",
      useCase: "US-011",
      setup(ctx) {
        seedCorrectness(ctx, { fastCheck: true });
      },
      prompt: correctnessPrompt,
      expect: {
        agentRan: CORRECTNESS_AGENT_RAN,
        files: ["reviews/*.md"],
        absentFiles: NO_INVENTED_OUTPUT,
        ...REVIEW_ARTIFACT,
        fileContains: [
          {
            glob: "reviews/*.md",
            // The artifact declares its analysis and names the criteria.
            includes: ["analysis: spec-correctness", "FND-001", "FR-001-AC-"],
            // A review that declares a verdict on the spec has broken CON-1.
            excludes: ["verdict", "reword"],
          },
          {
            // FR-028-AC-6: an emitted test runs. No inert markers anywhere.
            glob: "tests/props/*.test.*",
            excludes: ["it.skip", "describe.skip", "test.skip"],
          },
        ],
      },
    },
    {
      // TC-EV-052 — the handoff. After spec-correctness emits, gap-analysis must
      // reconcile every emitted row_id: no unbacked row for a generated test.
      // Two SpecReviews now exist (spec-correctness + gap-analysis), which is
      // the one-doc-per-analysis model working as intended.
      id: "TC-EV-052",
      useCase: "US-011",
      setup(ctx) {
        seedCorrectness(ctx, { fastCheck: true });
      },
      prompt:
        correctnessPrompt +
        " Then run the gap-analysis skill over the same repository and confirm " +
        "its matrix verification finds every row_id you emitted. Author the " +
        "SpecReview to reviews/.",
      expect: {
        agentRan: [
          ...CORRECTNESS_AGENT_RAN,
          { pattern: "quire\\s+validate\\b", desc: "validate with quire" },
        ],
        files: ["reviews/*.md"],
        absentFiles: NO_INVENTED_OUTPUT,
        artifacts: { require: { SpecReview: { min: 2, dir: "reviews" } } },
        validate: { globs: ["reviews/*.md"], shouldPass: true },
        fileContains: [{ glob: "reviews/*.md", includes: ["FR-001-AC-"] }],
      },
    },
    {
      // TC-EV-053 — the refusals, CON-1, and FR-028-CON-1. Nothing here is
      // groundable: one criterion's oracle is adjectival ("actionable and
      // clear"), one names a symbol absent from src/, and the manifest declares
      // no generator library. The skill must write no test file, install
      // nothing, record each reason as a finding — and invent no output format
      // to record them in.
      id: "TC-EV-053",
      useCase: "US-011",
      setup(ctx) {
        seedCorrectness(ctx, { fastCheck: false, groundable: false });
      },
      prompt: correctnessPrompt,
      expect: {
        agentRan: CORRECTNESS_AGENT_RAN,
        files: ["reviews/*.md"],
        ...REVIEW_ARTIFACT,
        // No test file: a file importing an absent generator library would break
        // collection before a single test runs. And no invented report format —
        // a closed enum with no fitting value is a ticket, not a new filename.
        absentFiles: [
          "tests/props/*.test.*",
          "tests/props/*.py",
          ...NO_INVENTED_OUTPUT,
        ],
        fileContains: [
          {
            glob: "reviews/*.md",
            // Both refusal reasons and the dependency remedy are recorded.
            includes: [
              "analysis: spec-correctness",
              "fast-check",
              "FR-001-AC-1",
              "FR-001-AC-2",
            ],
            // CON-1: no verdict, no grade, no rewording suggestion.
            excludes: ["verdict", "reword", "rewrite the criteri"],
          },
        ],
      },
    },
  ];
}

// Both gap-analysis guardrails: reference quoin for the template + validate with quire.
const GAP_AGENT_RAN = [
  {
    pattern: "quoin\\s+write\\b[\\s\\S]*[Ss]pec[Rr]eview",
    desc: "fetch the SpecReview template from quoin (quoin write --types SpecReview)",
  },
  { pattern: "quire\\s+validate\\b", desc: "validate the review with quire" },
];
// Shared task prompt; `semantic` toggles the optional Step 5 (intent↔test↔code).
const gapPrompt = (semantic) =>
  "Use the gap-analysis skill to verify the plan at plan/PLAN-001-core/ against " +
  "spec/matrix.md and the code in src/ and tests/. " +
  (semantic
    ? "When the skill offers the optional semantic review (intent↔test↔code), RUN it. "
    : "Do NOT run the optional semantic review. ") +
  "Fetch the SpecReview template from quoin first with `quoin write --types SpecReview`, " +
  "then author ONE review to reviews/<today>-gap-analysis.md with `type: SpecReview` and " +
  "`analysis: gap-analysis` frontmatter, a `## Summary`, a `## Verdict` " +
  "(PASS / CONDITIONAL / FAIL), and a `## Findings` table (columns ID | Severity | " +
  "Summary | Refs, FND-NNN ids, Severity one of low/medium/high). Record every gap you " +
  "find. Validate it with quire so it passes.";
// Verdict assertion: the word right under the `## Verdict` heading (bold or plain).
// `word` may be an alternation, e.g. "(PASS|CONDITIONAL)".
const verdict = (word) => `## Verdict[\\s\\S]{0,40}\\b${word}\\b`;

export const SCENARIOS = [
  {
    id: "TC-EV-001",
    useCase: "US-001",
    prompt:
      "Create a Functional Requirement (FR) plus a `domain` object and an `entity` " +
      "object for a small feature of your choice (for example: a user can export a " +
      "report). Author one file per type under spec/.",
    expect: {
      files: ["spec/**/*.md"],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-002",
    useCase: "US-002",
    setup(ctx) {
      copySkeleton(ctx, "spec-artifacts-iso", "fr.md", "spec/FR-002.md");
    },
    prompt:
      "The repo already contains spec/FR-002.md. Discover its authoring contract " +
      "(e.g. `quoin catalog show FR` or `quoin write`), then edit only the " +
      "sections needed to change the requirement to be about verifying a signed " +
      "release manifest. Keep it valid and re-validate the changed file.",
    expect: {
      files: ["spec/FR-002.md"],
      validate: { globs: ["spec/FR-002.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-003",
    useCase: "US-003",
    setup(ctx) {
      ctx.data.pluginPath = makeLocalPlugin(
        ctx.work,
        "spec-objects-local",
        "local-widget",
      );
    },
    prompt: (ctx) =>
      `Install the local plugin at \`path:${ctx.data.pluginPath}\` using ` +
      "`quoin plugin install`, then author one `local-widget` object under spec/ " +
      "and validate it.",
    expect: {
      plugin: { name: "spec-objects-local", present: true },
      files: ["spec/**/*.md"],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-004",
    useCase: "US-004",
    setup(ctx) {
      copySkeleton(ctx, "spec-artifacts-iso", "fr.md", "spec/FR-004.md");
      copySkeleton(
        ctx,
        "spec-objects-business",
        "domain.md",
        "spec/domain-004.md",
      );
    },
    prompt:
      "The repo contains a changed set of spec files (spec/FR-004.md and " +
      "spec/domain-004.md). Run scoped Quire validation over just those files and " +
      "report the result clearly.",
    expect: {
      files: ["spec/FR-004.md", "spec/domain-004.md"],
      validate: {
        globs: ["spec/FR-*.md", "spec/domain-*.md"],
        shouldPass: true,
      },
    },
  },
  {
    id: "TC-EV-005",
    useCase: "US-005",
    setup(ctx) {
      copySkeleton(ctx, "spec-artifacts-iso", "fr.md", "spec/FR-005.md");
    },
    prompt:
      "Start a review workflow for the spec/ directory with " +
      "`quoin review --target spec/ --id eval-review`, then inspect it with " +
      "`ix-flow status eval-review`.",
    expect: {
      flow: [{ id: "eval-review", defName: "review" }],
    },
  },
  {
    id: "TC-EV-006",
    useCase: "US-001,US-004",
    prompt:
      "Create two Functional Requirements and one `domain` object that share object " +
      "templates. Fetch each type's authoring contract only ONCE (one `quoin write` " +
      "per type) and reuse it across the files. Validate all of them.",
    expect: {
      files: ["spec/**/*.md"],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-007",
    useCase: "US-002",
    prompt:
      "Author one FR and one domain object, requesting them with LOWERCASE type names " +
      "(use `quoin write . --types fr,domain`). The CLI should resolve them to the " +
      "canonical types. Validate the result.",
    expect: {
      files: ["spec/**/*.md"],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-008",
    useCase: "US-004",
    prompt:
      "The file spec/FR-008.md currently FAILS validation. Run " +
      '`quire validate --scope . "spec/FR-008.md"`, read the diagnostics, repair the ' +
      "file so it conforms to the FR archetype, and re-validate until it passes.",
    setup(ctx) {
      const target = copySkeleton(
        ctx,
        "spec-artifacts-iso",
        "fr.md",
        "spec/FR-008.md",
      );
      removeSection(target, "Acceptance Criteria");
    },
    expect: {
      files: ["spec/FR-008.md"],
      validate: { globs: ["spec/FR-008.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-009",
    useCase: "US-003",
    setup(ctx) {
      ctx.data.pluginPath = makeLocalPlugin(
        ctx.work,
        "spec-objects-lifecycle",
        "lifecycle-object",
      );
    },
    prompt: (ctx) =>
      `Using the local plugin at \`path:${ctx.data.pluginPath}\`: install it, list ` +
      "plugins to confirm, remove it, then reinstall it. Confirm the catalog reflects " +
      "each step. Leave it installed at the end.",
    expect: {
      plugin: { name: "spec-objects-lifecycle", present: true },
    },
  },
  {
    id: "TC-EV-010",
    useCase: "US-003",
    // Local "packaged" plugin stand-in (versioned manifest) for the install→resolve
    // →author→validate path. TC-EV-020 covers a REAL `github:owner/repo//subdir` install
    // that clones from GitHub over the network.
    setup(ctx) {
      ctx.data.pluginPath = makeLocalPlugin(
        ctx.work,
        "spec-objects-package",
        "package-widget",
      );
    },
    prompt: (ctx) =>
      `Install the packaged plugin at \`path:${ctx.data.pluginPath}\`, then request ` +
      "its `package-widget` type via `quoin write` and author one such object under " +
      "spec/. Validate it.",
    expect: {
      plugin: { name: "spec-objects-package", present: true },
      files: ["spec/**/*.md"],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-011",
    useCase: "US-002",
    prompt:
      "Attempt to create an authoring pack for an unknown type by running " +
      "`quoin write . --types no-such-type`. The CLI should report the missing type " +
      "and exit non-zero. Confirm that it does, then finish (do not author any file).",
    expect: {
      cliRejects: ["no-such-type"],
    },
  },
  {
    id: "TC-EV-012",
    useCase: "US-004",
    setup(ctx) {
      copySkeleton(ctx, "spec-artifacts-iso", "fr.md", "spec/good/FR-012.md");
      const bad = copySkeleton(
        ctx,
        "spec-artifacts-iso",
        "fr.md",
        "spec/bad/FR-012.md",
      );
      removeSection(bad, "Acceptance Criteria");
    },
    prompt:
      "Validate two globs for a changed-file subset: `spec/good/*.md` and " +
      "`spec/bad/*.md`. Report which files pass and which fail (with their paths). " +
      "One of them is intentionally invalid; do not fix it — just report.",
    expect: {
      files: ["spec/good/FR-012.md", "spec/bad/FR-012.md"],
    },
  },
  {
    id: "TC-EV-013",
    useCase: "US-005",
    setup(ctx) {
      copySkeleton(ctx, "spec-artifacts-iso", "fr.md", "spec/FR-013.md");
    },
    prompt:
      "After requirements are accepted, start a matrix workflow with " +
      "`quoin matrix --target spec/ --id eval-matrix` and a to-plan workflow with " +
      "`quoin to-plan --target spec/ --id eval-to-plan`. Inspect both runs with " +
      "`ix-flow status`.",
    expect: {
      flow: [
        { id: "eval-matrix", defName: "matrix" },
        { id: "eval-to-plan", defName: "to-plan" },
      ],
    },
  },
  {
    id: "TC-EV-014",
    useCase: "US-001,US-002",
    // Stretch scenario: a full early-phase spec set. Some process types (Plan,
    // TestMatrix, master-requirements) ship a schema but no skeleton, so the agent
    // must read the schema/manifest to author a valid body.
    prompt:
      "Author a Phase 0 spec set from a settled idea: a `master-requirements` spec, a " +
      "user story (`US`), a functional requirement (`FR`), a `Plan`, and a " +
      "`TestMatrix` — one file each under spec/. Use `quoin write` to fetch the " +
      "contracts. Validate the whole set.",
    expect: {
      files: ["spec/**/*.md"],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-015",
    useCase: "US-001,US-004",
    setup(ctx) {
      ctx.data.devModulePath = makeDevModule(ctx, {
        moduleName: "spec-objects-business-dev",
        type: "domain",
        srcModule: "spec-objects-business",
        marker: "DEV DOMAIN SKELETON MARKER",
      });
    },
    env: (ctx) => ({ QUOIN_MODULE_PATHS: ctx.data.devModulePath }),
    prompt:
      "A sibling development module is present (via QUOIN_MODULE_PATHS) that " +
      "redefines the `domain` object. Author one `domain` object under spec/. The " +
      "catalog should prefer the dev module deterministically. Validate the result.",
    expect: {
      resolvesTo: { type: "domain", moduleNameIncludes: "business-dev" },
      files: ["spec/**/*.md"],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },

  // ---- Extended scenarios (beyond the original spec/evals.md TC-EV-001..TC-EV-015) ----

  {
    id: "TC-EV-016",
    useCase: "US-001,US-004",
    // Multi-repo: author into two sibling repos in one session; cwd + validation
    // scope are repointed to the parent workspace by makeRepos().
    setup(ctx) {
      makeRepos(ctx, ["core", "service"]);
    },
    prompt:
      "This workspace contains TWO repos: `core/` and `service/`. Author a " +
      "Functional Requirement under `core/spec/` and a `domain` object under " +
      "`service/spec/` (use `quoin write <repo> --types ...` per repo). Validate " +
      'both with a single scoped run: `quire validate --scope . "core/spec/**/*.md" ' +
      '"service/spec/**/*.md"`.',
    expect: {
      files: ["core/spec/**/*.md", "service/spec/**/*.md"],
      validate: {
        globs: ["core/spec/**/*.md", "service/spec/**/*.md"],
        shouldPass: true,
      },
    },
  },
  {
    id: "TC-EV-017",
    useCase: "US-001,US-002",
    // Larger, realistic feature spec set with cross-references — sustained authoring.
    prompt:
      'Author a fuller feature spec under spec/ for a settled feature (e.g. "users ' +
      'can schedule reports"): one Stakeholder Requirement (`StR`), two User Stories ' +
      "(`US`), three Functional Requirements (`FR`), one Non-Functional Requirement " +
      "(`NFR`), one `domain` object, and two `entity` objects — each in its own file, " +
      "with sensible cross-references in the bodies. Fetch each type's contract once " +
      "via `quoin write`, then validate the whole set.",
    expect: {
      files: [
        "spec/StR-*.md",
        "spec/US-*.md",
        "spec/FR-*.md",
        "spec/NFR-*.md",
        "spec/domain-*.md",
        "spec/entity-*.md",
      ],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-018",
    useCase: "US-001",
    // Objects drawn from THREE different object modules in one spec — exercises
    // multi-module catalog resolution in a single authoring pass.
    prompt:
      "Author three objects that come from different modules, one file each under " +
      "spec/: a `domain` (business), an `api_endpoint` (architecture), and a " +
      "`configuration` (operational). Request them together with " +
      "`quoin write . --types domain,api_endpoint,configuration`, author from the " +
      "skeletons, and validate.",
    expect: {
      files: ["spec/**/*.md"],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-020",
    useCase: "US-003",
    // REAL GitHub install: remove the operational module from the seeded home, then
    // the agent installs it from GitHub via the subdir source and authors one of its
    // (skeleton-backed) types. `agentRan` asserts the agent's own github install
    // command succeeded (a later write also lazily reconciles defaults, so the type
    // resolving alone wouldn't prove the agent did the install).
    setup(ctx) {
      removeSeededModule(ctx, "spec-objects-operational");
    },
    prompt:
      "The `spec-objects-operational` module is not installed. Install it FROM GITHUB " +
      "with `quoin plugin install " +
      "github:agent-ix/spec-objects-operational//spec_objects_operational@v0.2.0` " +
      "(this clones the module's subdirectory from GitHub). Then author one " +
      "`configuration` object under spec/ from its skeleton and validate it.",
    expect: {
      agentRan: [
        {
          pattern: "plugin install\\s+[\"']?github:[^\\s\"']*//",
          desc: "github subdir install",
        },
      ],
      files: ["spec/**/*.md"],
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },

  // --- TC-EV-021..TC-EV-025: artifact-completeness for spec-change requests ---------
  {
    id: "TC-EV-021",
    useCase: "US-001",
    prompt:
      "Start a new spec for a small URL-shortener service: a user submits a long " +
      "URL and gets back a short code that later redirects to the original. " +
      "Initialize the spec: create the master spec.md (the master-requirements " +
      "root/index) plus the user story and the functional requirement that " +
      "implements it — each as its own file in its OKF directory.",
    expect: {
      files: ["spec/spec.md"],
      artifacts: {
        require: {
          US: { min: 1, dir: "spec/usecase" },
          FR: { min: 1, dir: "spec/functional" },
        },
      },
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-022",
    useCase: "US-001",
    setup(ctx) {
      copySkeleton(
        ctx,
        "spec-artifacts-iso",
        "us.md",
        "spec/usecase/US-001-existing.md",
      );
    },
    prompt:
      "The repo already contains spec/usecase/US-001-existing.md. Add a SECOND " +
      "user story for a different capability (for example: an administrator " +
      "disables a short code). Author it as its own file. Do not write functional " +
      "or non-functional requirements.",
    expect: {
      artifacts: {
        require: { US: { min: 2, dir: "spec/usecase" } },
        absent: ["FR", "NFR", "IT"],
      },
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-023",
    useCase: "US-002",
    setup(ctx) {
      copySkeleton(
        ctx,
        "spec-artifacts-iso",
        "fr.md",
        "spec/functional/FR-001-checksums.md",
      );
    },
    prompt:
      "The repo has spec/functional/FR-001-checksums.md. Edit ONLY that file to " +
      "change the requirement to be about verifying a signed release manifest. " +
      "Keep it valid and re-validate. Do not create any new artifacts.",
    expect: {
      artifacts: {
        require: { FR: { min: 1, dir: "spec/functional" } },
        absent: ["US", "NFR", "IT"],
      },
      validate: {
        globs: ["spec/functional/FR-001-checksums.md"],
        shouldPass: true,
      },
    },
  },
  {
    id: "TC-EV-024",
    useCase: "US-001",
    prompt:
      "Add a user story for a user exporting their data as a CSV file, and the " +
      "functional requirement that implements it. Author each as its own artifact " +
      "file and trace the FR back to the user story.",
    expect: {
      artifacts: {
        require: {
          US: { min: 1, dir: "spec/usecase" },
          FR: { min: 1, dir: "spec/functional" },
        },
      },
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    id: "TC-EV-025",
    useCase: "US-001",
    setup(ctx) {
      writeRepoFile(
        ctx,
        "src/shorten.mjs",
        [
          "// Maps a long URL to a 6-char base62 code and stores the pair.",
          "export function shorten(url, store) {",
          "  if (!/^https?:\\/\\//.test(url)) throw new Error('invalid url');",
          "  const code = Math.abs(hash(url)).toString(36).slice(0, 6);",
          "  store.set(code, url);",
          "  return code;",
          "}",
          "export function resolve(code, store) {",
          "  if (!store.has(code)) throw new Error('unknown code');",
          "  return store.get(code);",
          "}",
          "function hash(s) { let h = 0; for (const c of s) h = (h * 31 + c.charCodeAt(0)) | 0; return h; }",
          "",
        ].join("\n"),
      );
    },
    prompt:
      "Backport a spec from the code in src/shorten.mjs. Capture its behavior as " +
      "functional requirement artifacts under spec/functional/ (not just a summary " +
      "table). Validate the spec files.",
    expect: {
      artifacts: { require: { FR: { min: 1, dir: "spec/functional" } } },
      validate: { globs: ["spec/**/*.md"], shouldPass: true },
    },
  },
  {
    // Direct-render spec-review: one validated SpecReview doc per selected
    // analysis, with the coverage gate enforcing the chosen set.
    id: "TC-EV-026",
    useCase: "US-005",
    setup(ctx) {
      copySkeleton(
        ctx,
        "spec-artifacts-iso",
        "fr.md",
        "spec/functional/FR-001.md",
      );
    },
    // The agent runs a subset spec-review and produces one validated SpecReview
    // doc per selected analysis. (TC-EV-005 separately covers the ix-flow workflow
    // lifecycle.) The two `agentRan` guardrails assert the durable behaviors:
    // reference quoin for the template + validate with quire.
    //
    // REQUIRES spec-artifacts-process >= 0.3.0 (the SpecReview archetype +
    // skeleton). Until that release is pinned in default-modules.yaml, the eval
    // seed reconciles v0.2.0 (no SpecReview), so `quoin write --types SpecReview`
    // is "catalog type not found" and the template-fetch guardrail fails.
    prompt:
      "Run a spec review of the spec/ directory using the spec-review skill " +
      "(`quoin review --target spec/ --id eval-specreview`). Choose the `subset` " +
      "review set with exactly the analyses `integrity` and `dependency`. Fetch the " +
      "SpecReview template from quoin first with `quoin write --types SpecReview` and " +
      "author from it. Produce " +
      "ONE SpecReview document per selected analysis under spec/reviews/ " +
      "(spec/reviews/integrity.md and spec/reviews/dependency.md) — each with " +
      "`type: SpecReview` frontmatter, a `## Summary`, and a `## Findings` table " +
      "(columns ID | Severity | Summary | Refs, FND-NNN ids, Severity one of " +
      "low/medium/high). Validate them with quire so they pass.",
    expect: {
      // Guardrails (over many uses, keep deviations rare): the agent MUST
      // reference quoin for the template (not invent the format) and MUST
      // validate its output with quire.
      agentRan: [
        {
          pattern: "quoin\\s+write\\b[\\s\\S]*[Ss]pec[Rr]eview",
          desc: "fetch the SpecReview template from quoin (quoin write --types SpecReview)",
        },
        {
          pattern: "quire\\s+validate\\b",
          desc: "validate the docs with quire",
        },
      ],
      files: ["spec/reviews/integrity.md", "spec/reviews/dependency.md"],
      validate: { globs: ["spec/reviews/*.md"], shouldPass: true },
    },
  },
  {
    // spec-to-plan emits a multi-plan bundle: plan/<Plan-id>-<slug>/ with a
    // type: Plan plan.md, type: Task task files, and the reserved index.md/log.md.
    // The DAG/ownership/test traces live in each Task's `relationships:`
    // (depends_on/references/verifies). This asserts the bundle is authored as
    // discrete Plan + Task artifacts and validates. The agent chooses Plan/Task ids
    // valid under the active spec-artifacts-process schema (e.g. PL-001/TSK-001 under
    // the uppercase-only published schema; Plan-001/Task-001 once the mixed-case
    // schema PR is pinned in default-modules.yaml).
    //
    // Also asserts the LEAN plan shape (spec-to-plan/references/step-3): plan.md is an
    // orchestration overview, not a mirror of the bundle — the per-task matrix lives in
    // a single "Task File Mapping" table, and tracks are described once under "Remaining
    // Work", so there is NO separate "Execution Tracks" section re-stating them. This
    // `fileContains` locks in the duplication removal the slim-template change made.
    id: "TC-EV-027",
    useCase: "US-008",
    setup(ctx) {
      copySkeleton(
        ctx,
        "spec-artifacts-iso",
        "fr.md",
        "spec/functional/FR-001.md",
      );
    },
    prompt:
      "Requirements are accepted in spec/. Use the spec-to-plan skill to create a " +
      "NEW implementation plan as a bundle under plan/ — a directory " +
      "plan/<Plan-id>-<slug>/ containing a `type: Plan` plan.md, an index.md " +
      "(`type: index`), a log.md (`type: log`), and a tasks/ directory of " +
      "`type: Task` files. Decompose into at least 3 tasks and encode the " +
      "dependencies as `relationships: depends_on` edges in the task frontmatter " +
      "(plus `references` to the requirement and `verifies` to its test cases). " +
      "Validate the whole bundle with quire so it passes.",
    expect: {
      artifacts: {
        require: {
          Plan: { min: 1, dir: "plan" },
          Task: { min: 3, dir: "plan" },
        },
      },
      files: ["plan/**/index.md", "plan/**/log.md", "plan/**/tasks/*.md"],
      validate: { globs: ["plan/**/*.md"], shouldPass: true },
      // Lean plan shape: a single per-task "Task File Mapping" table is present, and
      // tracks are NOT also re-stated in a separate "Execution Tracks" section.
      fileContains: [
        {
          glob: "plan/**/plan.md",
          includes: ["## +Task File Mapping"],
          excludes: ["## +Execution Tracks"],
        },
      ],
    },
  },
  {
    // Step-0 multi-plan selection: a project already holds a plan; the agent must
    // start a SECOND, independent plan (Plan-002) without disturbing the first.
    // Asserts >=2 Plan artifacts and a Plan-002 bundle, all validating.
    id: "TC-EV-028",
    useCase: "US-008",
    setup(ctx) {
      copySkeleton(
        ctx,
        "spec-artifacts-iso",
        "fr.md",
        "spec/functional/FR-001.md",
      );
      // Seed an existing, valid Plan-001 bundle.
      writeRepoFile(
        ctx,
        "plan/Plan-001-seed/plan.md",
        [
          "---",
          "id: PLAN-001",
          'title: "Seed plan"',
          "type: Plan",
          "status: active",
          "relationships:",
          "  - target: ix://agent-ix/eval/FR-001",
          "    type: references",
          "---",
          "# PLAN-001: Seed plan",
          "",
          "## Scope",
          "Pre-existing plan in this project.",
          "",
        ].join("\n"),
      );
      writeRepoFile(
        ctx,
        "plan/Plan-001-seed/index.md",
        [
          "---",
          "type: index",
          'title: "Plan-001 — Seed plan"',
          'description: "Contents of the Plan-001 bundle."',
          'okf_version: "0.1"',
          "---",
          "# Plan-001 — Seed plan",
          "",
          "## Contents",
          "",
          "* [Plan-001: Seed plan](./plan.md) - Plan overview.",
          "* [Task-001: seed task](./tasks/Task-001-seed.md) - Seed task.",
          "",
        ].join("\n"),
      );
      writeRepoFile(
        ctx,
        "plan/Plan-001-seed/log.md",
        [
          "---",
          "type: log",
          'title: "Plan-001 — Update Log"',
          'description: "Change log for the Plan-001 bundle."',
          "---",
          "# Plan-001 — Update Log",
          "",
          "## History",
          "",
          "* **2026-06-21** — Seed plan created.",
          "",
        ].join("\n"),
      );
      writeRepoFile(
        ctx,
        "plan/Plan-001-seed/tasks/Task-001-seed.md",
        [
          "---",
          "id: TASK-001",
          'title: "seed task"',
          "type: Task",
          "status: not_started",
          "track: A",
          "priority: P1",
          "relationships:",
          "  - target: ix://agent-ix/eval/FR-001",
          "    type: references",
          "---",
          "# Task-001: seed task",
          "",
          "## Scope",
          "Seed task in Plan-001.",
          "",
        ].join("\n"),
      );
    },
    prompt:
      "This project ALREADY has a plan at plan/Plan-001-seed/. Use the spec-to-plan " +
      "skill to start a NEW, SECOND plan (do not modify Plan-001) for a follow-up " +
      "effort based on the requirements in spec/. Produce a fresh bundle " +
      "plan/Plan-002-<slug>/ with a `type: Plan` plan.md, index.md, log.md, and a " +
      "tasks/ directory of `type: Task` files. Validate all plans with quire so they " +
      "pass.",
    expect: {
      artifacts: {
        require: {
          Plan: { min: 2, dir: "plan" },
          Task: { min: 2, dir: "plan" },
        },
      },
      files: [
        "plan/Plan-002-*/plan.md",
        "plan/Plan-002-*/index.md",
        "plan/Plan-002-*/log.md",
      ],
      validate: { globs: ["plan/**/*.md"], shouldPass: true },
    },
  },
  {
    // Update-in-place: the spec gained a requirement the existing plan doesn't
    // cover; the agent must regenerate the SAME plan (add a task, refresh
    // index/log) rather than spawn a second one. `Plan max:1` + `Task min:3`
    // proves the existing bundle grew; `absentFiles` rules out a second plan.
    id: "TC-EV-029",
    useCase: "US-008",
    setup(ctx) {
      copySkeleton(
        ctx,
        "spec-artifacts-iso",
        "fr.md",
        "spec/functional/FR-001.md",
      );
      copySkeleton(
        ctx,
        "spec-artifacts-iso",
        "fr.md",
        "spec/functional/FR-002.md",
      );
      const seedTask = (id, title, fr) =>
        [
          "---",
          `id: ${id}`,
          `title: "${title}"`,
          "type: Task",
          "status: not_started",
          "track: A",
          "priority: P1",
          "relationships:",
          `  - target: ix://agent-ix/eval/${fr}`,
          "    type: references",
          "---",
          `# ${id}: ${title}`,
          "",
          "## Scope",
          `Covers ${fr}.`,
          "",
        ].join("\n");
      writeRepoFile(
        ctx,
        "plan/PLAN-001-core/plan.md",
        [
          "---",
          "id: PLAN-001",
          'title: "Core plan"',
          "type: Plan",
          "status: active",
          "relationships:",
          "  - target: ix://agent-ix/eval/FR-001",
          "    type: references",
          "---",
          "# PLAN-001: Core plan",
          "",
          "## Scope",
          "Covers FR-001 only (FR-002 not yet planned).",
          "",
        ].join("\n"),
      );
      writeRepoFile(
        ctx,
        "plan/PLAN-001-core/index.md",
        [
          "---",
          "type: index",
          'title: "PLAN-001 — Core plan"',
          'description: "Contents of the PLAN-001 bundle."',
          'okf_version: "0.1"',
          "---",
          "# PLAN-001 — Core plan",
          "",
          "## Contents",
          "",
          "* [PLAN-001: Core plan](./plan.md) - Plan overview.",
          "* [TASK-001](./tasks/TASK-001-fr001.md) - FR-001 work.",
          "* [TASK-002](./tasks/TASK-002-fr001-tests.md) - FR-001 tests.",
          "",
        ].join("\n"),
      );
      writeRepoFile(
        ctx,
        "plan/PLAN-001-core/log.md",
        [
          "---",
          "type: log",
          'title: "PLAN-001 — Update Log"',
          'description: "Change log for the PLAN-001 bundle."',
          "---",
          "# PLAN-001 — Update Log",
          "",
          "## History",
          "",
          "* **2026-06-21** — Plan created covering FR-001.",
          "",
        ].join("\n"),
      );
      writeRepoFile(
        ctx,
        "plan/PLAN-001-core/tasks/TASK-001-fr001.md",
        seedTask("TASK-001", "FR-001 implementation", "FR-001"),
      );
      writeRepoFile(
        ctx,
        "plan/PLAN-001-core/tasks/TASK-002-fr001-tests.md",
        seedTask("TASK-002", "FR-001 tests", "FR-001"),
      );
    },
    prompt:
      "The spec now has a NEW requirement, FR-002 (spec/functional/FR-002.md), that " +
      "the existing plan plan/PLAN-001-core/ does NOT cover — it currently plans only " +
      "FR-001. Use the spec-to-plan skill to UPDATE that existing plan in place: add " +
      "task(s) covering FR-002 (continuing the TASK-NNN numbering), extend the plan's " +
      "`relationships: references`, and refresh its index.md and log.md. Do NOT create " +
      "a second plan bundle. Validate all plans with quire so they pass.",
    expect: {
      artifacts: {
        require: {
          Plan: { min: 1, max: 1, dir: "plan" },
          Task: { min: 3, dir: "plan" },
        },
      },
      files: ["plan/PLAN-001-core/tasks/*.md"],
      absentFiles: ["plan/PLAN-002-*/plan.md", "plan/Plan-002-*/plan.md"],
      validate: { globs: ["plan/**/*.md"], shouldPass: true },
    },
  },
  {
    // gap-analysis verification gate: given a plan bundle, a Test Matrix, and
    // code/tests that contain DELIBERATE gaps (an incomplete task, a matrix TC with
    // no backing tagged test, and an untraced function), the agent runs the
    // gap-analysis skill and emits ONE validated SpecReview (analysis: gap-analysis)
    // to reviews/ with a Verdict + Findings table. The two `agentRan` guardrails
    // assert the durable behaviors: reference quoin for the template + validate with
    // quire. Semantic review (Step 5) is explicitly declined to keep the run cheap.
    //
    // REQUIRES spec-artifacts-process v0.4.0 — the feature bump that adds
    // `gap-analysis` to the SpecReview `analysis` enum (shipped alongside this skill).
    // Until v0.4.0 is tagged + pinned in default-modules.yaml, the seed reconciles the
    // published v0.3.0 module (enum lacks `gap-analysis`), so `quire validate` rejects
    // the doc and this eval is RED — the same release-coupling TC-EV-026 documents for the
    // SpecReview archetype itself. Proven GREEN locally (sonnet, 1/1) by temporarily
    // sourcing the module from the local working tree (`source.type: path`) before the
    // tag exists; the committed pin stays at the released v0.3.0.
    id: "TC-EV-030",
    useCase: "US-005",
    setup(ctx) {
      seedGapBundle(ctx, { taskDone: false, tc2: "none", untraced: "purge" });
    },
    prompt: gapPrompt(false),
    expect: {
      agentRan: GAP_AGENT_RAN,
      files: ["reviews/*.md"],
      artifacts: { require: { SpecReview: { min: 1, dir: "reviews" } } },
      validate: { globs: ["reviews/*.md"], shouldPass: true },
      // FAIL verdict; the incomplete task + unbacked matrix TC are named.
      fileContains: [
        {
          glob: "reviews/*.md",
          includes: [verdict("FAIL"), "TASK-002", "TC-002"],
        },
      ],
    },
  },
  {
    // TC-EV-031 — HAPPY path: every task done, both matrix TCs backed by real tagged
    // tests, no untraced code, semantic review declined → Verdict PASS with the single
    // "no gaps" finding. Proves the PASS branch + that a clean review still validates.
    // Shares the TC-EV-030 v0.4.0 release-coupling (RED in CI until v0.4.0 is pinned).
    id: "TC-EV-031",
    useCase: "US-005",
    setup(ctx) {
      seedGapBundle(ctx, { taskDone: true, tc2: "real", untraced: false });
    },
    prompt: gapPrompt(false),
    expect: {
      agentRan: GAP_AGENT_RAN,
      files: ["reviews/*.md"],
      artifacts: { require: { SpecReview: { min: 1, dir: "reviews" } } },
      validate: { globs: ["reviews/*.md"], shouldPass: true },
      // Clean plan → a non-blocking verdict (PASS or CONDITIONAL), never FAIL.
      // (A diligent agent may still note low-severity nuances, so PASS isn't
      // guaranteed; the happy signal is "no blocking gaps".)
      fileContains: [
        {
          glob: "reviews/*.md",
          includes: [verdict("(PASS|CONDITIONAL)")],
          excludes: [verdict("FAIL")],
        },
      ],
    },
  },
  {
    // TC-EV-032 — SAD (medium-only) → Verdict CONDITIONAL: plan done and matrix fully
    // backed, but the source has an untraced read-only `listCodes` API with no owning
    // requirement (Step 4). Isolates the reverse-gap path + the CONDITIONAL gate.
    id: "TC-EV-032",
    useCase: "US-005",
    setup(ctx) {
      seedGapBundle(ctx, {
        taskDone: true,
        tc2: "real",
        untraced: "listCodes",
      });
    },
    prompt: gapPrompt(false),
    expect: {
      agentRan: GAP_AGENT_RAN,
      files: ["reviews/*.md"],
      artifacts: { require: { SpecReview: { min: 1, dir: "reviews" } } },
      validate: { globs: ["reviews/*.md"], shouldPass: true },
      // The untraced listCodes is flagged (medium); verdict is not a blocking FAIL.
      fileContains: [
        {
          glob: "reviews/*.md",
          includes: ["listCodes"],
          excludes: [verdict("FAIL")],
        },
      ],
    },
  },
  {
    // TC-EV-033 — OPTIONAL semantic review (Step 5): plan done, matrix TC-002 is backed
    // by a tagged test so Step 3 passes — but that test is HOLLOW (asserts nothing
    // about resolve). Only the semantic review (which the prompt opts into) catches it.
    // The TC-002 finding is the signal that Step 5 actually ran (Steps 2-4 are clean).
    id: "TC-EV-033",
    useCase: "US-005",
    setup(ctx) {
      seedGapBundle(ctx, { taskDone: true, tc2: "hollow", untraced: false });
    },
    prompt: gapPrompt(true),
    expect: {
      agentRan: GAP_AGENT_RAN,
      files: ["reviews/*.md"],
      artifacts: { require: { SpecReview: { min: 1, dir: "reviews" } } },
      validate: { globs: ["reviews/*.md"], shouldPass: true },
      // Semantic review surfaces the hollow TC-002 test; the review is not clean.
      fileContains: [
        {
          glob: "reviews/*.md",
          includes: [
            "TC-002",
            "(hollow|assert|exercise|does not test|resolve)",
          ],
          excludes: ["No gaps found"],
        },
      ],
    },
  },
  {
    // EARS requirement-grammar (quire-rs FR-042): author EARS-clean from the
    // start. The `--strict` validate asserts the authored FR is BOTH
    // structurally valid AND grammar-clean — exercising the EARS skeleton
    // guidance + `/specify`.
    id: "TC-EV-040",
    useCase: "US-001",
    prompt:
      "Author one Functional Requirement at spec/functional/FR-100.md for this " +
      "feature: 'the gateway streams agent responses to connected clients and " +
      "retransmits unacknowledged frames'. Write each requirement statement to " +
      "follow EARS — one `shall` per statement, a named subject, a concrete " +
      "response verb (not support/handle/manage/provide), and a canonical " +
      "`When …` / `While …` / `If … then …` / `Where …` trigger when " +
      "conditional. Then run " +
      '`quire validate --scope . "spec/**/*.md" --summary` and revise until the ' +
      "summary reports the document grammar-clean.",
    expect: {
      files: ["spec/functional/FR-100.md"],
      validate: {
        globs: ["spec/functional/FR-100.md"],
        strict: true,
        shouldPass: true,
      },
    },
  },
  {
    // EARS repair loop (mirrors TC-EV-008 but for the grammar check): a
    // structurally-valid FR with three EARS defects (non-singular + vague
    // response + non-canonical trigger). The agent reads the `[ears:…]`
    // warnings and rewrites the Description until `--strict` passes.
    id: "TC-EV-041",
    useCase: "US-004",
    prompt:
      "The requirement statement in spec/functional/FR-001.md trips the EARS " +
      "requirement-grammar check. Run " +
      '`quire validate --scope . "spec/functional/FR-001.md" --summary`, read the ' +
      "`[ears:…]` warnings, and rewrite the Description so every statement is " +
      "EARS-clean — one `shall`, a named subject, a concrete response, and a " +
      "canonical trigger instead of `On …`. Re-run until the summary reports it " +
      "grammar-clean. Do not change the Acceptance Criteria or Dependencies.",
    setup(ctx) {
      writeRepoFile(
        ctx,
        "spec/functional/FR-001.md",
        [
          "---",
          "id: FR-001",
          'title: "Stream agent responses to the client"',
          "type: FR",
          "---",
          "# [FR-001] Stream agent responses to the client",
          "",
          "## Description",
          "",
          "On connection, the gateway shall support streaming responses and shall also",
          "buffer partial frames until the client acknowledges them.",
          "",
          "## Acceptance Criteria",
          "",
          "| ID | Criteria | Verification |",
          "|----|----------|--------------|",
          "| FR-001-AC-1 | A connected client receives streamed frames in order | Test |",
          "| FR-001-AC-2 | Unacknowledged frames are retransmitted on timeout | Test |",
          "",
          "## Dependencies",
          "",
          "- **Upstream**: none",
          "- **Downstream**: none",
        ].join("\n"),
      );
    },
    expect: {
      files: ["spec/functional/FR-001.md"],
      validate: {
        globs: ["spec/functional/FR-001.md"],
        strict: true,
        shouldPass: true,
      },
    },
  },
  {
    // FR-044 project Ubiquitous-Language HAPPY path. The agent authors an FR
    // using a project-specific term AND defines that term in a `domain` object's
    // `## Ubiquitous Language`; `--strict` passes only because the harvested
    // term suppresses the otherwise-vague `provide a <term>` (validates the
    // harvest+inject chain end-to-end through a live agent). Uses the DDD UL
    // form (a released archetype) so the eval needs only the FR-044 engine.
    id: "TC-EV-042",
    useCase: "US-001",
    prompt:
      "Author one Functional Requirement at spec/functional/FR-100.md whose " +
      "Description says exactly: 'The gateway shall provide a Sprocket to each " +
      "connected client.' 'Sprocket' is a term specific to THIS project, so the " +
      "EARS grammar check reads `provide a Sprocket` as a vague response unless " +
      "the project defines 'Sprocket'. Define it: author a `domain` object at " +
      "spec/objects/gateway-domain.md (use `quoin write --types domain` for the " +
      "skeleton) with a `## Bounded Context` section and a `## Ubiquitous " +
      "Language` section containing the bullet `- **Sprocket** — <a " +
      'definition>`. Then run `quire validate --scope . "spec/**/*.md" ' +
      "--summary` and confirm the requirement is grammar-clean.",
    expect: {
      files: ["spec/functional/FR-100.md", "spec/objects/gateway-domain.md"],
      validate: { globs: ["spec/**/*.md"], strict: true, shouldPass: true },
    },
  },
  {
    // FR-044 project Ubiquitous-Language SAD/repair path: a seeded FR trips the
    // grammar check on a project term. The correct fix is to DEFINE the term in
    // a `domain` object's `## Ubiquitous Language` — NOT to reword the
    // requirement. After the agent adds the definition, `--strict` passes.
    id: "TC-EV-043",
    useCase: "US-004",
    prompt:
      "spec/functional/FR-001.md trips the EARS requirement-grammar check: " +
      '`quire validate --scope . "spec/functional/FR-001.md" --summary` flags ' +
      "'Sprocket' as a vague response. 'Sprocket' is a real project term, NOT " +
      "vague. Resolve the finding by DEFINING 'Sprocket' for the project — " +
      "author a `domain` object at spec/objects/gateway-domain.md (use `quoin " +
      "write --types domain`) with a `## Bounded Context` section and a `## " +
      "Ubiquitous Language` section containing `- **Sprocket** — <a " +
      "definition>`. Do NOT change FR-001's wording. Re-run until the summary " +
      "reports it grammar-clean.",
    setup(ctx) {
      writeRepoFile(
        ctx,
        "spec/functional/FR-001.md",
        [
          "---",
          "id: FR-001",
          'title: "Deliver a Sprocket"',
          "type: FR",
          "---",
          "# [FR-001] Deliver a Sprocket",
          "",
          "## Description",
          "",
          "The gateway shall provide a Sprocket to each connected client.",
          "",
          "## Acceptance Criteria",
          "",
          "| ID | Criteria | Verification |",
          "|----|----------|--------------|",
          "| FR-001-AC-1 | A connected client receives a Sprocket | Test |",
          "",
          "## Dependencies",
          "",
          "- **Upstream**: none",
          "- **Downstream**: none",
        ].join("\n"),
      );
    },
    expect: {
      files: ["spec/objects/gateway-domain.md"],
      validate: { globs: ["spec/**/*.md"], strict: true, shouldPass: true },
    },
  },

  // --- spec-correctness (TC-EV-050..TC-EV-053, FR-028) ------------------------------
  ...specCorrectnessScenarios(),
  // --- spec-fuzz (TC-EV-054..TC-EV-057, FR-038) -------------------------------------
  ...specFuzzScenarios(),
];

export const CANARY_IDS = ["TC-EV-001", "TC-EV-008"];

export function selectScenarios({ canary, all, filter }) {
  if (filter) {
    const wanted = filter.split(",").map((f) => f.trim());
    return SCENARIOS.filter((s) => wanted.some((f) => s.id.includes(f)));
  }
  if (all) return SCENARIOS;
  if (canary) return SCENARIOS.filter((s) => CANARY_IDS.includes(s.id));
  return [];
}
