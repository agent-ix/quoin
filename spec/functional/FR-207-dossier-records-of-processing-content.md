---
id: FR-207
title: "The dossier records of processing render authored activities and listed model facts"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-191"
    type: extends
  - target: "ix://agent-ix/spec-objects-security/FR-028"
    type: depends_on
  - target: "ix://agent-ix/quire-rs/FR-096"
    type: depends_on
  - target: "ix://agent-ix/spec-artifacts-process/FR-018"
    type: depends_on
---
# FR-207: The dossier records of processing render authored activities and listed model facts

## Description

When `quoin dossier generate` selects `records-of-processing` (document id `DOS-ROP`), it SHALL write one section for each `processing_activity` in the selected scope. Each entry SHALL use the sources in the table below. The document is the committed written record; `check` and `export` SHALL use the existing dossier currency rules in FR-191. The document SHALL show gaps instead of filling missing entries with defaults or deriving them from an unlisted source.

## Inputs

- Valid `processing_activity` records and their `covers` edges (spec-objects-security FR-028).
- The `quire.flow-report/v1` model facts from quire-rs FR-096, read through `quoin-quire`.
- `DossierDocument` with `document_kind: records-of-processing` (spec-artifacts-process FR-018).

## Outputs

- `dossier/DOS-ROP.md`, with `type: DossierDocument`, `document_kind: records-of-processing` and `generated_by: quoin dossier`.
- A deterministic JSON gap report with the same gap rows and order as the Markdown.

## Field-to-source table

The source column is exhaustive. Quoin SHALL use only the named authored fields and model facts. A missing source value is a gap. An absent field, edge or record SHALL NOT be inferred from a name, code, runtime observation, another unlisted field or another document.

| GDPR Art 30 entry | Controller record (Art 30(1)) | Processor record (Art 30(2)) |
|---|---|---|
| (a) Controller / processor identity and contacts | Authored `processing_activity.controller`; optional `joint_controllers`, `representative` and `dpo_contact` | Authored `processing_activity.controller` list (each controller served); optional `representative` and `dpo_contact` |
| (b) Purposes | Authored `processing_activity.purposes` | Model: each covered sink's authored `kind` and `purpose` |
| (c) Categories of data subjects | Authored `processing_activity.data_subject_categories` | Model: categories of processing from covered sinks' `kind` and `purpose` |
| (c) Categories of personal data | Model: `data_classification` records carried by covered connections or sinks, and records carried by connections delivering to covered sinks; use each classification's authored `regulated_categories` and title | Same source and rendering as controller record |
| (d) Categories of recipients | Model: each covered sink's authored `processor`, `kind` and `audience_roles` | Model: each covered sink's authored `processor`, `kind` and `audience_roles` |
| (e) Transfers to third countries | Model: each covered sink with an authored `region` member outside the activity's authored `home_regions` | Same source and rendering as controller record |
| (e) Safeguards for those transfers | Model: controls with a `protects` edge to each transfer sink and their authored `safeguards` | Same source and rendering as controller record |
| (f) Time limits for erasure | Authored `processing_activity.retention` | Not applicable to the Art 30(2) layout; SHALL NOT be printed as a processor fact |
| (g) Technical and organisational security measures | Model: controls with a `protects` edge to a covered connection or sink and their authored `safeguards` | Same source and rendering as controller record |

For a covered `part`, its direct `delivers_to` sinks are covered sinks for this report. A connection is covered only when it is an explicit `covers` target. A classification reaches a covered sink when the sink carries it or a connection with a `delivers_to` edge to that sink carries it. A control selected by more than one covered element SHALL appear once per activity entry.

## Behavior

### Document layout

- `DOS-ROP.md` SHALL contain one `## <activity id> — <activity title>` section per `processing_activity`, ordered by dossier id order.
- A controller activity SHALL render the Art 30(1) entries. A processor activity SHALL render the Art 30(2) entries and SHALL omit controller-only entries (b) purposes authored by the processor and (f) retention.
- Model records and controls SHALL be listed by dossier id order. The document SHALL preserve authored category, purpose, region, audience and safeguards values and SHALL NOT infer a value from a display name.
- The document SHALL name source model records by id. It SHALL NOT claim legal compliance, adequacy or completeness of the underlying processing.

### Gaps

- A missing or unusable value named by the field-to-source table SHALL produce a gap with the relevant `gap_kind`, a source subject id, and a detail identifying the missing field or relation.
- With no `processing_activity`, the report SHALL contain `no_processing_activity` and no activity section.
- If required facts are absent from the flow report, the report SHALL contain `model_facts_unavailable` naming the activity or model element; Quoin SHALL NOT read a substitute source.
- A covered connection with no `carries` edge SHALL include the row `GAP: (c) categories of personal data — <connection id> carries no classification` and one `personal_data_categories_unstated` gap.
- A covered sink with no processor, kind or audience-role evidence for the recipient entry SHALL include a `recipient_categories_unstated` gap naming the sink.
- A covered sink with no `region` SHALL include a `transfer_region_unstated` gap because the transfer condition cannot be determined. A region within `home_regions` is not a transfer.
- A transfer sink without a protecting control with a stated `safeguards` value SHALL include a `transfer_safeguards_unstated` gap naming the sink.
- A covered connection or sink without any protecting control with a stated `safeguards` value SHALL include a `security_measures_unstated` gap naming that element.
- Every gap SHALL appear in the document's final `Gaps` section and in the JSON report. The document SHALL have `status: incomplete` when any gap exists, following FR-191.

### Source boundary

- The source reader SHALL consume `quire.flow-report/v1` facts through `quoin-quire`; it SHALL NOT parse QSL or implement a second edge resolver.
- FR-096 facts are emitted only for archetypes, fields and edges read by loaded flow rules. When a needed activity or model fact is not present in those facts, Quoin SHALL report the gap defined above. It SHALL NOT claim that an omitted fact is absent from the repository.
- No field or value outside the field-to-source table SHALL fill an Art 30 entry.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-207-AC-1 | A complete controller fixture yields one activity section whose personal-data categories include `CLASS-PHI` and its `health` category, recipients include `EXT-LAB` and the sink's audience role, and transfers include `SINK-US-ANALYTICS` with its protecting control and safeguards. | Test |
| FR-207-AC-2 | A controller activity without authored `retention` fails `quire validate`; a processor activity without it validates and its Art 30(2) section omits entry (f). | Test |
| FR-207-AC-3 | A covered connection without `carries` produces `GAP: (c) categories of personal data — CONN-7 carries no classification`, one `personal_data_categories_unstated` gap, and a non-zero gap count. | Test |
| FR-207-AC-4 | A transfer sink with no protecting control that states `safeguards` produces a `transfer_safeguards_unstated` gap naming the sink. | Test |
| FR-207-AC-5 | A processor activity renders the Art 30(2) layout, uses covered sinks' `kind` and `purpose` for categories of processing, and omits controller-only entries (b) and (f). | Test |
| FR-207-AC-6 | A fixture with a missing `covers` target fails validation as dangling, and a `covers` target of `queue` fails as wrong-kind. | Test |
| FR-207-AC-7 | Missing facts in `quire.flow-report/v1` produce an explicit gap; changing only an unlisted source such as runtime data or an artifact name does not change any entry. | Test |
| FR-207-AC-8 | Two runs over the same authored records and flow report produce byte-identical Markdown and JSON; adding a `carries` edge changes `check` to `differs` until regeneration. | Test |

## Dependencies

- **Upstream**: spec-objects-security FR-028; spec-objects-architecture `connection`, `data_sink`, `carries` and `delivers_to` (PLAT-1440, PLAT-1532, PLAT-1535); quire-rs FR-096; spec-artifacts-process FR-018; quoin FR-191.
- **Downstream**: PLAT-1630 (EU-Law Compliance Pack).
