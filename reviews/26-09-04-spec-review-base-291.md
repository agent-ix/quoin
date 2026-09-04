---
id: SR-140
title: "Base review of the advisory corpus measurement contracts"
type: SpecReview
analysis: base
scope: "US-022; FR-084..FR-092; NFR-021..NFR-023; TC-1500..TC-1565"
review_set: base
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "reviews"
  - target: "ix://agent-ix/quoin/TM-001"
    type: "references"
---

# SR-140: Base review of the advisory corpus measurement contracts

## Summary

The base checklist was run over the artifacts added by commit `2e5d704` for
agent-ix/quoin#291: one use case, nine functional requirements, three
non-functional requirements, and 66 matrix rows. Nothing was edited; this review
reports only.

Structurally the set is in good shape. Identifiers are well formed and unique
(`US-022`, `FR-084`..`FR-092`, `NFR-021`..`NFR-023`, `TC-1500`..`TC-1565`, with
`{PARENT}-AC-N` and `{PARENT}-CON-N` throughout), the three index files and
`spec/spec.md` were all updated, every requirement traces to `US-022`, every
requirement states inputs, outputs, behavior, constraints and criteria, and
every acceptance criterion of every FR and NFR has at least one matrix row.
Every new row carries Type, Priority, Trace and a `🚧` status with a reason and
a tracking issue, which is what FR-082's own rule asks for. The advisory,
read-only and never-weaken obligations of the ticket are each carried by an
explicit requirement (FR-092, FR-091, FR-089), and the "check passed" versus
"check could not run" distinction the ticket insists on is stated as a first
class outcome in FR-087, FR-090 and FR-091.

Two findings are high. The `unsupported-representation` token is used as a
non-failure outcome in FR-088 and as a failure class in FR-089, and no
requirement says how such a document is counted in a rate — the exact class of
ambiguity that produced the withdrawn figures this campaign exists to prevent.
And nothing in the set binds the "declared module set" to the nine completed
modules named in the ticket, so a run measuring a single module satisfies every
criterion authored here while leaving the campaign's exit criterion unmet.

The remaining findings are seven mediums — three counting or exit-status
contradictions between requirements, three matrix-coverage gaps, and one
unmeasurable performance environment — and one low.

Verdict: **changes requested**. The set is not ready to enter planning until
FND-1400 and FND-1401 are resolved; the mediums are all resolvable inside the
authored requirements without new scope.

## Checklist Results

| Gate | Result |
| --- | --- |
| ID format and uniqueness (US/FR/NFR/TC/AC/CON) | Pass — no duplicates, no gaps within the allocated blocks |
| Use-case quality | Pass with note — `US-022` follows this repo's illustrative `EX-n` convention rather than binding `AC-n`, matching US-013..US-021 |
| Functional requirement quality | Pass — inputs, outputs, behavior, constraints, criteria and dependencies present on all nine |
| Coverage: every AC has at least one TC | Pass for all 58 acceptance criteria |
| Coverage: every CON has at least one TC | Fail — `FR-091-CON-1` is untraced (FND-1406) |
| Error-path and edge-case coverage | Pass — refusal, unreadable, unresolved, contested-type, zero-denominator and both-representation paths all carry rows |
| Cross-referencing and link integrity | Pass — relative links resolve; `FR-074` referenced by `FR-088` exists |
| Matrix coverage sections updated | Fail — the FR, NFR and use-case coverage tables were not extended (FND-1405) |
| Terminology consistency | Fail — outcome vocabulary diverges across FR-087, FR-088, FR-089 and FR-090 (FND-1400, FND-1403) |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-1400 | high | `unsupported-representation` is a non-failure outcome in FR-088 and a failure class in FR-089, and FR-090 excludes only `could-not-run` and `not-applicable` from rates, so how such a document counts in the representation rate is undefined. | FR-088 Behavior; FR-088-CON-1; FR-089 Description; FR-090 Behavior; TC-1530, TC-1531 | wrong-requirement |
| FND-1401 | high | Nothing binds the declared module set to the completed modules of the campaign — FR-085 takes the set as an unconstrained input, so a run resolving one module satisfies every criterion while the ticket's "every completed module schema" stays unmet; US-022 says nine modules plus a tenth, FR-088 says nine of ten, and no requirement names the set. | FR-085 Inputs; US-022 Context; FR-088 Rationale; agent-ix/quoin#291 acceptance criteria | missing-requirement |
| FND-1402 | medium | Exit-status contradiction: FR-085 records an unresolvable revision as `unresolved` and continues, while FR-092-AC-6 requires a non-zero exit when the declared module set cannot be resolved; the boundary between one unresolved module and an unresolvable set is unstated. | FR-085 Behavior; FR-092-AC-6; FR-092-CON-2; TC-1509, TC-1556 | wrong-requirement |
| FND-1403 | medium | FR-091 requires every failure covered by a tool-defect entry to be classified `tool-defect` in the FR-089 partition, but the same requirement reports those documents `could-not-run`, which FR-087-CON-1 removes from the rate and which FR-089 does not define as a failure; the partition's input set is ambiguous. | FR-091 Behavior; FR-089 Inputs; FR-087-CON-1; TC-1546, TC-1547 | wrong-requirement |
| FND-1404 | medium | FR-090-AC-7 requires a by-type partition to publish a rate at denominator zero, and no requirement says what a rate over an empty denominator prints; FR-090 defines the small-denominator case only down to one. | FR-090 Behavior; FR-090-AC-6; FR-090-AC-7; FR-090-CON-2; TC-1544 | missing-requirement |
| FND-1405 | medium | The matrix's own coverage sections were not extended: FR-084..FR-092 are absent from the Functional Requirements table, NFR-021..NFR-023 from the Non-Functional Requirements table, and US-022 from Use Case Coverage — so no row realises US-022-EX-1..EX-4, unlike US-020 and US-021. | spec/matrix.md sections "Functional Requirements", "Non-Functional Requirements", "Use Case Coverage"; US-022 Acceptance Examples | missing-requirement |
| FND-1406 | medium | `FR-091-CON-1` declares verification method Inspection and appears in no matrix row and in no entry of the matrix's "Tracking-tag coverage" list of criteria verified without a test, so a citable tool-defect entry is asserted and never checked; every other CON in the set is traced. | FR-091 Constraints; spec/matrix.md "Tracking-tag coverage"; TC-1545..TC-1550 | missing-requirement |
| FND-1407 | medium | NFR-022's 15-minute threshold names only "a developer workstation" with no reference machine or corpus size, its metric table targets 5 minutes against a 15-minute statement, and TC-1560/TC-1561 introduce a `Benchmark` test type this matrix does not otherwise use or declare — so the threshold is not reproducibly verifiable. | NFR-022 Statement; NFR-022 Measurement and Evaluation; TC-1560, TC-1561 | correct-requirement-no-evidence |
| FND-1408 | medium | FR-084 defines the governed corpus as any directory under a declared workspace root carrying `.git/` and `spec/`, with no link to the governed corpus of the ticket's dependency (agent-ix/quire-rs#385), so the population is whatever a given machine has cloned — which NFR-021's "on any machine" reproducibility claim cannot then hold. | FR-084 Description; FR-084 Behavior; NFR-021 Scope; agent-ix/quoin#291 Dependencies | missing-requirement |
| FND-1409 | low | FR-088's declared dependencies are incomplete and asymmetric: it consumes the resolved module vocabulary of FR-085 and the measured documents of FR-086 but declares only FR-074 upstream, and it declares no downstream FR-089 although FR-087 does. | FR-088 relationships; FR-088 Dependencies; FR-089 Dependencies | missing-requirement |

## Recommendation

Resolve FND-1400 by closing FR-088's outcome vocabulary the way FR-086 and
FR-089 close theirs — an explicit exhaustive, mutually exclusive set with a
stated rate treatment for each member — and by separating the FR-089 class name
from the FR-088 outcome name if they are not the same thing. Resolve FND-1401 by
naming the completed module set the campaign measures, either in FR-085 or in a
declared input the report must pin. The remaining mediums are edits inside the
authored requirements and matrix rows; FND-1409 is a dependency-declaration fix.

No spec artifact was modified by this review.
