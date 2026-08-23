// Tier-1 seeded-defect corpora (agent-ix/quoin#199, FR-043-AC-7).
//
// Every defect here is placed deliberately and enumerated in `labels.json`, so
// precision and recall are computable rather than estimated. That is the whole
// point: tier 2 (`filament-ide-rs` at a pinned SHA) tells you whether the tools
// find what humans found in the wild; tier 1 tells you whether they find a
// defect you KNOW is there, cheaply enough to run every time.
//
// Tier 1 alone overfits — a seeded defect is one somebody already knew how to
// describe — which is why FR-043 requires both.
//
// Built with the existing `evals/lib/fixtures.mjs` helpers rather than a new
// generator, per #199.

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

/** The module every mini-repo is validated against. */
const MODULE = `name: bench
manifest_version: 1.0.0
version: 0.0.1
artifact_types:
- name: FR
  grammar_ref: iso-spec-core
- name: TestMatrix
  body_extraction:
    yield_pattern:
      match:
        test_cases:
          from: table_row
          under_section: Test Cases
          required: true
          multiple: true
          assert:
            id_column: ID
            column_choices:
              Type: [Unit, Integration, Inspection]
traceability:
  trace_targets:
  - name: test-case
    archetype: TestMatrix
    section: Test Cases
    id_column: ID
  - name: acceptance-criterion
    archetype: FR
    section: Acceptance Criteria
    id_column: ID
  document_references:
  - name: traces-to
    archetype: TestMatrix
    section: Test Cases
    row_id_column: ID
    column: Traces To
    targets: [acceptance-criterion, test-case]
    pattern: '([A-Z]{2,4}-\\d+(?:-[A-Z]{2,4}-\\d+)?)'
  status:
    column: Status
    complete: ["✅"]
    pending: ["\u{1F6A7}"]
  vocabularies:
    test_type_column: Type
    test_type: [Unit, Integration, Inspection]
  trace_tags:
    markers:
    - name: rust-trace-attribute
      language: rust
      pattern: '#\\[trace\\(([^)]*)\\)\\]'
      template: '#[trace({ids})]'
`;

const fr = (criteria) =>
  `---\nid: FR-001\ntype: FR\n---\n\n## Acceptance Criteria\n\n` +
  `| ID | Criteria | Verification |\n|----|----------|--------------|\n` +
  criteria
    .map((c, i) => `| FR-001-AC-${i + 1} | ${c} | Test (TC-00${i + 1}) |\n`)
    .join("");

const matrix = (rows) =>
  `---\nid: TM-001\ntype: TestMatrix\n---\n\n## Test Cases\n\n` +
  `| ID | Traces To | Type | Status |\n|----|-----------|------|--------|\n` +
  rows.map((r) => `| ${r} |\n`).join("");

/**
 * The corpora. Each is one defect FAMILY in isolation — a mini-repo mixing
 * three defects cannot tell you which one a finding was about, and precision
 * per family is what FR-043-AC-2 asks for.
 */
export const CORPORA = [
  {
    name: "marker-mismatch",
    family: "marker-form-mismatch",
    summary:
      "Real tests, real tags, a marker spelling the module never declared. " +
      "1,292 such symbols in filament-ide-rs bound nothing and the report said 23%.",
    files: {
      "module/manifest.yaml": MODULE,
      "spec/FR-001.md": fr(["Every finding shall default to warning."]),
      "spec/tests.md": matrix(["TC-001 | FR-001-AC-1 | Unit | \u{1F6A7}"]),
      "src/lib.rs":
        '//! seeded\n\n#[cfg(test)]\nmod tests {\n    #[tracks("TC-001")]\n' +
        "    #[test]\n    fn covers() {\n        assert_eq!(1, 1);\n    }\n}\n",
    },
    defects: [
      {
        id: "MM-1",
        family: "marker-form-mismatch",
        location: "src/lib.rs:5",
        row_id: "TC-001",
        findable: true,
        expect_reason: "no-symbol-bound",
        note: "the module declares `#[trace(...)]`; the source writes `#[tracks(...)]`",
        confirmed_at: "quire-rs v0.43.0",
      },
    ],
  },
  {
    name: "wrong-type-cell",
    family: "undeclared-type-value",
    summary:
      "A `Type` cell naming a verification method the vocabulary has no word " +
      "for. 128 such rows in filament-ide-rs, and the `Inspection` ones were " +
      "reported as status lies because no declared type said what they are.",
    files: {
      "module/manifest.yaml": MODULE,
      "spec/FR-001.md": fr(["Every finding shall default to warning."]),
      "spec/tests.md": matrix(["TC-001 | FR-001-AC-1 | Demonstration | ✅"]),
      "src/lib.rs":
        '//! seeded\n\n#[cfg(test)]\nmod tests {\n    #[trace("TC-001")]\n' +
        "    #[test]\n    fn covers() {\n        assert_eq!(1, 1);\n    }\n}\n",
    },
    defects: [
      {
        id: "WT-1",
        family: "undeclared-type-value",
        // Line 10 is the row carrying `Demonstration`. Line 8 is the table
        // HEADER, which is where this label pointed until #199 rendered the
        // fixture and counted: `---`, `id`, `type`, `---`, blank, heading,
        // blank, header, separator, row.
        location: "spec/tests.md:10",
        row_id: "TC-001",
        findable: true,
        expect_reason: "assert",
        note:
          "`Demonstration` is not in this module's declared test_type vocabulary. " +
          "The first draft of this label was WRONG: the module declared the " +
          "vocabulary but no `column_choices` assert, so nothing could fire. " +
          "Verifying the labels is what caught it. The SECOND draft was wrong " +
          "too, at `spec/tests.md:8` — the table header, not the row. Both " +
          "errors were found by rendering the corpus and reading it, which is " +
          "the only way a label is ground truth rather than a claim.",
        // The engine reported line 9 — the `|---|---|` separator — until
        // agent-ix/quire-rs#254. `coverage` reported 10 for the same row the
        // whole time; two paths disagreed. This label states where the defect
        // IS, so it was a miss plus a false positive against every engine
        // before that fix, which is the honest reading of "detected but
        // cannot say where".
        confirmed_at: "quire-rs main (post v0.44.2, agent-ix/quire-rs#254)",
        needs_engine: "> v0.44.2",
      },
    ],
  },
  {
    name: "catch-all-properties",
    family: "catch-all-universal",
    summary:
      "Every extractable criterion is the `universal` catch-all, so the " +
      "extractable headline reads far higher than the figure for 'the tool " +
      "told me what property to write' — 54% against 8% in pass 2.",
    files: {
      "module/manifest.yaml": MODULE,
      "spec/FR-001.md": fr([
        "Every request shall carry a trace id.",
        "Every response shall carry a status.",
      ]),
      "spec/tests.md": matrix([
        "TC-001 | FR-001-AC-1 | Unit | \u{1F6A7}",
        "TC-002 | FR-001-AC-2 | Unit | \u{1F6A7}",
      ]),
      "src/lib.rs": "//! seeded, no tests\n",
    },
    defects: [
      {
        id: "CA-1",
        family: "catch-all-universal",
        location: "spec/FR-001.md",
        row_id: null,
        findable: true,
        expect_metric: "coverage.specific_shaped",
        expect_value: 0,
        note: "specific_shaped must be 0 while property_shaped is not",
        confirmed_at: "quire-rs v0.43.0",
      },
    ],
  },
  {
    name: "vacuous-property-suite",
    family: "vacuous-under-guard",
    summary:
      "Every assertion sits behind a narrowing guard, so an input that does " +
      "not enter it passes unchecked. TC-1596 was green while checking 2.3% " +
      "of its samples.",
    files: {
      "module/manifest.yaml": MODULE,
      "spec/FR-001.md": fr(["Every finding shall default to warning."]),
      "spec/tests.md": matrix(["TC-001 | FR-001-AC-1 | Unit | ✅"]),
      "src/lib.rs":
        '//! seeded\n\n#[cfg(test)]\nmod tests {\n    #[trace("TC-001")]\n' +
        "    #[test]\n    fn covers() {\n" +
        "        if let Some(v) = parse() {\n            assert_eq!(v, 1);\n        }\n" +
        "    }\n}\n",
    },
    defects: [
      {
        id: "VP-1",
        family: "vacuous-under-guard",
        location: "src/lib.rs:7",
        row_id: "TC-001",
        findable: true,
        expect_suspicion: "vacuous-under-guard",
        note: "the only assertion is inside an `if let`",
        // Confirmed against the detector directly; the CLI surface needs an
        // engine newer than v0.43.0, which is when `suspicions` landed.
        confirmed_at: "quire-rs main (post v0.43.0)",
        needs_engine: "> v0.43.0",
      },
    ],
  },
  {
    name: "hollow-metric",
    family: "hollow-denominator",
    summary:
      "A ratio published over a population the measurement could not read. " +
      "`coverage` reported 23% over filament-ide-rs while binding 0 of 1,292 " +
      "evidence symbols, and three SpecReviews cite figures computed under it.",
    files: {
      "module/manifest.yaml": MODULE,
      "spec/FR-001.md": fr(["Every finding shall default to warning."]),
      "spec/tests.md": matrix(["TC-001 | FR-001-AC-1 | Unit | \u{1F6A7}"]),
      // A real test, no marker at all — the binder walks one candidate and
      // binds none, so `coverage.backed` is a ratio over a corpus it could
      // not read.
      "src/lib.rs":
        "//! seeded\n\n#[cfg(test)]\nmod tests {\n    #[test]\n" +
        "    fn covers() {\n        assert_eq!(parse(), 1);\n    }\n}\n",
    },
    defects: [
      {
        id: "HD-1",
        family: "hollow-denominator",
        location: "src/lib.rs",
        row_id: null,
        findable: true,
        expect_reason: "hollow-denominator",
        note:
          "One evidence symbol examined, none bound, so `coverage.backed` " +
          "reports a ratio over nothing. The diagnostic itself carries NO " +
          "path — it names the metric, not a place — so this label pairs on " +
          "family alone and correctly drags `finding_localisation_rate` down: " +
          "a number that cannot say where is exactly the thing being measured.",
        confirmed_at: "quire-rs v0.44.2",
        // `no-symbol-bound` necessarily co-fires, and it is CORRECT: the same
        // tree is both a hollow denominator and an unread marker. Measured
        // rather than assumed — an untagged source and a misspelled marker
        // produce the identical pair of diagnostics and the identical census,
        // so the engine cannot separate the two causes at all.
        //
        // `coverage.backed` is the only ratio metric that CAN go hollow:
        // `property_shaped` and `specific_shaped` set examined == matched ==
        // criteria (quire-rs `coverage.rs:560-600`), so `is_hollow`'s
        // `matched: 0 if examined > 0` is unsatisfiable for them. Which means
        // there is no corpus that isolates this family, and declaring the
        // collateral is the honest alternative to scoring a correct finding
        // as a false positive.
        collateral: [
          {
            family: "marker-form-mismatch",
            reason: "no-symbol-bound",
            note:
              "the same unbound symbol seen through the binder's own lens; " +
              "`hollow-denominator` is the generalization of it, so a corpus " +
              "that makes one fire makes both fire",
          },
        ],
      },
    ],
  },
  {
    name: "oracle-copy",
    family: "oracle-is-code-copy",
    summary:
      "A test oracle that recomputes the implementation instead of judging " +
      "it. Pass 2's highest-value manual finding: TC-1598's oracle was a " +
      "character-for-character copy, redundant branch included, and replacing " +
      "it exposed a real Windows containment gap.",
    files: {
      "module/manifest.yaml": MODULE,
      "spec/FR-001.md": fr([
        "Every path shall be relative and free of dot segments.",
      ]),
      "spec/tests.md": matrix(["TC-001 | FR-001-AC-1 | Unit | ✅"]),
      // The oracle restates the implementation's own rules, token for token.
      // A copy computes the same answer the same way, so it asserts only that
      // the code equals itself, and it cannot fail.
      "src/lib.rs":
        "//! seeded\n\n" +
        "pub fn is_contained(path: &str) -> bool {\n" +
        "    !path.starts_with('/') && !path.contains(\"..\") && !path.contains('\\0')\n" +
        "}\n\n" +
        "#[cfg(test)]\nmod tests {\n    use super::*;\n\n" +
        '    #[trace("TC-001")]\n' +
        "    #[test]\n    fn covers() {\n" +
        '        let path = "a/b";\n' +
        "        let expected =\n" +
        "            !path.starts_with('/') && !path.contains(\"..\") && !path.contains('\\0');\n" +
        "        assert_eq!(is_contained(path), expected);\n" +
        "    }\n}\n",
    },
    defects: [
      {
        id: "OC-1",
        family: "oracle-is-code-copy",
        location: "src/lib.rs:13",
        row_id: "TC-001",
        findable: true,
        expect_suspicion: "oracle-resembles-implementation",
        note:
          "The oracle is the implementation's boolean expression, copied. " +
          "NOT YET DETECTED END TO END: `skeptic::oracle_copies` ships and its " +
          "similarity comparison is tested, but it has NO production caller — " +
          "`grep -rn oracle_copies src/` in quire-rs returns the definition " +
          "and two unit tests. The join from an oracle span to the " +
          "implementation it judges needs the `Registry` and the `implements` " +
          "relation, which the binder does not hold (agent-ix/quire-rs#236). " +
          "Recall for this family reads 0 until that lands, and that 0 is a " +
          "measurement of a real gap rather than a fixture problem.",
        confirmed_at: "not detected at quire-rs v0.44.2 — detector unwired",
        needs_engine: "agent-ix/quire-rs#236",
      },
    ],
  },
  {
    name: "mocked-confirmation",
    family: "mocked-confirmation",
    summary:
      "The behaviour a criterion verifies, substituted by the test that " +
      "verifies it. Pass 2's case: FR-017-AC-7's trusted-UI confirmation had " +
      "no implementation and its test passed by injecting `Confirmation::allow()`.",
    files: {
      "module/manifest.yaml": MODULE,
      "spec/FR-001.md": fr([
        "The shell shall obtain the user's confirmation before granting a root.",
      ]),
      "spec/tests.md": matrix(["TC-001 | FR-001-AC-1 | Unit | ✅"]),
      // The only binding for the criterion injects a stand-in for the exact
      // behaviour the criterion is about. Green test, green AC, absent
      // behaviour — there is no `confirm` in production at all.
      "src/lib.rs":
        "//! seeded\n\n" +
        "pub struct Confirmation;\n\n" +
        "impl Confirmation {\n" +
        "    pub fn allow() -> Self {\n        Self\n    }\n}\n\n" +
        "pub fn grant_root(_c: Confirmation) -> bool {\n    true\n}\n\n" +
        "#[cfg(test)]\nmod tests {\n    use super::*;\n\n" +
        '    #[trace("TC-001")]\n' +
        "    #[test]\n    fn covers() {\n" +
        "        assert!(grant_root(Confirmation::allow()));\n" +
        "    }\n}\n",
    },
    defects: [
      {
        id: "MC-1",
        family: "mocked-confirmation",
        // A bare path, no line. The auditor's `mocked-confirmation` finding
        // carries `{kind, obligation, severity, summary}` and NO locus at all
        // (`src/auditor/audit.ts:241`), so a label pinned to a line would be a
        // claim about output the detector does not produce. Stated this way it
        // pairs on family and correctly costs `finding_localisation_rate`,
        // which is the true reading: the check names an obligation, not a
        // place to look.
        location: "src/lib.rs",
        row_id: "TC-001",
        findable: true,
        expect_reason: "mocked-confirmation",
        note:
          "The criterion's only binding injects `Confirmation::allow()` — the " +
          "behaviour under verification — so the test passes whether or not " +
          "that behaviour exists. NOT YET DETECTED: quoin's detector ships at " +
          "`src/auditor/audit.ts:241` and `mockedBindings` returns [] on every " +
          "production call, because `AuditInput.injections` is optional and no " +
          "caller supplies it and nothing in the tree produces a " +
          "`MockInjection` (agent-ix/quoin#204, reopened). Scoring this family " +
          "additionally needs the runner to call `quoin evidence audit` — " +
          "`quire coverage` and `quire properties` cannot see it — and an " +
          "evidence store this corpus deliberately does NOT carry, because a " +
          "hand-written `statementHashAtBinding` would fabricate the one field " +
          "the store's suspect-link check exists to compare.",
        confirmed_at: "not detected at quoin 0.21.8 — detector has no producer",
        needs_engine: "agent-ix/quoin#204",
      },
    ],
  },
  {
    name: "gate-that-gates-nothing",
    family: "gate-that-gates-nothing",
    summary:
      "A gate that is wired, green, and asserts no condition that could " +
      "fail. Observed across filament-ide-rs's own CI during pass 2, found by " +
      "a human, and detected by nothing anywhere in the ecosystem.",
    files: {
      "module/manifest.yaml": MODULE,
      "spec/FR-001.md": fr(["No production symbol shall call `unwrap`."]),
      "spec/tests.md": matrix(["TC-001 | FR-001-AC-1 | Inspection | ✅"]),
      // The gate the matrix row claims verifies FR-001-AC-1. It greps, it
      // exits 0, and it exits 0 whatever the grep finds: the pipeline's status
      // is `wc`'s, which is 0 for any count including a non-zero one. It
      // cannot fail, so its green says nothing about the criterion.
      "scripts/check_unwrap.sh":
        "#!/usr/bin/env bash\n" +
        "# Gate for FR-001-AC-1: no production `unwrap`.\n" +
        "set -euo pipefail\n" +
        'grep -rn "unwrap()" src/ | wc -l\n' +
        "exit 0\n",
      "src/lib.rs":
        "//! seeded\n\npub fn parse(text: &str) -> u32 {\n" +
        "    text.parse().unwrap()\n}\n",
    },
    defects: [
      {
        id: "GN-1",
        family: "gate-that-gates-nothing",
        location: "scripts/check_unwrap.sh:5",
        row_id: "TC-001",
        findable: true,
        // The reason a detector MUST emit, declared before the detector
        // exists. The dictionary is the contract, and a family whose expected
        // signal is only in somebody's head cannot be built against.
        expect_reason: "gate-that-gates-nothing",
        note:
          "`exit 0` follows unconditionally, so the gate is green for any " +
          "grep result — and `src/lib.rs` carries the `unwrap()` it claims to " +
          "forbid, so the corpus proves the gate is not merely tautological " +
          "but actively wrong. NO DETECTOR EXISTS anywhere in the ecosystem " +
          "(agent-ix/quoin#197 workstream 4), so recall reads 0 and that 0 is " +
          "the measurement. This is also the shape quoin#204 turned out to " +
          "have: a check that runs, evaluates one branch and returns empty " +
          "every time is indistinguishable from a clean result.",
        confirmed_at:
          "not detected at quire-rs v0.44.2 / quoin 0.21.8 — no detector",
        needs_engine: "a detector that does not yet exist",
      },
    ],
  },
  {
    name: "clean-control",
    family: "none",
    summary:
      "No seeded defect. The control every corpus needs and most lack: a " +
      "check that cannot stay silent on healthy input is not a check, it is " +
      "a constant.",
    files: {
      "module/manifest.yaml": MODULE,
      "spec/FR-001.md": fr(["Every finding shall default to warning."]),
      "spec/tests.md": matrix(["TC-001 | FR-001-AC-1 | Unit | ✅"]),
      "src/lib.rs":
        '//! seeded\n\n#[cfg(test)]\nmod tests {\n    #[trace("TC-001")]\n' +
        "    #[test]\n    fn covers() {\n        assert_eq!(parse(), 1);\n    }\n}\n",
    },
    defects: [],
  },
];

/** Materialize every corpus under `root`, and write the label file. */
export function buildBenchCorpora(root) {
  const labels = { corpora: [] };
  for (const corpus of CORPORA) {
    for (const [rel, body] of Object.entries(corpus.files)) {
      const target = join(root, corpus.name, rel);
      mkdirSync(dirname(target), { recursive: true });
      writeFileSync(target, body);
    }
    labels.corpora.push({
      name: corpus.name,
      family: corpus.family,
      summary: corpus.summary,
      defects: corpus.defects,
    });
  }
  writeFileSync(
    join(root, "labels.json"),
    JSON.stringify(labels, null, 2) + "\n",
  );
  return labels;
}
