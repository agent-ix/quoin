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
        location: "spec/tests.md:8",
        row_id: "TC-001",
        findable: true,
        expect_reason: "assert",
        note:
          "`Demonstration` is not in this module's declared test_type vocabulary. " +
          "The first draft of this label was WRONG: the module declared the " +
          "vocabulary but no `column_choices` assert, so nothing could fire. " +
          "Verifying the labels is what caught it.",
        confirmed_at: "quire-rs v0.43.0",
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
