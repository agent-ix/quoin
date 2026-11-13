---
id: SR-20261113055215
title: "EARS conformance review of analysis-result counting (FR-152, FR-153, NFR-033)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #849 (git diff origin/main...origin/ver-81-spec-retained-evidence-analysis at 480ba1c2): spec/functional/FR-152, spec/functional/FR-153, spec/non-functional/NFR-033"
---

# EARS conformance review of analysis-result counting (FR-152, FR-153, NFR-033)

## Summary

Ticket: VER-81. Each Behavior bullet and each Statement SHALL was checked against EARS patterns
(ubiquitous, event-driven, unwanted-behaviour "If ... then", state-driven). Most bullets conform and
are atomic. The defects are below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two unwanted-behaviour bullets have no scope: "If no link is supplied, then `quoin evidence record` SHALL store the entry with no `traceIds`" and "... SHALL create no binding". `junit` and `agent-eval` entries carry emitted trace ids and bind with no link, so as written these contradict FR-030 and FR-033. Add the precondition "for a `differential-report` report". | spec/functional/FR-152-analysis-run-entry-link-to-obligation.md:90-91 |
| FND-002 | low | One bullet holds two requirements: "If the linked entry's outcome is `fail`, then the result SHALL be `failed`; if it is `error`, then `errored`." Split it. | spec/functional/FR-153-analysis-result-counting-by-disposition.md:226 |
| FND-003 | low | A normative SHALL sits in Description prose instead of Behavior: "The bridge SHALL ask the FR-116 qualifier about an analysis entry with the obligation's declared method and a producer definition ...". Move it to Behavior. | spec/functional/FR-153-analysis-result-counting-by-disposition.md:198-200 |
| FND-004 | low | NFR-033's third Measurement row (obligations counted from a binding or a healthy audit without an FR-116 decision) measures a property its Statement does not state. The Statement has only no-inferred-binding and order-independence. Add the SHALL or move the row to FR-153. | spec/non-functional/NFR-033-analysis-counting-never-infers-a-binding.md:333-336 |

## Verdict

Mostly conformant. FND-001 needs fixing because the unscoped bullets contradict existing adapter
behaviour. The rest are wording.

Examined (role examined): all 13 FR-152 Behavior bullets, all 11 FR-153 Behavior bullets, the two
NFR-033 Statement SHALLs and the NFR-033 Measurement table.
