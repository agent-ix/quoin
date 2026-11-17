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
  - target: "ix://agent-ix/spec-objects-security/FR-025"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-architecture/FR-007"
    type: "depends_on"
---

# FR-202: The dossier threat-model document renders each threat model's covered crossings, sinks, treatments and open findings

## Description

When `quoin dossier generate` selects `threat-model` (document id `DOS-TM`), the document SHALL render, for every
`threat_model` in the selected scope, its scope, its covered crossings and sinks with the threats, controls and acceptances
recorded against them, and the open findings of the three completeness checks of spec-objects-security FR-027. The document
only renders recorded facts. It SHALL NOT create a threat, assign a STRIDE class, rate a risk, accept a risk or decide that a
treatment is adequate; threats, ratings and acceptance are person-authored (spec-objects-security NFR-005).

The checks are `flow_rules` data evaluated by the quire-rs relation report once quire-rs delivers `flow_rules` (FR-096; PLAT-1537, PLAT-1538). This requirement adds a dossier generator and no
engine change.

## Inputs

- `threat-model-records`: the `threat_model`, `threat`, `risk`, `control`, `connection`, `data_sink`, `trust_boundary` and
  `data_classification` documents of the common dossier `--scope` and `--module` selection, read as a library operation of the
  Quoin workspace. Only committed documents count: a draft outside the selected corpus is never read, and `threat_model` and
  `threat` documents are the person-authored records of spec-objects-security FR-026.
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
  controls and safeguards, as the Behavior section defines.
- `### Open findings` holds one row `GAP: <check> <subject> (<severity>)` per open finding of the three checks on that
  model's covered elements and considered threats; the section reads `none` when it holds no finding.

### Ordering

- `## <threat_model id>` sections are in dossier order, the order of FR-191.
- `### Scope` lists covered `part` ids, then covered `trust_boundary` ids, each group in dossier id order.
- `### Covered crossings` rows are in dossier id order of the connection; `### Covered sinks` entries are in dossier id order
  of the sink.
- `### Open findings` rows are in the order (check, subject id), the check in the order `crossing-without-threat`,
  `classified-sink-without-threat`, `threat-without-treatment` and the subject id in dossier id order.
- The `## Gaps` entries follow FR-191 Ordering: the gap kind position is the position of the kind in the FR-191 closed list;
  a connection, a sink or a threat is the gap's element id; `no_threat_model` names no id and sorts before gaps that name an id.

## Behavior

- The generator SHALL take a model's in-scope elements to be the targets of its `scopes` edges. A target reached only by a
  `covers` edge SHALL NOT be included. It SHALL list a `scopes` target that is a `part` or a `trust_boundary` under
  `### Scope` by id, and SHALL add a crossing row only for a `connection`.
- The `Threats` cell SHALL list each `threat` with an incoming `threatens` edge to the connection, as `<id> (<stride_category>)`,
  in dossier id order; an element with none SHALL read `none`.
- The `Controls and safeguards` cell SHALL list each `control` that `protects` the connection, with its `safeguards` as written.
- The `Controls and safeguards` cell SHALL also list each `control` that `mitigates` a threat listed in the `Threats` cell, with
  its `safeguards` as written.
- A control selected by both rules SHALL appear once in the cell.
- The cell SHALL order its controls by dossier id, the two groups merged and not shown apart. A cell with no control SHALL read `none`.
- The `### Covered sinks` controls SHALL be selected, deduplicated, ordered and shown with their safeguards by the same three
  rules, with `protects` read against the sink and `mitigates` against the sink's threats, which are the `threat` documents
  with an incoming `threatens` edge to the sink.
- The classifications reaching a covered sink SHALL be those of the `data_classification` documents carried, by `carries`
  edges, by the sink itself or by a `connection` with a `delivers_to` edge to the sink, read from the `threat-model-records`
  graph in dossier id order. This is the walk FR-027 uses for `classified-sink-without-threat`; the generator reads it to show
  the sink and SHALL NOT use it to produce a finding. A sink reached by none reads `none`.
- The `Acceptance` cell SHALL list, for each listed threat, each `risk` that `arises_from` it as
  `<risk id>: <decision>, <decided_by>`, reading `not stated` for an absent `decided_by`. It SHALL NOT compute or show a score.
- A carried classification SHALL be shown as its id with its `regulated_categories` as written; a connection carrying none
  reads `none`. A connection crossing no boundary reads `none` in the boundary cell; otherwise the cell shows the boundary id
  and its `trust_level` as written.
- `GAP:` rows and gaps SHALL come only from the three checks' findings at their resolved severity. The generator SHALL NOT
  recompute a check. A check a project sets to `off` yields no row and no gap, as quire-rs FR-096 drops its denials.
- A finding at severity `warning` SHALL yield its `GAP:` row, shown with `(warning)`, and its gap, so a document with such a
  finding is `incomplete`; a finding at `error` is shown with `(error)`. The generator SHALL NOT drop or soften a warning.
- An element covered, or a threat considered, by two models SHALL get its `GAP:` row in each model's section and one gap in
  `## Gaps`.
- Each `crossing-without-threat` finding SHALL produce one `crossing_without_threat` gap naming the connection; each
  `classified-sink-without-threat` finding one `classified_sink_without_threat` gap naming the sink; each
  `threat-without-treatment` finding one `threat_without_treatment` gap naming the threat. When no `threat_model` is in the
  selection the document SHALL hold no model section and one document-level `no_threat_model` gap.
- The generator SHALL count only committed `threat_model` and `threat` documents; a draft outside the selected corpus is not
  read and is not a model or a threat (spec-objects-security FR-026).
- The document is committed and checked like other dossier documents: `check` compares its bytes, `export` requires it current.
  Two generations over the same records SHALL be byte-identical.
- If a source reader reports an unreadable record, the command SHALL surface a source failure under FR-191 instead of
  treating the record as absent.
- The document SHALL NOT claim a finding the source did not report.
- The document SHALL NOT state that a system is secure, compliant or adequately modelled.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-202-AC-1 | A fixture with a `threat_model` whose `scopes` edges target two connections, one carrying `CLASS-PHI` across `BOUND-NET` with `THREAT-1` threatening it, mitigated by `CONTROL-1` and with an accepted risk, lists both connections in `### Covered crossings` with the carried classification, the boundary and trust level, `THREAT-1 (<stride_category>)`, the control and its safeguards, and `RISK-1: accepted, A. Person`. | Test (TC-601) |
| FR-202-AC-2 | Removing `THREAT-1`'s `threatens` edge yields a `GAP: crossing-without-threat CONN-1` row under `### Open findings` and one `crossing_without_threat` gap naming `CONN-1`; both appear in `--json` in the same order. | Test (TC-602) |
| FR-202-AC-3 | A considered threat with no control and no accepted risk yields `GAP: threat-without-treatment` and one `threat_without_treatment` gap; with a risk `decision: undecided` it still does, and with `decision: accepted` it does not. | Test (TC-603) |
| FR-202-AC-4 | A covered sink reached by a classification and with no threat yields `GAP: classified-sink-without-threat` and one `classified_sink_without_threat` gap naming the sink, and the sink is listed under `### Covered sinks`. | Test (TC-604) |
| FR-202-AC-5 | A selection with no `threat_model` writes `DOS-TM.md` with no model section, one `no_threat_model` gap, and status `incomplete`; the three checks produce no finding. | Test (TC-605) |
| FR-202-AC-6 | A fully treated fixture reads `none` under `### Open findings`, `Gaps: none`, and status `complete`. | Test (TC-606) |
| FR-202-AC-7 | Two `generate` runs over the same records produce byte-identical Markdown and `--json`; `check` after an unedited run reports `matches`, and after a hand-edited line reports `differs`. | Test (TC-607) |
| FR-202-AC-8 | The document holds no threat, STRIDE class, `decision` or `decided_by` value that does not appear in a record, and no score, rank or adequacy statement; a risk with no `decided_by` shows `not stated`. | Test (TC-608) |
| FR-202-AC-9 | An unreadable `threat` record makes `generate` exit 3 naming the source and leaves every existing file unchanged. | Test (TC-609) |
| FR-202-AC-10 | A connection covered by two models and lacking a threat gets a `GAP:` row in both sections and one `crossing_without_threat` gap; a considered threat with no treatment gets its `GAP:` row in the section of the model that considers it. | Test (TC-610) |
| FR-202-AC-11 | A `threat_model` with `scopes` edges to a `part`, a `trust_boundary` and a `connection` lists the part and the boundary by id under `### Scope`, in that order, and adds a crossing row only for the connection; a `covers` edge does not include its target in scope. | Test (TC-611) |
| FR-202-AC-12 | With `trace:crossing-without-threat` set to `off`, a connection lacking a threat yields no `GAP:` row and no gap; with it set to `warning`, the row reads `(warning)`, the gap is present and status is `incomplete`. | Test (TC-612) |
| FR-202-AC-13 | A threat considered by two models and lacking treatment gets its `GAP:` row in each model's section and one `threat_without_treatment` gap. | Test (TC-613) |
| FR-202-AC-14 | A fixture with two connections, two sinks, three findings of different checks and two part ids in non-sorted source order yields rows, entries, findings and `## Gaps` in the Ordering above. A selection with no `threat_model` yields one `no_threat_model` document gap and no findings. | Test (TC-614) |
| FR-202-AC-15 | A sink carrying `CLASS-PHI` and one reached by a connection that `delivers_to` it and carries `CLASS-PII` each list that classification; a sink with a control that `protects` it and a control that `mitigates` its threat lists each once with safeguards, and a control doing both is listed once. | Test (TC-615) |
| FR-202-AC-16 | A `threat_model` or `threat` file outside the selected corpus is not read: it yields no model section and no threat cell entry. | Test (TC-616) |

## Dependencies

- **Upstream**: [FR-191](./FR-191-dossier-command.md); spec-artifacts-process FR-018 (the `threat-model` `document_kind`);
  spec-objects-security FR-025 (`protects` and `safeguards`), FR-026 (`scopes`) and FR-027; spec-objects-architecture
  `connection`, `data_sink`, `carries`, `crosses` and `delivers_to` (PLAT-1532, PLAT-1440); the `covers` verb (PLAT-1657); the quire-rs
  relation report and `flow_rules` (FR-096; PLAT-1537, PLAT-1538), which must be delivered before the checks evaluate.
- **Downstream**: PLAT-1543.
