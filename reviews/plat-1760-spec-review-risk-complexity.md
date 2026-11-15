---
id: SR-20261115183525
title: "Risk and complexity — PLAT-1760 FR-115 typed stale-evidence cause"
type: SpecReview
analysis: risk-complexity
scope: "agent-ix/quoin; spec/functional/FR-115-evidence-backed-test-matrix.md; PLAT-1760"
review_set: subset
---

## Summary

Ticket: PLAT-1760. Risk register for the FR-115 change in PR #931.

| Req | Tech Risk | Volatility | Drivers | Mitigation |
|-----|-----------|------------|---------|------------|
| FR-115 (typed cause, AC-17..19) | Low | Low | One optional field on a shared wire type, set by two producers that exist; no concurrency, security or performance bound | Unit tests at both producers plus one end-to-end `quoin matrix --json` fixture |

## Verdict

**CONDITIONAL**, with one low finding. The change is small and adds only
information. The one risk is to wire bytes, not to behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "The cause is additive: it changes no `evidence_status` value ... or any verdict" covers semantics only. Adding `cause` changes the canonical JSON bytes of every `stale-evidence` finding the auditor emits, and so of `quoin evidence audit --json`, `quoin matrix --json`, and any golden or difftest corpus built from them (FR-115-AC-10's boundary corpus, `quoin-assurance` goldens). Nothing tells implementers to expect those goldens to change. | spec/functional/FR-115-evidence-backed-test-matrix.md:226-228 |

## Top hazards

1. Ambiguous AC-17 fixture (integrity FND-001): a test built on an absent
   symbol would expect `suspect`, not `stale`.
2. Unknown or misplaced `cause` on supplied input (code-review FND-002,
   object-review FND-001).

## Failure-domain gaps

The failure-domain lens was not selected for this run. The open questions
are in the code-review and object-review artifacts.

## Dispositions

Round 1, fix commit 0883d0c3.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-115 now states the canonical-JSON compatibility change: stale-evidence bytes change because of the new cause key, while other finding bytes are unchanged. |
