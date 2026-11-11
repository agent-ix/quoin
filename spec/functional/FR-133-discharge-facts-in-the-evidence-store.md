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
  attested fact, the clause set key it was made against, the instant it was
  recorded and the exact version of the recording tool. It holds no clause
  list, force, expected output or applicability: which clauses bind is decided
  by Quire from the clause set and the declared context on every read
  (FR-030, governing principle).
- A fact is stored at
  `discharge/<authority>/<id>/<version>/<clause>.json`, one file per
  (clause set, clause), each path component encoded so it cannot escape its
  directory. Recording a clause again replaces its fact, which is how an
  expired attestation is renewed; nothing merges two facts.
- Recording validates the whole fact list with the FR-046 predicates first, so a
  refused list writes nothing. A fact for a clause that does not currently bind
  is stored and reported as unused when read, because applicability can change
  with the declared context.
- Currency is decided at read time. A stored fact whose attestation has expired
  or is not yet valid leaves its clause open with the reason visible (FR-046-AC-4).
- `quoin evidence record-discharge` stores facts. `quoin discharge --repo`
  reads the stored facts for the binding report's clause set instead of
  `--facts`, and `--inward` renders the inward-completeness report. A library
  caller reads the same partition with `discharge_report_from_store`.
- No config digest, source revision or environment is recorded: the record
  states the tool and its exact version only (FR-030-AC-17), and the store
  keeps no hash of anything (FR-044-AC-13).

## Constraints

- The store SHALL NOT persist a clause list, a clause's force, expected outputs,
  applicability, or any count or rate derived from them.
- The inward-completeness report SHALL carry no score, percentage or ratio.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-133-AC-1 | A recorded fact is stored with its clause set key, recording instant and recording tool and version, and the stored file holds no other clause of the binding report and no force or expected outputs. | Test (TC-2600) |
| FR-133-AC-2 | The discharge partition for a live binding report uses the stored facts of its clause set; the same stored fact discharges a clause before its attestation expires and leaves it open after. | Test (TC-2601) |
| FR-133-AC-3 | Recording a clause again replaces its stored fact, and a fact list the FR-046 predicates refuse (a duplicate clause, a malformed fact, a bad instant) writes nothing and exits as a bad request. | Test (TC-2602) |
| FR-133-AC-4 | A stored file that is not valid JSON, or whose clause set or clause differs from its path, is an error naming the file, never an open clause. | Test (TC-2603) |
| FR-133-AC-5 | The inward-completeness report states `binding`, `discharged`, dispositions by decision and the open remainder by clause with its reason, such that `discharged + Σ dispositions + open == binding`; unresolved applicability and unused facts are counted outside `binding`; the report carries no score. | Test (TC-2604) |
| FR-133-AC-6 | `evidence.record_discharge`, `evidence.discharge_report` and `assurance.inward_completeness` answer through the production runtime with absolute store paths, and each refuses a request over its ceiling before reading it. | Test (TC-2605) |
| FR-133-AC-7 | `quoin evidence record-discharge` followed by `quoin discharge --repo ... --inward` reports the recorded fact as discharged and the remaining clause as open. | Test (TC-2606) |

## Dependencies

- FR-030 supplies the store, its governing principle and the tool-version rule.
- FR-046 supplies the fact predicates, the partition and the report this
  requirement persists and summarises.
