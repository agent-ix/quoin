---
id: SR-165
title: "Failure-domain review of FR-115 evidence-backed test matrix (quoin#646)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin@b2b8df657fda0fd86caf75aff45436174223fdac; spec/functional/FR-115-evidence-backed-test-matrix.md, spec/functional/FR-021-launch-ix-flow-runs.md, spec/matrix.md, spec/log.md, spec/functional/index.md (read for contract: rust/crates/quoin-auditor/src/audit/{mod,ladder}.rs, rust/crates/quoin-finding-types/src/report.rs, agent-ix/quire-rs@af5ec21 spec/functional/FR-050 AC-47..51)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-115"
    type: "reviews"
---

# Failure-domain review: FR-115 (PR quoin#646)

## Summary

Ticket: PLAT-1080. This review walked all 12 auditor `FindingKind`s, the
`healthy`/`unevaluated` split and the empty and edge inputs through
FR-115's mapping. The mapping is total: every kind resolves, because AC-4
ends in "else", and `requirement_of` is total. `healthy` is disjoint from
`findings` and `unevaluated` by construction (`audit/mod.rs` `clean`), so
putting `bound` first is safe on genuine auditor output.

Kind → status as specified:
- `suspect`: `suspect-link`, `mocked-confirmation`, `insufficient-independence`.
- `stale`: `stale-evidence`.
- `undischarged`: `undischarged`, `vacuous-evidence`, `unknown-method`,
  `method-conformance`, `combinatorial-gap`, `insufficient-multiplicity`,
  `insufficient-mutation-score` and `unmeasured-mutation-score`, plus any
  future kind. `undischarged` and `vacuous-evidence` get there only through
  "else".

The gaps are in states the vocabulary collapses together, and in input
shapes the spec leaves undefined.

## Findings

| ID      | Severity | Summary | Refs | Escape Cause |
| ------- | -------- | ------- | ---- | ------------ |
| FND-001 | medium | An `unevaluated` entry maps to `undischarged`. An obligation with a current, passing, clean run whose suite simply lacks an `inspect-mocks` record at HEAD therefore renders the same as one with no test at all. Until mock inspection runs everywhere, that describes most populated rows. As base FND-002 notes, no `evidence_detail` is specified that would tell the two apart. AC-5's `bound` row also silently requires the fixture to carry a mock inspection at HEAD; without one it reads `undischarged` and the AC fails for a reason the AC never names. Keep the five statuses, but require `evidence_detail` to name the unevaluated check and state the AC-5 fixture precondition. | spec/functional/FR-115-evidence-backed-test-matrix.md:59, spec/functional/FR-115-evidence-backed-test-matrix.md:82, spec/functional/FR-115-evidence-backed-test-matrix.md:178, rust/crates/quoin-auditor/src/audit/mod.rs:171 | wrong-requirement |
| FND-002 | low | The Description table presents an enumeration of "the auditor's remaining findings" that omits `vacuous-evidence` and the `undischarged` kind itself, so a reader takes a partial list as exhaustive. Separately, `behind_head` does not stop the ladder, so a run behind HEAD in which every symbol was skipped carries `stale-evidence` plus `vacuous-evidence` and renders `stale`. That hides the high-severity vacuity behind a medium one. The `stale` row also says the evidence is "recorded as out of date", but `stale-evidence` is also minted, at high severity, for a binding whose suite has NO recorded run (`unrecorded_evidence`). | spec/functional/FR-115-evidence-backed-test-matrix.md:57, spec/functional/FR-115-evidence-backed-test-matrix.md:59, spec/functional/FR-115-evidence-backed-test-matrix.md:89-93, rust/crates/quoin-auditor/src/audit/ladder.rs:228-258 | wrong-requirement |
| FND-003 | low | `audit` arrives on stdin and is not validated against itself. An id listed in both `healthy` and a `suspect-link` finding (a hand-edited or merged report) renders `bound`. That is the opposite of the stated suspect > stale precedence, and nothing refuses the contradiction. Either refuse an id that appears in both `healthy` and `findings`/`unevaluated`, or state that `healthy` is trusted verbatim. | spec/functional/FR-115-evidence-backed-test-matrix.md:78, spec/functional/FR-115-evidence-backed-test-matrix.md:124-126 | missing-requirement |
| FND-004 | low | Input and output shape edges are undefined. (a) quire-rs FR-050-AC-51 OMITS `coverage_matrix` when a module declares no `obligations:` source, but `coverage` is a required request field (AC-2 → `BadRequest`), and the `quoin matrix` behaviour for that repository (refuse, or `reason`) is unstated. (b) FR-050-AC-48 makes `method` present only "when the obligation states one", but the output always lists `method` without saying absent vs null. (c) `bindings` is `Vec<Binding>` while the store holds `BindingsFile {schema_version, bindings}`, and which one is on the wire is unstated. | spec/functional/FR-115-evidence-backed-test-matrix.md:121-123, spec/functional/FR-115-evidence-backed-test-matrix.md:130-139 | missing-requirement |

## Verdict

One MEDIUM and three LOW findings. None blocks on its own, but FND-001 should
be fixed in the same round as base FND-002, because both concern
`evidence_detail`.

## New findings (disposition pass 1)

Reviewed at agent-ix/quoin@e609508fcf96f3b97d5ec817c5ff051e56653c34.

| ID      | Severity | Summary | Refs | Escape Cause |
| ------- | -------- | ------- | ---- | ------------ |
| FND-005 | low | AC-11 introduces a new `quoin matrix --strict` flag (non-zero exit on an empty population) that appears nowhere in Behavior or Inputs. The Behavior section, which enumerates the CLI surface, therefore under-states it. Neither the HEAD-unresolved refusal (Behavior, AC-14) nor the `ContradictoryAudit` refusal (AC-5) names its exit class. An implementer can pass AC-14 with any non-zero exit and any message. | spec/functional/FR-115-evidence-backed-test-matrix.md:276, spec/functional/FR-115-evidence-backed-test-matrix.md:236-242, spec/functional/FR-115-evidence-backed-test-matrix.md:279 | missing-requirement |

## New findings (disposition pass 2)

Reviewed at agent-ix/quoin@3d058f9af6c53cc47f7c3b8363852046e17f1e59.

| ID      | Severity | Summary | Refs | Escape Cause |
| ------- | -------- | ------- | ---- | ------------ |
| FND-006 | medium | The FND-005 fix names the wrong gate on both axes. AC-11 says "the gates over the same data are `quire coverage --strict` (the static axis) and `quoin evidence audit --ratchet` (the evidence axis)". The static gate over the computed `coverage_matrix` is `quire matrix --strict` (quire-cli FR-026 §G and AC-10, merged in quire-cli#103), which exits 1 on any `untagged` or `tagged-by-ignored-test` criterion and on the zero-population state. `quire coverage --strict` gates the older unbacked-row and zero-trace-target report (FR-050-AC-14), not criterion statuses. On the evidence axis, `--ratchet` alone never exits non-zero: `quoin-cli/src/evidence/audit.rs:121` sets a non-zero outcome only under `--strict`, and `--ratchet` merely narrows the report to findings not in the baseline (FR-032:73, 81). An operator who wires up the two named commands gets two gates that gate nothing. Name `quire matrix --strict` and `quoin evidence audit --strict` (optionally with `--ratchet`). The evidence half matches the team leader's ruling as relayed, so the leader should correct the ruling too. | spec/functional/FR-115-evidence-backed-test-matrix.md:283, rust/crates/quoin-cli/src/evidence/audit.rs:121, spec/functional/FR-032-evidence-auditor.md:73-81 | wrong-requirement |

## Dispositions

Round 1, reviewed at agent-ix/quoin@e609508fcf96f3b97d5ec817c5ff051e56653c34.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e609508 |
| FND-002 | fixed | e609508 |
| FND-003 | fixed | e609508 |
| FND-004 | fixed | e609508 |

Round 2, reviewed at agent-ix/quoin@3d058f9af6c53cc47f7c3b8363852046e17f1e59.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | 3d058f9 |

Round 3, reviewed at agent-ix/quoin@6c0b5d0ece17df9659fcca18bc2139d9a9131efa.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-006 | fixed | 6c0b5d0 |
