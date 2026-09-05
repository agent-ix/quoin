---
id: SR-142
title: "EARS conformance review of the quoin#291 corpus measurement requirements"
type: SpecReview
analysis: ears-conformance
scope: "FR-084..FR-092 and NFR-021..NFR-023 (US-022 as context)"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-084"
    type: "reviews"
  - target: "ix://agent-ix/quoin/NFR-021"
    type: "reviews"
---

# SR-142: EARS conformance review of the quoin#291 corpus measurement requirements

## Summary

Twelve requirement-bearing documents were reviewed — FR-084..FR-092 and
NFR-021..NFR-023, the artifacts added by commit `2e5d704` for `agent-ix/quoin#291`.
The deterministic grammar engine (`quire 0.31.0`, engine `0.46.0`) reports **zero**
findings of any class against all twelve: no `non-singular`, `vague-response`,
`missing-subject`, `non-canonical-trigger`, `unclassifiable`,
`quality:agentless-passive` or `quality:ambiguous-term`. Every warning in the run
belongs to a pre-existing quoin requirement and is out of scope here.

Ten findings remain from the semantic pass the engine cannot make. One is high:
FR-088 declares a three-value outcome vocabulary in its Description and then emits
two further outcomes in its Behavior, one of which FR-089 simultaneously treats as
a failure class — so what a `unsupported-representation` document does to a
published rate is not stated anywhere. The rest are response-clause defects: two
statements conflate two obligations, two leave a response undefined at a boundary
their own acceptance criteria exercise, and four are keyword or agency choices that
do not change what gets built. US-022 is a user story (`As a / I want / So that`)
and is out of EARS scope; it was read for context only.

## Verdict

CONDITIONAL. The set is grammar-clean by the engine and the requirement statements
are, with one exception, singular, subject-bearing and concretely responded. FND-1420
should be resolved before the requirements are tasked, because the outcome vocabulary
it names decides a published denominator. FND-1421..FND-1424 are worth fixing while
the statements are still cheap to edit; FND-1425..FND-1429 are advisory.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-1420 | high | FR-088's Description declares outcomes `pass`/`fail`/`could-not-run`, but its Behavior emits `unsupported-representation` and `not-applicable`; FR-089 lists `unsupported-representation` as a failure class, contradicting "rather than `fail`", and no statement says whether such a document enters a rate's denominator. | FR-088, FR-089, FR-090 |
| FND-1421 | medium | FR-092's Description carries two obligations in one statement — the success exit status, and write confinement as a trailing participial clause ("writing no file outside its declared output directory") that is not a normative `SHALL` response. | FR-092 |
| FND-1422 | medium | FR-092-AC-6 requires a non-zero exit when the declared module set cannot be resolved, while FR-085 requires an unresolved revision to be recorded and the run to continue; no statement defines the boundary between the two, so the exit-status response is unverifiable. | FR-092, FR-085 |
| FND-1423 | medium | NFR-022's Statement conflates a timing obligation and a read-only-open obligation, joined by `while` used as a plain conjunction rather than the EARS state keyword; split into two statements so each maps to one acceptance criterion. | NFR-022 |
| FND-1424 | medium | FR-090 states the response only for a partition whose denominator is one, while FR-090-AC-7 exercises a declared type with a denominator of zero; the response for an undefined rate is not stated. | FR-090 |
| FND-1425 | low | NFR-022's 15-minute threshold is qualified by "on a developer workstation", an unnamed reference machine, so the target is not reproducibly verifiable; name the machine class or the measured configuration. | NFR-022 |
| FND-1426 | low | `Where …` is used as a state/condition trigger in FR-085, FR-088, FR-090 and FR-091, whereas EARS reserves `Where` for optional features; use `While …` for a state or `If … then …` for a condition. | FR-085, FR-088, FR-090, FR-091 |
| FND-1427 | low | FR-089's three `only when` clauses and FR-091's "SHALL refuse a ledger entry that carries no repository and issue number" state necessary conditions rather than triggered responses; recast as unwanted-condition statements (`If … then the measurement SHALL NOT …`). | FR-089, FR-091 |
| FND-1428 | low | NFR-023's Statement is agentless — "Every numeric figure … SHALL be traceable" names the artifact, not the system obliged to make it so; name the report generator as the subject. | NFR-023 |
| FND-1429 | low | Rationale prose is embedded in requirement statements: two free paragraphs inside FR-086's Behavior, a justification sentence on FR-087's last Behavior bullet, and `so that …` tails on the FR-084/FR-085/FR-087/FR-090 Descriptions; move them to Rationale so each clause is a bare EARS statement. | FR-084, FR-085, FR-086, FR-087, FR-090 |

## Evidence

`quire validate --scope /home/peter/dev/quoin/.worktrees/291-corpus-measurement
"spec/**/*.md" --summary` reports `218/246 docs grammar-clean (88%); 55 grammar
finding(s)`. Filtering that output to the twelve in-scope documents returns no
lines: every `ears:` and `quality:` warning in the run is attributed to a
pre-existing quoin requirement (FR-001, FR-003, FR-004, FR-006, FR-008, FR-009,
FR-011, FR-012, FR-015, FR-017, FR-019..FR-022, FR-044, FR-045, FR-068, FR-069,
NFR-001, NFR-004, NFR-009, StR-006). Findings FND-1420..FND-1429 are the semantic
pass over the statement text of FR-084..FR-092 and NFR-021..NFR-023.
