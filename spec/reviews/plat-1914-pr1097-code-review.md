---
id: SR-967175026302197768900474181941935084889
title: "code-review — CycloneDX SOUP VEX export"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1914; PR #1097; soup exporter, advisory/report paths, integration tests, workflow, how-to, FR-156"
review_set: subset
---

## Summary

Reviewed the exact GitHub head 927028c04409c121b27909ac6aadbbb4b932e83b. Worktree edits by the author after this snapshot are not findings against this head. Explicit authoring/matrix tool: /tmp/regfinish-quoin-v0.37.0/quoin 0.37.0; module spec-artifacts-process@v0.36.0; Quire validator 0.52.0 (engine 0.62.2). Exact model ID is not exposed (model=unknown). No product source was edited by the reviewer.


## Verdict

**FAIL** — findings below are the initial review at the recorded head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Two identical valid authored statements in one record are emitted as identical vulnerability entries. Export exits 0, but the published CycloneDX 1.7 schema requires vulnerabilities.uniqueItems=true and rejects the output. Reproduced with two equal affected statements for pkg:npm/example@1.0.0; independent Draft7 validation reports uniqueItems. Preserve one entry per statement while refusing an unrepresentable duplicate rather than reporting success. | rust/crates/quoin-soup/src/soup/cyclonedx.rs:235 |
| FND-002 | high | The new exporter duplicates the existing OpenVEX source_statement/source_invalid parsing helpers and atomic-write implementation (cyclonedx.rs:462 versus vex.rs:317). These are the same source-field admission and persistence rules with format-specific labels, creating two places to repair them. Extract one shared source parser and one shared staged writer; keep only format-specific mapping in the exporters. The code-review no-duplication rule requires a single owner. | rust/crates/quoin-soup/src/soup/cyclonedx.rs:297 |
| FND-003 | medium | The determinism test writes first.json and second.json, while AC-9 explicitly requires repeated export at the same path. An implementation that appends when the output exists would pass this test. Export twice to the same file and compare the complete bytes before and after overwrite. | rust/crates/quoin-cli/tests/tc_1914_cyclonedx_vex_export.rs:506 |
| FND-004 | medium | The delegated packageurl 0.7.1 PyPI normalizer only lowercases and replaces individual underscores. Current PURL rules collapse each run of dash, underscore or dot to one dash. Reproduced source pkg:pypi/Foo..__Bar@1.0.0 exports component name foo..--bar instead of foo-bar. Use a compliant normalizer and add mixed-separator coverage. | rust/crates/quoin-soup/src/soup/cyclonedx.rs:282 |
| FND-005 | medium | A valid source record/PURL version longer than 1024 characters is exported successfully, but the published CycloneDX affected-version schema has maxLength 1024. Reproduced a 1025-character version: export exits 0 and independent Draft7 validation rejects affects[0].versions[0].version. Refuse the unrepresentable value before output replacement and cover 1024/1025 boundaries. | rust/crates/quoin-soup/src/soup/cyclonedx.rs:382 |

## Assurance Context

AP-201 (`spec/assurance/AP-201-finding-quality.md`) and AP-202 (`spec/assurance/AP-202-jev-lens-evaluation.md`) were inspected. AP-201 governs finding-quality benchmark/measurement comparison and report populations; this SOUP export changes authored vulnerability projection and advisory metadata, not that benchmark pipeline. AP-202 governs live Jev lens acceptance and is unrelated. Neither selects an enforced review run for this change (both recommend). No profile-specific selected architecture or measurement evidence is applicable; active exceptions: none identified.

## Examined units and bindings

```yaml
scope:
- id: FR-156-CON-1
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: The export SHALL conform to the published CycloneDX 1.7 JSON schema.
- id: FR-156-CON-2
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: The repository SHALL NOT contain a downloaded schema copy.
- id: FR-156-CON-3
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: The export SHALL NOT infer a VEX status or create an entry without a stored
    person-authored statement.
- id: FR-156-AC-1
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: A fixture with `pkg:npm/%40babel/core@12.3.1` exports a component with
    `type=library`, `group=@babel`, and `name=core`; a fixture with `pkg:pypi/requests@2.31.0`
    exports a component with `type=library`, no `group`, and `name=requests`. Each
    component has its exact canonical PURL in both `purl` and `bom-ref`, and each
    vulnerability entry references that component.
- id: FR-156-AC-2
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: A fixture containing one statement for each of the four statuses exports
    exactly four vulnerability entries, each carrying the expected CycloneDX 1.7 `analysis.state`,
    affected-version status and source status property.
- id: FR-156-AC-3
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: For every exported entry, `advisory`, assessor, assessment date, impact
    statement, optional action statement and each evidence reference equal the source
    statement; each statement in a record produces exactly one entry, and the same
    advisory id on two records produces two entries.
- id: FR-156-AC-4
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: Statements with all five allowed justifications export the exact source
    value; `vulnerable_code_not_present`, `vulnerable_code_not_in_execute_path` and
    `inline_mitigations_already_exist` map to their equivalent CycloneDX justifications,
    while `component_not_present` and `vulnerable_code_cannot_be_controlled_by_adversary`
    preserve the source value and omit `analysis.justification`.
- id: FR-156-AC-5
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: The output validates against the published CycloneDX 1.7 JSON schema fetched
    as a CI input; the schema is absent from the repository.
- id: FR-156-AC-6
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: With no `vex_statement`, the output contains no `vulnerabilities[]` entry,
    including when the scan reports advisories; every output entry with an advisory
    has exactly one matching stored person-authored statement.
- id: FR-156-AC-7
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: A statement absent from all current applicable scans is exported with `quoin:vex:stale=true`;
    an unavailable, unordered or vacuous applicable scan does not produce that property.
- id: FR-156-AC-8
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: For a case-sensitive PURL type, `components[].group` (when namespace exists)
    and `components[].name` preserve the decoded namespace and name case; for a PURL
    type that normalizes either component, those fields carry the PURL-defined normalized
    values without additional case folding or normalization.
- id: FR-156-AC-9
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: Repeating export with identical records, SBOM and scan inputs yields byte-identical
    JSON at the same output path.
- id: FR-156-AC-10
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: '`--format cyclonedx` selects CycloneDX without OpenVEX metadata flags;
    omitting `--out` requires `--sbom` and writes `<sbom-stem>.vex.cdx.json` beside
    the SBOM. The default `--format openvex` still requires its OpenVEX metadata and
    writes OpenVEX.'
- id: FR-156:changed-behavior-1
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: The command SHALL select CycloneDX output with `--format cyclonedx`; the
    existing default `--format openvex` remains the OpenVEX mode defined by FR-216.
- id: FR-156:changed-behavior-2
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: When CycloneDX output is selected and `--out` is omitted, `--sbom` SHALL
    be required and the output SHALL be placed beside that path. For `probe.cdx.json`,
    the default output SHALL be `probe.vex.cdx.json`.
bindings:
- test_id: purl_components_follow_type_definitions_and_bom_matches_schema
  ac_id: FR-156-AC-1
  trace: correct
- test_id: purl_components_follow_type_definitions_and_bom_matches_schema
  ac_id: FR-156-AC-8
  trace: correct
- test_id: purl_components_follow_type_definitions_and_bom_matches_schema
  ac_id: FR-156-AC-5
  trace: correct
- test_id: purl_components_follow_type_definitions_and_bom_matches_schema
  ac_id: FR-156-CON-1
  trace: correct
- test_id: four_statuses_map_to_cyclonedx_analysis_and_version_status
  ac_id: FR-156-AC-2
  trace: correct
- test_id: authored_fields_and_all_justifications_are_preserved_without_advisory_deduplication_across_records
  ac_id: FR-156-AC-3
  trace: correct
- test_id: authored_fields_and_all_justifications_are_preserved_without_advisory_deduplication_across_records
  ac_id: FR-156-AC-4
  trace: correct
- test_id: authored_fields_and_all_justifications_are_preserved_without_advisory_deduplication_across_records
  ac_id: FR-156-AC-10
  trace: correct
- test_id: scanner_advisories_do_not_create_vulnerabilities_without_authored_statements
  ac_id: FR-156-AC-6
  trace: correct
- test_id: scanner_advisories_do_not_create_vulnerabilities_without_authored_statements
  ac_id: NFR-030-AC-6
  trace: correct
- test_id: only_a_current_nonvacuous_clean_applicable_scan_marks_a_statement_stale
  ac_id: FR-156-AC-7
  trace: correct
- test_id: identical_inputs_produce_byte_identical_output
  ac_id: FR-156-AC-9
  trace: correct
- test_id: omitted_output_path_uses_the_sbom_directory_and_vex_suffix
  ac_id: FR-156-AC-5
  trace: correct
- test_id: omitted_output_path_uses_the_sbom_directory_and_vex_suffix
  ac_id: FR-156-AC-10
  trace: correct
- test_id: cyclonedx_without_out_requires_an_sbom_path
  ac_id: FR-156-AC-10
  trace: correct
```

## Coverage and evidence

Independent focused pre-fix build evidence: CycloneDX integration binary 8/8 passed; existing OpenVEX integration binary 6/6 passed; soup library unordered-applicable-scan filter 1/1 passed. Existing binaries reside in the explicit worktree rust/target/debug tree and embed this worktree's CARGO manifest/binary paths. Schema fixtures use /tmp/quoin-cyclonedx-bom-1.7.schema.json plus referenced schemas and the current security-module fixture. Both concrete output failures were reproduced through the built public CLI, with fixtures retained under /tmp/reviewquoin1097/duplicate-statements and pypi-normalization.

Fmt and whitespace checks passed at the original head. Independent cargo-deny with CARGO_HOME=/home/peter/.cache/cargo-home-shared and --disable-fetch exited 1 on inherited RUSTSEC-2026-0486 in rustix 1.1.4 (deny.log); that version predates this diff. A clippy invocation overlapped the author's uncommitted refactor and failed on a temporarily removed Write import; original head explicitly has the import, so that transient result is not attributed to the reviewed commit and is not counted as a passing gate. No broad suite run is claimed.

Workflow scope: `.github/workflows/build-test.yml` edits the existing schema-fetch step, expanding one pinned OpenVEX fetch with four pinned CycloneDX resources and adding CYCLONEDX_SCHEMA_PATH to the existing Test environment. No trigger, job, permission, credential scope or test-gate narrowing was found. This remains a real edit to an existing workflow step; it is not described as a new standalone workflow. The additional fetches are test inputs, not production runtime downloads.

Primary references: [pinned CycloneDX 1.7 schema](https://raw.githubusercontent.com/CycloneDX/specification/349314a9d7671d7d2ca5b711a725f49a73979da6/schema/bom-1.7.schema.json), [current PyPI PURL definition](https://raw.githubusercontent.com/package-url/purl-spec/main/types/pypi-definition.json).

Additional confined schema-boundary probe: /tmp/reviewquoin1097/version-boundary/out.json contains a 1025-character affected version; export exit 0, schema maxLength failure. The other emitted free-text native fields were checked and have no corresponding maxLength bound.

Ticket resolution: the dispatching lead explicitly authorized PLAT-1914 from the PR body Refs field because the planner directs ticket IDs to stay out of branch names and PR titles. The ticket is real and already validated in this reviewer session. This instruction supersedes the skill's default branch/title-only resolver for this PR.

## Disposition pass 1

Exact reviewed head: ed0f1f6b2e1855b57ae291a5864c761abf9360a2. **PASS; no unresolved finding and no new defect found in the fix-round diff.** PR mergeable: yes. Original findings above remain unchanged. Rechecked the entire four-file fix-round delta from the original review head, including shared source parsing, shared atomic writing, duplicate refusal before persistence, same-path replacement, PyPI separator normalization, version boundaries, and new criterion tags.

Additional examined scope:

```yaml
scope:
- id: FR-156-AC-11
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: Two identical authored statements in one SOUP record refuse export with
    an error naming the record and `vex_statements`, and no output file is created
    or replaced.
- id: FR-156-AC-12
  path: spec/functional/FR-156-vex-export-for-soup-statements.md
  role: examined
  excerpt: A 1,024-character component version exports and validates against the CycloneDX
    1.7 schema; a 1,025-character component version refuses with an error naming the
    record and `version`, and does not replace an existing output.
```

Final matrix: 2,633 rows; 1,785 tagged, 622 untagged, 179 method-only, 11 ignored-only, 36 retired. FR-156 AC-1 through AC-12 and CON-1/CON-3 are tagged; CON-2 is inspection. These are row counts, not unique missing requirement counts, and do not claim executed evidence. Inherited generated/uncertain identities remain for follow-up classification.

Evidence: final focused CycloneDX test run 10/10 and OpenVEX regression 6/6 independently reproduced against the freshly rebuilt CLI, with retained final output in /tmp/reviewquoin1097/final-tests.txt and final-openvex.txt. Fmt, diff check, and final quoin-soup library clippy passed; final FR-156 grammar validation reports 1/1 clean, zero findings. The inherited cargo-deny rustix advisory remains routed separately; no dependency suppression was added. GitHub reports current head MERGEABLE and all actual check runs successful (one CLA alternative skipped). No broad suite is claimed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified fixed at the disposition head; verbatim after_excerpt recorded below. |
| FND-002 | fixed | Verified fixed at the disposition head; verbatim after_excerpt recorded below. |
| FND-003 | fixed | Verified fixed at the disposition head; verbatim after_excerpt recorded below. |
| FND-004 | fixed | Verified fixed at the disposition head; verbatim after_excerpt recorded below. |
| FND-005 | fixed | Verified fixed at the disposition head; verbatim after_excerpt recorded below. |

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "if !unique_vulnerabilities.insert(unique) {\n                return\
    \ Err(super::vex::source_invalid(\n                    record,\n             \
    \       \"vex_statements\",\n                    \"duplicate statements produce\
    \ identical CycloneDX entries, which the schema forbids\",\n                ));\n\
    \            }"
- fnd: FND-002
  outcome: fixed
  after_excerpt: "pub(super) fn write_atomic<T: Serialize>(\n    output: &Path,\n\
    \    document: &T,\n    format: &str,\n) -> Result<(), Stop> {\n    let mut bytes\
    \ = serde_json::to_vec_pretty(document)\n        .map_err(|error| Stop::Fail(format!(\"\
    cannot serialize {format} document: {error}\")))?;\n    bytes.push(b'\\n');"
- fnd: FND-003
  outcome: fixed
  after_excerpt: "let output_path = fixture.path().join(\"same-output.json\");\n \
    \   assert!(fixture.command(Some(&output_path), None).status.success());\n   \
    \ let first = std::fs::read(&output_path).unwrap();\n    assert!(fixture.command(Some(&output_path),\
    \ None).status.success());\n    assert_eq!(first, std::fs::read(output_path).unwrap());"
- fnd: FND-004
  outcome: fixed
  after_excerpt: "fn normalize_pypi_name(name: &str) -> String {\n    let mut normalized\
    \ = String::with_capacity(name.len());\n    let mut previous_was_separator = false;\n\
    \    for character in name.chars() {\n        if matches!(character, '-' | '_'\
    \ | '.') {\n            if !previous_was_separator {\n                normalized.push('-');\n\
    \            }\n            previous_was_separator = true;\n        } else {\n\
    \            previous_was_separator = false;\n            normalized.extend(character.to_lowercase());\n\
    \        }\n    }\n    normalized\n}"
- fnd: FND-005
  outcome: fixed
  after_excerpt: "if component_version\n        .as_ref()\n        .is_some_and(|version|\
    \ version.chars().count() > 1024)\n    {\n        return Err(super::vex::source_invalid(\n\
    \            record,\n            \"version\",\n            \"exceeds CycloneDX\
    \ 1.7's 1024-character affected-version limit\",\n        ));\n    }"
```
