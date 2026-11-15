---
id: FR-202
title: "The dossier threat-model document renders each threat model's covered crossings, sinks, treatments and open findings"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-010"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-191"
    type: "extends"
  - target: "ix://agent-ix/spec-objects-security/FR-026"
    type: "references"
  - target: "ix://agent-ix/spec-objects-security/FR-027"
    type: "references"
---

# FR-202: The dossier threat-model document renders each threat model's covered crossings, sinks, treatments and open findings

## Description

When `quoin dossier generate` selects `threat-model` (document id `DOS-TM`), the document SHALL render, for every
`threat_model` in the selected scope, its scope, its covered crossings and sinks with the threats, controls and acceptances
recorded against them, and the open findings of the three completeness checks of spec-objects-security FR-027. The document
only renders recorded facts. It SHALL NOT create a threat, assign a STRIDE class, rate a risk, accept a risk or decide that a
treatment is adequate; threats, ratings and acceptance are person-authored (spec-objects-security NFR-005).

The checks are `flow_rules` data evaluated by the existing relation report. This requirement adds a dossier generator and no
engine change.

## Inputs

- `threat-model-records`: the `threat_model`, `threat`, `risk`, `control`, `connection`, `data_sink`, `trust_boundary` and
  `data_classification` documents of the common dossier `--scope` and `--module` selection, read as a library operation of the
  Quoin workspace.
- `flow-findings`: the findings under `trace:crossing-without-threat`, `trace:classified-sink-without-threat` and
  `trace:threat-without-treatment` of the relation report over the same selection, as a library operation giving the same
  result as `quire relations --flow` for those three checks.

## Outputs

- `dossier/DOS-TM.md`: `# Threat model`, then one `## <threat_model id>` section per `threat_model` in dossier order, then
  `## Gaps`.
- Each section holds, in order: `### Scope` (the model's `method`, `reviewed_by` and its `## Scope`, `## Assumptions` and
  `## Out of scope` text as written); `### Covered crossings`; `### Covered sinks`; `### Open findings`.
- `### Covered crossings` is a table with the columns `Connection | Carried classifications and categories | Boundary and trust
  level | Threats (STRIDE class) | Controls and safeguards | Acceptance`. A row exists for every covered `connection`, whether
  or not it crosses a boundary or carries data.
- `### Covered sinks` lists each covered `data_sink` with its `kind`, the classifications reaching it, its threats and its
  controls.
- `### Open findings` holds one row `GAP: <check> <subject>` per open finding of the three checks for that model's covered
  elements; the section reads `none` when it holds no finding.

## Behavior

- The generator SHALL take a model's covered elements to be the targets of its `covers` edges. It SHALL list a `covers` target
  that is a `part` or a `trust_boundary` under `### Scope` by id, and SHALL add a crossing row only for a `connection`.
- The `Threats` cell SHALL list each `threat` with an incoming `threatens` edge to the connection, as `<id> (<stride_category>)`,
  in dossier id order; an element with none SHALL read `none`.
- The `Controls and safeguards` cell SHALL list each `control` that `protects` the connection, with its `safeguards` as written,
  in dossier id order, and each `control` that `mitigates` a listed threat; an absent entry reads `none`.
- The `Acceptance` cell SHALL list, for each listed threat, each `risk` that `arises_from` it as
  `<risk id>: <decision>, <decided_by>`, reading `not stated` for an absent `decided_by`. It SHALL NOT compute or show a score.
- A carried classification SHALL be shown as its id with its `regulated_categories` as written; a connection carrying none
  reads `none`. A connection crossing no boundary reads `none` in the boundary cell; otherwise the cell shows the boundary id
  and its `trust_level` as written.
- `GAP:` rows and gaps SHALL come only from the three checks' findings. The generator SHALL NOT recompute a check.
- Each `crossing-without-threat` finding SHALL produce one `crossing_without_threat` gap naming the connection; each
  `classified-sink-without-threat` finding one `classified_sink_without_threat` gap naming the sink; each
  `threat-without-treatment` finding one `threat_without_treatment` gap naming the threat. When no `threat_model` is in the
  selection the document SHALL hold no model section and one document-level `no_threat_model` gap.
- Only a `threat_model` document a person authored counts. A proposed draft the repository marks as not committed is not a
  model and SHALL NOT yield a section.
- The document is committed and checked like other dossier documents: `check` compares its bytes, `export` requires it current.
  Two generations over the same records SHALL be byte-identical.
- If a source reader reports an unreadable record, the command SHALL surface a source failure under FR-191 instead of
  treating the record as absent.
- The document SHALL NOT claim a finding the source did not report.
- The document SHALL NOT state that a system is secure, compliant or adequately modelled.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-202-AC-1 | A fixture with a `threat_model` covering two connections, one carrying `CLASS-PHI` across `BOUND-NET` with `THREAT-1` threatening it, mitigated by `CONTROL-1` and with an accepted risk, lists both connections in `### Covered crossings` with the carried classification, the boundary and trust level, `THREAT-1 (<stride_category>)`, the control and its safeguards, and `RISK-1: accepted, A. Person`. | Test |
| FR-202-AC-2 | Removing `THREAT-1`'s `threatens` edge yields a `GAP: crossing-without-threat CONN-1` row under `### Open findings` and one `crossing_without_threat` gap naming `CONN-1`; both appear in `--json` in the same order. | Test |
| FR-202-AC-3 | A considered threat with no control and no accepted risk yields `GAP: threat-without-treatment` and one `threat_without_treatment` gap; with a risk `decision: undecided` it still does, and with `decision: accepted` it does not. | Test |
| FR-202-AC-4 | A covered sink reached by a classification and with no threat yields `GAP: classified-sink-without-threat` and one `classified_sink_without_threat` gap naming the sink, and the sink is listed under `### Covered sinks`. | Test |
| FR-202-AC-5 | A selection with no `threat_model` writes `DOS-TM.md` with no model section, one `no_threat_model` gap, and status `incomplete`; the three checks produce no finding. | Test |
| FR-202-AC-6 | A fully treated fixture reads `none` under `### Open findings`, `Gaps: none`, and status `complete`. | Test |
| FR-202-AC-7 | Two `generate` runs over the same records produce byte-identical Markdown and `--json`; `check` after an unedited run reports `matches`, and after a hand-edited line reports `differs`. | Test |
| FR-202-AC-8 | The document holds no threat, STRIDE class, `decision` or `decided_by` value that does not appear in a record, and no score, rank or adequacy statement; a risk with no `decided_by` shows `not stated`. | Test |
| FR-202-AC-9 | An unreadable `threat` record makes `generate` exit 3 naming the source and leaves every existing file unchanged. | Test |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md); spec-artifacts-process FR-018 (the `threat-model` `document_kind`);
  spec-objects-security FR-026 and FR-027; the quire-rs relation report and `flow_rules` (FR-096).
- **Downstream**: PLAT-1543.
