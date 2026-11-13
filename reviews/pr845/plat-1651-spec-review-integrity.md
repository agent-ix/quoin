---
id: SR-20261113051915
title: "Integrity review of quoin risk advise (FR-145..FR-148, NFR-031)"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #845 (git diff origin/main...HEAD at 96566e29): spec/functional/FR-145..FR-148, spec/non-functional/NFR-031, spec/functional/index.md, spec/non-functional/index.md, spec/spec.md"
---

# Integrity review of quoin risk advise (FR-145..FR-148, NFR-031)

## Summary

Ticket: PLAT-1651. Completeness, consistency and atomicity of FR-145..148 and NFR-031,
cross-read against each other, quoin FR-136 and spec-objects-safety FR-101..107, FR-201..205 and
FR-301..305 on `origin/main`. The no-write, no-store and attribution rules agree across FR-145,
NFR-031 and safety FR-104; the epistemic-marker handling of FR-146 matches FR-102 exactly; the
post/pre-control non-comparison matches FR-204. Three internal contradictions were found.

Examined (all role examined): FR-145..FR-148 statements and all their criteria, NFR-031
statement and NFR-031-AC-1..AC-7. Context only: spec-objects-safety FR-101..107, FR-202..205,
FR-301..305 on origin/main, quoin FR-136, FR-144, StR-007, StR-009.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Contradicts FR-147, whose entry SHALL carry "the ids of its situation and hazards" (FR-147 line 36): the JSON cannot both hold the hazard and situation ids and hold none of them. Restate AC-7 as "holds no row-per-chain-pairing structure" or drop the second clause. | FR-145-AC-7, FR-147:36 |
| FND-002 | medium | FR-136 has `quoin anomaly refs` report `{ anomaly, kind, impact }`, not `{ anomaly, impact, status }`; status is not in that output, so "as quoin anomaly refs reports them" is false. Say the status is read from the anomaly record, or list `{ anomaly, kind, impact }` plus status. | FR-147:39, FR-136 Reverse references |
| FND-003 | medium | FR-145's exhaustive read list omits the retained evidence FR-147 needs for its second verification route, and FR-147 never says what "bound by retained evidence" reads. If it is a recorded run result (StR-009: a failing test discharges nothing), the route reads pass status, which FR-147 line 43 and the open-question-1 ruling forbid. Name the source in FR-145's read list and define bound as "a retained record names the criterion, whatever its outcome". | FR-145:37, FR-147:38,43 |
| FND-004 | low | The no_record group restates safety FR-303's post-control-estimate-without-decision (already in risk-summary's findings) without naming it; name the FR-303 token so the two reports read as one fact. | FR-148:38 |

## Verdict

Not mergeable: FND-001 is a direct contradiction between FR-145-AC-7 and FR-147 that no
implementation can satisfy.

## Dispositions

Round 1, against commit 9a653e9d.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
