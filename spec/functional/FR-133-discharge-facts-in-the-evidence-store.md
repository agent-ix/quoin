---
id: FR-133
title: "Discharge facts in the evidence store and inward-completeness report"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-030"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "extends"
---

# FR-133: Discharge facts in the evidence store and inward-completeness report

## Description

Quoin SHALL persist the attested discharge facts of FR-046 in the evidence
store and SHALL derive the discharge partition and an inward-completeness
report from a live `clause-binding-v1` report plus the stored facts.

Inward completeness answers one question: is every clause that binds me
discharged by something I have. It is stated as populations that add up,
`discharged + Σ dispositions + open == binding`, with the undischarged
remainder named clause by clause. It is never a bare score.

## Rationale

FR-046 partitions binding clauses from facts supplied on the command line, so
nothing a reviewer attested survives the invocation and no view can read the
result. The evidence store (FR-030) is the artifact of record for what was
verified; a discharge attestation is the same kind of fact.

## Behavior

- **Facts are persisted, obligations are re-derived.** The store holds the
  attested fact and the clause set key it was made against. It holds no clause
  list, force, expected output or applicability: which clauses bind is decided
  by Quire from the clause set and the declared context, and the binding report
  is supplied live on every read (FR-030, governing principle).
- `evidence.record_discharge` SHALL persist a disposition fact's
  `decisionExpiresAt`, when present for `temporary_exception` under FR-046, as
  part of the fact unchanged. At read time,
  `asOf >= decisionExpiresAt` leaves that clause open even if the attestation's
  `expiresAt` is later.
- A fact is stored at
  `discharge/<authority>/<id>/<version>/<clause>.json`, one file per
  (clause set, clause). Each of the four ids is 1 to 64 bytes, is escaped so it
  cannot leave its directory, and is lowercased in the path, so a
  case-insensitive filesystem cannot alias two ids (FR-130's identifier rules).
  Recording a clause again replaces its fact, which is how an expired
  attestation is renewed; nothing merges two facts.
- Recording validates **every** fact before it writes any file: the FR-046
  predicates, unique clause ids in the list and in the binding report, the id
  bound, and case folding both inside the list and against facts already
  stored. A list that fails any check writes nothing. A different id that folds
  to an existing name is refused as `case-collision`, an id outside the bound
  as `invalid-identifier`.
- A binding report that names one clause twice is refused wherever FR-133
  consumes it, and by the FR-046 builder, so a clause is never counted twice.
- A fact for a clause that does not currently bind is stored and returned only
  as an unused `{clauseId, kind, reason}` by the report, because applicability
  can change with the declared context. `read_discharge_facts` returns the full
  stored facts for a caller that needs their content.
- Reading a clause set refuses more than 10000 stored files or a file over
  1 MiB, and refuses (never skips) a file that is not valid, belongs to another
  clause set, or sits at another clause's path.
- Currency is decided at read time from the fact's own attestation and, for a
  `temporary_exception`, its `decisionExpiresAt`. A fact whose attestation has
  expired or is not yet valid leaves its clause open with the reason visible
  (FR-046-AC-4); a temporary exception also leaves it open at or after
  `decisionExpiresAt` (FR-046-AC-8).
- `evidence.inward_completeness` takes the live binding report, an explicit
  `asOf` and the repository, and builds the discharge report itself from the
  binding report and the stored facts, so the populations add up by
  construction and the caller supplies no snapshot of the obligations.
- `quoin evidence record-discharge` stores facts. `quoin discharge --repo`
  reads the stored facts for the binding report's clause set instead of
  `--facts`, and `--inward` (which needs `--repo`) renders the
  inward-completeness report. A library or view caller reads the partition with
  `discharge_report_from_store`, the single entry point.

## Constraints

- The store SHALL NOT persist a clause list, a clause's force, expected outputs,
  applicability, a recording time or tool identity, or any count or rate derived
  from them.
- The inward-completeness report SHALL carry no score, percentage or ratio.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-133-AC-1 | A recorded fact is stored as the FR-046 fact beside its clause set key and a schema version, and the stored file holds no other clause of the binding report, no force or expected outputs, and no recording time or tool identity. | Test |
| FR-133-AC-2 | The discharge partition for a live binding report uses the stored facts of its clause set; a `temporary_exception` fact discharges only while its attestation is current and `asOf` is earlier than `decisionExpiresAt`, and leaves the clause open when `asOf` equals or exceeds `decisionExpiresAt`. | Test (TC-004) |
| FR-133-AC-3 | Recording a clause again replaces its fact. A list with a malformed fact, an unknown field, a bad instant, a duplicate clause, a clause id over 64 bytes, two ids that fold to one file name, an id folding onto a different stored id, or a binding report naming a clause twice is refused with a bad-request status (`invalid-identifier` or `case-collision` where those apply) and writes nothing. | Test |
| FR-133-AC-4 | A stored file that is not valid JSON, that names another clause set or clause than its path, or that is over 1 MiB, and a clause set holding more than 10000 files, are each an error naming the file or directory, never an open clause. | Test |
| FR-133-AC-5 | `evidence.inward_completeness` derives its report from the live binding report and the stored facts, and states `binding`, `discharged`, dispositions by decision and the open remainder by clause with its reason, such that `discharged + Σ dispositions + open == binding`; clause ids in the binding report are unique, so a binding report naming a clause twice is refused; an expired fact leaves its clause in the open remainder; unresolved applicability and unused facts are counted outside `binding`; the report carries no score. | Test |
| FR-133-AC-6 | `evidence.record_discharge`, `evidence.discharge_report` and `evidence.inward_completeness` answer through the production runtime with absolute store paths, each refuses a request over its ceiling before reading it, and `evidence.discharge_report` and `evidence.inward_completeness` each exit as a bad request when given a doubled binding clause. | Test |
| FR-133-AC-7 | `quoin evidence record-discharge` followed by `quoin discharge --repo ... --inward` reports the recorded fact as discharged and the remaining clause as open, and `--inward` refuses a doubled binding clause. | Test |

## Dependencies

- FR-030 supplies the store, its governing principle and the tool-version rule.
- FR-046 supplies the fact predicates, the partition and the report this
  requirement persists and summarises.
