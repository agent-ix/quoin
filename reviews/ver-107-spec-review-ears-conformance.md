---
id: SR-20261113155332
title: "EARS conformance review of the FR-116 analysis-act producer definition (FR-184, FR-153 amendment)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #862 at 404cb32c: spec/functional/FR-184-analysis-act-producer-definition.md, spec/functional/FR-153-analysis-result-counting-by-disposition.md (amended Behavior rule)"
---

# EARS conformance review of the FR-116 analysis-act producer definition (FR-184, FR-153 amendment)

## Summary

Ticket: VER-107. I checked each normative statement in FR-184 and the amended FR-153 Behavior rule against
the EARS patterns.

The new FR-153 rule is a well-formed unwanted-behaviour statement: "If no FR-093 assessment exists, or
the run-package step has no CI checkout SHA, then the result SHALL be `unevaluated` ...". The two FR-184
Default-`unevaluated` bullets are well formed too.

Two statements in FR-184 are not.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "The bridge MAY emit a record claim for an obligation only when **all** of these hold" is a permission with a restriction attached, not an EARS requirement. As written it obliges nothing. State it as "The bridge SHALL NOT emit a record claim for an obligation unless all of these hold". | spec/functional/FR-184-analysis-act-producer-definition.md:105-106 |
| FND-002 | low | The rules the definition exists to fix are declarative prose with no SHALL: CI-only association, the source revision being the CI checkout SHA, the signer as actor, and freshness as revision plus supersession only. Only the two Default-`unevaluated` bullets are normative. Add one SHALL statement per rule, for example "The qualifier SHALL take the performed revision from the CI checkout SHA of the `run-package` step", so each rule traces to an AC. | spec/functional/FR-184-analysis-act-producer-definition.md:81-101, :115-116 |

## Verdict

Minor. Both findings are wording; the meaning is unambiguous from context. Fix them in the same round as
the base findings.

Examined (role examined): the What runs exclusion, Execution association, Source revision, Tool and
configuration, Actor authority, the positive-decision statement and its premises 1-5, Default
`unevaluated` (2 bullets), CON-1, CON-2 and the amended FR-153 Behavior rule.
