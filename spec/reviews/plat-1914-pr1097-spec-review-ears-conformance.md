---
id: SR-967175026302197768900474181941935084892
title: "spec-review/ears-conformance — CycloneDX SOUP VEX export"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; PLAT-1914; PR #1097; soup exporter, advisory/report paths, integration tests, workflow, how-to, FR-156"
review_set: subset
---

## Summary

Reviewed the exact GitHub head 927028c04409c121b27909ac6aadbbb4b932e83b. Worktree edits by the author after this snapshot are not findings against this head. Explicit authoring/matrix tool: /tmp/regfinish-quoin-v0.37.0/quoin 0.37.0; module spec-artifacts-process@v0.36.0; Quire validator 0.52.0 (engine 0.62.2). Exact model ID is not exposed (model=unknown). No product source was edited by the reviewer.


## Verdict

**CONDITIONAL** — findings below are the initial review at the recorded head.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The added default-output statement contains two SHALL obligations and the agentless passive shall be required. Quire emits ears:non-singular and quality:agentless-passive at this exact line. Split it into explicit command obligations for requiring the SBOM argument and placing the output. | spec/functional/FR-156-vex-export-for-soup-statements.md:39 |

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
bindings: []
```

## Coverage and evidence

Deterministic engine check in spec-validation.txt exits 0 with two advisory findings at the changed line. EARS review covers requirement statements, not acceptance-criterion grammar.

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

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '- When CycloneDX output is selected and `--out` is omitted, the
    command SHALL require `--sbom`.

    - When CycloneDX output is selected, `--out` is omitted and `--sbom` names `probe.cdx.json`,
    the command SHALL write `probe.vex.cdx.json` beside the SBOM.'
```
