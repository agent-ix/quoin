---
id: FR-191
title: "quoin dossier generates, checks and exports the selected dossier documents"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
---

# FR-191: quoin dossier generates, checks and exports the selected dossier documents

## Description

Quoin SHALL provide `quoin dossier generate`, which writes the selected dossier documents from the repository's records.

Quoin SHALL provide `quoin dossier check`, which compares the selected dossier documents with the files on disk and writes nothing.

Quoin SHALL provide `quoin dossier export`, which exports the selected, committed and current dossier documents through `quire export`.

The M1 documents are SRS (`srs`, id `DOS-SRS`), architecture
(`architecture`, id `DOS-ARCH`), trace matrix (`trace-matrix`, id
`DOS-TRACE`) and requirements mapping matrix (`rmm`, id `DOS-RMM`). The
M2 documents are verification report (`verification-report`, `DOS-VR`),
SOUP list (`soup`, `DOS-SOUP`), unresolved anomalies (`anomalies`,
`DOS-ANOM`), risk management (`risk-management`, `DOS-RISK`), risk
management plan (`risk-management-plan`, `DOS-RMP`) and release record
(`release-record`, `DOS-REL`). The M3 document is the threat model (`threat-model`, `DOS-TM`),
specified in [FR-202](./FR-202-dossier-threat-model-content.md). The records-of-processing document is
`records-of-processing` (`DOS-ROP`), specified in [FR-207](./FR-207-dossier-records-of-processing-content.md).
A document kind becomes selectable for generation when its generator is registered; the command frame can ship before later
generators. `records-of-processing` is generated directly and is excluded from the versioned `check` and `export` flows.

## Inputs

- `generate [--doc <registered-kind>]... [--out <DIR>] [--scope <DIR>] [--module <PATH>]... [--clause-binding <PATH>]... [--pack <PACK>] [--soup-binding <PATH>] [--soup-records <DIR>] [--soup-support <PATH>] [--snapshot <PATH>] [--release-verify <PATH>] [--as-of <RFC-3339>] [--version <TEXT>] [--json]`. The common `--scope` and `--module` select anomaly, risk and threat-model records; the SOUP and release flags supply their separate sources. `records-of-processing` does not use `--version`.
- `check` takes the selectors for versioned document kinds; it excludes `records-of-processing`.
- `export --to pdf|docx --output <PATH>` takes the selectors for exportable versioned kinds, and also `[--release <TAG>] [--approval <PATH>]... [--template <PATH>] [--quire <PATH>]`.

## Outputs

- `generate`: one Markdown file per selected registered document at `<DIR>/<document-id>.md`. `<DIR>` is `dossier` unless `--out` names another. With no `--doc`, every registered generator is selected. `check` and `export` select only registered kinds supported by those operations.
- `check`: per document, `matches` or `differs`.
- `export`: the file `quire export` writes.
- With `--json`: `{documents:[{id, kind, path, status, gaps:[{gap_kind, subject, detail}]}]}` where `status` is `complete` or `incomplete`. `check` and `export` add `comparison` (`matches` or `differs`) to each document. On exit 3 the report holds `source_failure:{source, detail}`. On exit 4 the report holds `quire:{exit_code, stderr}`.

## Behavior

### Commit then export

- `generate` SHALL write Markdown into the repository.
- `generate` SHALL NOT call `quire export`.
- `export` SHALL resolve `HEAD` once, before checking export currency, to a full immutable commit id and SHALL pass that id as `--source-rev` to `quire export`.
- `export` SHALL pass `--release`, each user-supplied `--approval` and `--template` to `quire export` unchanged.
- When `release-record` is selected and `--release-verify` is supplied, `export` SHALL append that EA result as one more `--approval` after every user-supplied approval, including in a bundle; when `release-record` is not selected, it SHALL NOT append that result.
- When `export` selects more than one document, `export` SHALL run `quire export` once, as one bundle, with the files in this fixed dossier order, independent of generator registration time: the four M1 ids above, then `DOS-VR`, `DOS-SOUP`, `DOS-ANOM`, `DOS-RISK`, `DOS-RMP`, `DOS-REL` for the registered M2 kinds, then `DOS-TM` when registered, then later exportable kinds in their specified order. `DOS-ROP` is not exportable under this command. It SHALL pass `--bundle-title "Submission Dossier"` and `--bundle-id DOS`.
- When `export` selects one document, `export` SHALL run `quire export` once with that file and without `--bundle-title` and `--bundle-id`.
- The flow is commit-then-export because `quire export` takes identity and content from the blob committed at `--source-rev` and refuses a working file that differs from that commit (quire-cli FR-034 section A), and refuses identity options on JSON input. A bundle requires at least two inputs (quire-cli FR-039-AC-7), which is why one document is exported without bundle arguments.
- Quoin SHALL NOT export an uncommitted file.
- Quoin SHALL NOT render a document itself.
- Quoin SHALL NOT bypass `quire export`.

### Export currency

- `export` SHALL resolve each selected exportable document at `<DIR>/<document-id>.md`, where `<DIR>` is `dossier` unless `--out` names another.
- `export` SHALL regenerate each selected document in memory and compare its bytes with the file at its path before it starts `quire export`.
- `export` SHALL compare the regenerated bytes with the selected document blob at the resolved commit id; a missing or different committed blob SHALL fail before `quire export` starts.
- If a selected file or committed blob is missing or its bytes differ from the regenerated bytes, `export` SHALL exit 1, report that document as `differs`, leave `--output` unchanged, and SHALL NOT start `quire export`.
- `export` SHALL NOT refuse a document because it is `incomplete`; the `Gaps` section travels with the document.
- If `quire export` fails, `export` SHALL leave any file already present at `--output` byte-unchanged, as required by quire-cli FR-033.

### Selection and check

- If a `--doc` value has no registered generator, the command SHALL exit 2 and SHALL write nothing.
- `generate`, `check` and `export` SHALL require `--version` when a selected kind uses version metadata. `records-of-processing` generation SHALL neither require nor emit a version.
- `check` and `export` SHALL reject an explicit `records-of-processing` selection with exit 2 and no output. When no `--doc` is supplied, `check` and `export` SHALL exclude this generation-only kind.
- `check` SHALL regenerate each selected versioned document in memory and compare its bytes with the file at its path.
- `check` SHALL write nothing.
- `check` SHALL count a missing file as `differs`.

### Sources and failure

- A source is one of the record readers named in FR-192 to FR-207: `requirements`, `design-join`, `design-matrix`, `systems-records`, `matrix`, `discharge`, `soup-report`, `anomaly-report`, `risk-summary`, `risk-advise`, `acceptability-policy`, `release-snapshot`, `release-verify`, `threat-model-records`, `flow-findings` and `processing-activities`. A source is a library operation of the Quoin workspace or an explicitly supplied EA result and gives the same result as its named source command, except as FR-195 states for approval-class clauses.
- `generate`, `check` and `export` SHALL compute every selected document in memory before they write or compare anything.
- If a source fails, the command SHALL exit 3, SHALL name the failed source and its error in `source_failure`, and SHALL write nothing.
- If `generate` exits 2 or 3, `generate` SHALL leave every existing file under `<DIR>` unmodified.
- `generate` SHALL check before writing that `<DIR>` can be created, and SHALL exit 2 if it cannot.

### Ordering

- Quoin SHALL compare ids in dossier order: an id is split at `-` into segments; two segments of ASCII digits compare by numeric value, any other two segments compare by byte value, a digit segment sorts before a non-digit segment, and an id that is a prefix of another sorts first. `FR-99` sorts before `FR-100`, and `FR-100-AC-2` before `FR-100-AC-10`.
- Quoin SHALL sort gaps by the key (requirement id, criterion id, element id, clause id, gap kind position), where an absent id sorts first and the gap kind position is the position of the kind in the closed list below.
- A gap that names a part, a connection, a sink or a threat uses that id as its element id.
- A gap with no id (`no_requirements`, `design_relations_unavailable`, `no_clause_binding`, `no_threat_model`, `no_processing_activity`) sorts before gaps that name an id.

### Document set

- The generated files SHALL carry the frontmatter of the `DossierDocument` archetype that spec-artifacts-process defines for this feature: `id`, `title`, `type: DossierDocument`, `document_kind` and `generated_by: quoin dossier`; versioned kinds also carry `version`, while `records-of-processing` does not. Source-backed RMM and SOUP documents also carry `as_of`.
- Each file SHALL end with the level-2 `Gaps` section below.

### Gaps section

- Each missing input SHALL be one bullet under the final level-2 heading `Gaps`, written `` `<gap_kind>` <subject>: <detail> ``.
- The section SHALL read `Gaps: none` when it holds no gap.
- Quoin SHALL use only these `gap_kind` values, in this order: `no_requirements`, `no_acceptance_criteria`, `criticality_unassigned`, `design_relations_unavailable`, `no_design_element`, `dangling_design_reference`, `design_kind_mismatch`, `no_allocation`, `unbacked_criterion`, `no_evidence`, `no_clause_binding`, `clause_open`, `evidence_failed`, `evidence_stale`, `evidence_suspect`, `soup_unresolved_advisory`, `soup_past_end_of_support`, `soup_field_not_stated`, `anomaly_unassessed`, `anomaly_no_release_rationale`, `risk_chain_incomplete`, `residual_risk_undecided`, `control_unverified`, `no_acceptability_policy`, `no_plan_record`, `release_not_verified`, `no_release_verify_input`, `no_snapshot`, `crossing_without_threat`, `classified_sink_without_threat`, `threat_without_treatment`, `no_threat_model`, `no_processing_activity`, `model_facts_unavailable`, `personal_data_categories_unstated`, `recipient_categories_unstated`, `transfer_region_unstated`, `transfer_safeguards_unstated`, `security_measures_unstated`. Later document kinds SHALL extend this list through their own spec before registration.
- An M3 gap SHALL name a connection for `crossing_without_threat`, a sink for `classified_sink_without_threat`, a threat for `threat_without_treatment`, and the document for `no_threat_model`.
- A gap SHALL name one subject: the document for `no_requirements`, `design_relations_unavailable` and `no_clause_binding`; a requirement for `no_acceptance_criteria`, `criticality_unassigned` and `no_design_element`; a criterion for `unbacked_criterion` and `no_evidence`; a design reference or element for `dangling_design_reference`, `design_kind_mismatch` and `no_allocation`; a clause for `clause_open`.
- An M2 gap SHALL name its source subject: a criterion for the three `evidence_*` kinds; a component and finding or field for the three `soup_*` kinds; an anomaly for the two `anomaly_*` kinds; a risk row, residual risk or control for the three risk-chain kinds; the document for absent `no_acceptability_policy` and `no_plan_record`; and the release document or its source result for the three release kinds.
- A records-of-processing gap SHALL name a covered connection or sink for absent or unstated personal-data categories, a covered sink for an unstated recipient or transfer region, a sink for absent transfer safeguards, a covered connection or sink for missing security measures, and the document for `no_processing_activity` or unavailable model facts.
- A document with at least one gap SHALL have status `incomplete`.
- The `--json` report SHALL hold the same gaps as the Markdown, in the same order.
- Quoin SHALL report a gap and SHALL NOT drop or default it.

### Exit status

| Code | Meaning |
|------|---------|
| 0 | Every selected document is complete (and, for `check`, matches; for `export`, `quire export` succeeded). |
| 1 | Documents were generated but at least one is incomplete, or `check` found a difference, or `export` found a missing or stale file. |
| 2 | Usage error or invalid input, including a missing `--version` and a missing `--as-of` for a supplied RMM or SOUP binding (FR-195, FR-197). |
| 3 | A source failed; the report names it. |
| 4 | `quire export` failed or could not start; its exit code and stderr are passed through, and a missing executable names the path. |

Once the risk-management-plan generator is registered, an unfiltered
`generate` includes `DOS-RMP` and returns 1 because its `no_plan_record` gap
is permanent under FR-200. It still writes every selected document. Select
`--doc` kinds explicitly to receive exit 0 for a complete subset; `export`
continues to allow committed, current incomplete documents.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-191-AC-1 | `generate --doc srs --doc rmm` writes exactly two files and no other | Test |
| FR-191-AC-2 | With only the four M1 generators registered, `generate` with no `--doc` writes those four files at `dossier/<document-id>.md` | Test |
| FR-191-AC-3 | `generate --doc nope` exits 2 and writes no file | Test |
| FR-191-AC-4 | On a fixture whose documents are all complete, `check` on a committed file with one hand-edited line reports that document as `differs` and exits 1; with no edit it reports `matches` and exits 0 | Test |
| FR-191-AC-5 | `check` writes no file and a deleted file reports `differs` | Test |
| FR-191-AC-6 | `export --quire <missing path>` exits 4 and the report names that path | Test |
| FR-191-AC-7 | When a current generated file is absent from the resolved commit or its committed bytes differ from the regenerated bytes, `export` exits 1, reports the document as `differs` in its report, does not start Quire, and leaves `--output` unchanged | Test |
| FR-191-AC-8 | When only M1 kinds are selected, `export` of two or more documents invokes `quire export` once with the committed files in the order `DOS-SRS`, `DOS-ARCH`, `DOS-TRACE`, `DOS-RMM` regardless of `--doc` order, `--source-rev` set to the full commit id resolved before the currency checks, `--bundle-title "Submission Dossier"`, `--bundle-id DOS`, and `--approval`, `--template` and `--release` unchanged | Test |
| FR-191-AC-9 | `export --doc srs` invokes `quire export` once with that one file and with neither `--bundle-title` nor `--bundle-id` | Test |
| FR-191-AC-10 | A dossier generated with `--out other` is exported by `export --out other`, and `export` without `--out` looks under `dossier` and exits 1 naming the missing document | Test |
| FR-191-AC-11 | `export` over a committed file that differs from the regenerated bytes exits 1, names the document and never starts `quire export` | Test |
| FR-191-AC-12 | A source that fails makes `generate` exit 3, the report names that source in `source_failure`, no file is written for any selected document, and a file that existed before the run is unchanged | Test |
| FR-191-AC-13 | `generate`, `check` and `export` without `--version` exit 2 and write nothing when a selected kind uses version metadata; `generate --doc records-of-processing` succeeds without `--version` and omits `version` from its output. | Test |
| FR-191-AC-14 | A fixture with two gaps of different kinds gives a `Gaps` section with two bullets in gap order, the same two entries in the same order in `--json`, and `Gaps: none` for a complete document | Test |
| FR-191-AC-15 | A fixture with requirements `FR-99` and `FR-100` and criteria `FR-100-AC-2` and `FR-100-AC-10` lists them in dossier order, and a document-level gap sorts before a gap that names a requirement | Test |
| FR-191-AC-16 | `export` of an `incomplete` document that is committed and current runs `quire export` and does not refuse it | Test |
| FR-191-AC-17 | `generate --out <path that cannot be created>` exits 2 and writes nothing | Test |
| FR-191-AC-18 | With only the M1 and M2 generators registered, no `--doc` selects ten files and a bundle export orders them M1 first, then the six M2 ids above, regardless of selector order | Test |
| FR-191-AC-19 | A registered M2 generator's gap appears in both Markdown and JSON with the same kind, subject, detail and global gap order | Test |
| FR-191-AC-20 | An unregistered future document kind exits 2 without writing | Test |
| FR-191-AC-21 | A bundle selecting `release-record` passes user `--approval` files in their given order and appends the supplied EA `--release-verify` result once after them; a bundle without `release-record` does not append it | Test |
| FR-191-AC-22 | With only the ten M1 and M2 generators registered, unfiltered `generate` writes ten files and exits 1 due to `DOS-RMP`'s permanent `no_plan_record` gap; a complete explicit subset exits 0 | Test |
| FR-191-AC-23 | With the threat-model generator registered, `generate --doc threat-model` writes `dossier/DOS-TM.md` only, `--doc nope` still exits 2, and a bundle export that selects `DOS-TM` and any M1 or M2 id orders `DOS-TM` after every M1 and M2 id regardless of selector order. | Test |
| FR-191-AC-24 | With the records-of-processing generator registered, `generate --doc records-of-processing` writes `dossier/DOS-ROP.md` without a version or commit check; `--doc nope` exits 2; explicit `check` or `export` selection of this kind exits 2, and unfiltered `check` or `export` excludes it. | Test |
| FR-191-AC-25 | The full commit id resolved for `HEAD` is the same id used to read each selected committed document blob and passed to the single `quire export` invocation as `--source-rev`. | Test |
| FR-191-AC-26 | When `--output` already contains sentinel bytes and `quire export` exits unsuccessfully, `export` returns the Quire failure and the output retains exactly the sentinel bytes. | Test |

## Dependencies

- **Upstream**: spec-artifacts-process FR-018 (the `DossierDocument` archetype); quire-cli FR-031 (`quire export`), FR-033 (failed-export output preservation), FR-034, FR-038, FR-039.
- **Downstream**: [FR-192](./FR-192-dossier-srs-content.md) through [FR-195](./FR-195-dossier-rmm-content.md) define M1 content; [FR-196](./FR-196-dossier-verification-report-content.md) through [FR-201](./FR-201-dossier-release-record-content.md) define M2 content; [FR-202](./FR-202-dossier-threat-model-content.md) defines M3 threat-model content; [FR-207](./FR-207-dossier-records-of-processing-content.md) defines records-of-processing content. Later document generators register after their own spec and archetype extension.
