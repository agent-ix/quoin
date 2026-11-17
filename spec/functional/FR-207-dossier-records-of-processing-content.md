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

When `quoin dossier generate` selects `records-of-processing` (document id `DOS-ROP`), it SHALL write one section for each `processing_activity` in the selected scope. Each entry SHALL use the sources in the table below. Generation SHALL NOT require a commit, version, `check`, or byte-comparison step. The document SHALL show gaps instead of filling missing entries with defaults or deriving them from an unlisted source.

## Inputs

- Valid `processing_activity` records and their security-owned `scopes` edges to `connection`, `data_sink` or `part` objects (spec-objects-security FR-028). These model-scope edges are distinct from ISO `covers` requirement-coverage edges.
- The `quire.flow-report/v1` model facts from quire-rs FR-096, read through `quoin-quire`.
- `DossierDocument` with `document_kind: records-of-processing` (spec-artifacts-process FR-018).

## Outputs

- `dossier/DOS-ROP.md`, with `type: DossierDocument`, `document_kind: records-of-processing` and `generated_by: quoin dossier`.
- A deterministic JSON gap report with the same gap rows and order as the Markdown.

## Field-to-source table

The source column is exhaustive. Quoin SHALL use only the named authored fields and model facts. A missing source value is a gap. An absent field, edge or record SHALL NOT be inferred from a name, code, runtime observation, another unlisted field or another document.

| GDPR Art 30 entry | Controller record (Art 30(1)) | Processor record (Art 30(2)) |
|---|---|---|
| (a) Identity and contacts | Authored `processing_activity.controller`; optional `joint_controllers`, `representative` and `dpo_contact` | Art 30(2)(a): authored `processing_activity.processor` for the processor's own name and contact, and `processing_activity.controller` for each controller it acts for; optional `representative` and `dpo_contact` |
| (b) Purposes / categories of processing | Art 30(1)(b): authored `processing_activity.purposes` | Art 30(2)(b): each scoped sink's authored `kind` is listed as a category of processing under each controller named by `processing_activity.controller`; its `purpose`, when present, is shown separately as purpose context and is not counted as a category |
| (c) Categories of data subjects | Authored `processing_activity.data_subject_categories` | Not an Art 30(2) entry; SHALL NOT be printed as a processor fact |
| (c) Categories of personal data | Model: `data_classification` records carried by scoped connections or sinks, and records carried by connections delivering to scoped sinks; use each classification's authored `regulated_categories` and title | Not an Art 30(2) entry; SHALL NOT be printed as a processor fact |
| (d) Categories of recipients | Model: each scoped sink's authored `processor`, `kind` and `audience_roles` | Not an Art 30(2) entry; SHALL NOT be printed as a processor fact |
| (e) Transfers to third countries and safeguards | Not derivable from `region` and `home_regions`; emit `transfer_region_unstated` and `transfer_safeguards_unstated` gaps for each scoped sink. The labels MAY be reported separately as model location facts. | Art 30(2)(c): same transfer gap and supplemental-location contract as the controller record |
| (f) Time limits for erasure | Authored `processing_activity.retention` | Not an Art 30(2) entry; SHALL NOT be printed as a processor fact |
| (g) Technical and organisational security measures | Model: controls with a `protects` edge to a scoped connection or sink and their authored `safeguards` | Art 30(2)(d): model controls with a `protects` edge to a scoped connection or sink and their authored `safeguards` |

`data_sink.region` and `processing_activity.home_regions` are authored location labels. A location is outside the declared home-region labels only when its value does not exactly match any value in `home_regions`; `eu` and `eea` are compared as labels and are not expanded to country lists. This comparison MAY be shown as a separate location fact. It SHALL NOT be described as, or used to infer, a transfer to a third country. The model supplies no legal transfer classification, so the Art 30 transfer entry remains an explicit gap even when location labels are present. Controls protecting a sink MAY supply general security measures, but SHALL NOT be represented as safeguards for a third-country transfer without a transfer classification.

For a `processing_activity`, an explicit security-owned `scopes` edge selects a `connection`, `data_sink` or `part` as scoped. Direct `scopes` targets of type `connection` or `data_sink` are scoped elements; each direct `delivers_to` sink of a scoped `part` is also a scoped sink. Quoin SHALL NOT use an ISO `covers` edge to determine model scope. A classification reaches a scoped sink when the sink carries it or a connection with a `delivers_to` edge to that sink carries it. A control selected by more than one scoped element SHALL appear once per activity entry.

## Behavior

### Document layout

- `DOS-ROP.md` SHALL contain one `## <activity id> — <activity title>` section per `processing_activity`, ordered by dossier id order.
- A controller activity SHALL render the Art 30(1) entries. A processor activity SHALL render the Art 30(2) entries, including the processor's own identity/contact and the controllers it acts for, and SHALL NOT render the Art 30(1) purpose, data-subject category, personal-data category, recipient or retention entries as processor facts. Its scoped sinks' `kind` values SHALL appear once under Art 30(2)(b) as categories of processing; a sink's `purpose` SHALL be separate purpose context, not a category or a duplicate Art 30(1) purpose entry.
- Model records and controls SHALL be listed by dossier id order. The document SHALL preserve authored category, purpose, region, audience and safeguards values and SHALL NOT infer a value from a display name.
- The document SHALL name source model records by id. It SHALL NOT claim legal compliance, adequacy or completeness of the underlying processing.

### Gaps

- A missing or unusable value named by the field-to-source table SHALL produce a gap with the relevant `gap_kind`, a source subject id, and a detail identifying the missing field or relation.
- With no `processing_activity`, the report SHALL contain `no_processing_activity` and no activity section.
- If required facts are absent from the flow report, the report SHALL contain `model_facts_unavailable` naming the `DOS-ROP` document; Quoin SHALL NOT read a substitute source or claim the model element is absent from the repository.
- A scoped connection with no `carries` edge SHALL include the row `GAP: (c) categories of personal data — <connection id> carries no classification` and one `personal_data_categories_unstated` gap.
- A scoped sink with no processor, kind or audience-role evidence for the recipient entry SHALL include a `recipient_categories_unstated` gap naming the sink.
- Every scoped sink SHALL include a `transfer_region_unstated` gap because the model's location labels cannot establish a third-country transfer. The report MAY separately show whether a sink's location label is outside the activity's exact `home_regions` labels.
- Every scoped sink SHALL include a `transfer_safeguards_unstated` gap because the model does not identify which movements are third-country transfers. A control's safeguards may still be listed as general security measures.
- A scoped connection or sink without any protecting control with a stated `safeguards` value SHALL include a `security_measures_unstated` gap naming that element.
- Every gap SHALL appear in the document's final `Gaps` section and in the JSON report. The document SHALL have `status: incomplete` when any gap exists, following FR-191.

### Source boundary

- The source reader SHALL consume `quire.flow-report/v1` facts through `quoin-quire`; it SHALL NOT parse QSL or implement a second edge resolver.
- For model scope, Quoin SHALL use the processing activity's security-owned `scopes` targets exposed in `quire.flow-report/v1`. It SHALL NOT treat ISO `covers` requirement-coverage edges as processing-activity scope.
- FR-096 facts are emitted only for archetypes, fields and edges read by loaded flow rules. When a needed activity or model fact is not present in those facts, Quoin SHALL report the gap defined above. It SHALL NOT claim that an omitted fact is absent from the repository.
- No field or value outside the field-to-source table SHALL fill an Art 30 entry.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-207-AC-1 | A controller fixture with security-owned `scopes` targets yields one activity section whose personal-data categories include `CLASS-PHI` and its `health` category, recipients include `EXT-LAB` and the sink's audience role, and the Art 30 transfer entry has explicit region and safeguards gaps; the sink's region label and protecting control are reported only as model facts. | Test |
| FR-207-AC-2 | A controller activity without authored `retention` fails `quire validate`; a processor activity without it validates and its Art 30(2) section omits entry (f). | Test |
| FR-207-AC-3 | A scoped connection without `carries` produces `GAP: (c) categories of personal data — CONN-7 carries no classification`, one `personal_data_categories_unstated` gap, and a non-zero gap count. | Test |
| FR-207-AC-4 | A scoped sink produces `transfer_region_unstated` and `transfer_safeguards_unstated` gaps naming the sink even when it has a region label and a protecting control; controls with `safeguards` may appear only under general security measures. | Test |
| FR-207-AC-5 | A processor activity with authored processor identity/contact and controller contacts renders Art 30(2)(a), lists each scoped sink's `kind` once as a category of processing under Art 30(2)(b), shows its `purpose` separately when present, and does not duplicate either as data-subject categories. | Test |
| FR-207-AC-6 | A fixture with a dangling `processing_activity` `scopes` target fails validation as dangling, and a `scopes` target of `queue` fails as wrong-kind; ISO `covers` edges do not select processing scope. | Test |
| FR-207-AC-7 | Missing facts in `quire.flow-report/v1` produce an explicit gap; changing only an unlisted source such as runtime data or an artifact name does not change any entry. | Test |
| FR-207-AC-8 | Two runs over the same authored records and flow report produce byte-identical Markdown and JSON; generation requires no commit, version, check, or byte-comparison step. | Test |
| FR-207-AC-9 | With two scoped sink facts sharing an artifact id across different repositories and a classified scoped connection whose only `delivers_to` fact names the repo-a sink, the report retains both sinks as distinct `(repository, id)` entries, associates the classification only with repo-a, and contains no `personal_data_categories_unstated` gap for repo-a and exactly one such gap naming repo-b. The result uses only those listed flow-report facts and report output, without legal inference. | Test |

## Dependencies

- **Upstream**: spec-objects-security FR-028 (security-owned `scopes`, distinct from ISO `covers`); spec-objects-architecture `connection`, `data_sink`, `carries` and `delivers_to` (PLAT-1440, PLAT-1532, PLAT-1535); quire-rs FR-096; spec-artifacts-process FR-018; quoin FR-191.
- **Downstream**: PLAT-1630 (EU-Law Compliance Pack).
