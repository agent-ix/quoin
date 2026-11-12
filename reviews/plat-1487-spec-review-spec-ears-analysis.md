---
id: SR-20261112191021
title: "EARS analysis of FR-140 pack requirements mapping report in quoin PR 832"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin@f0a0122c; spec/functional/FR-140-pack-requirements-mapping-report.md"
review_set: subset
---

# EARS analysis of FR-140 pack requirements mapping report

## Summary

Ticket: PLAT-1487. Scoped to the requirement statements of FR-140 at f0a0122c. The
Description, the refusal bullets (Unknown pack, Not a pack, Load failure) and the Constraints
use SHALL with an EARS trigger. The other Behavior bullets are descriptive paragraphs.

## Verdict

**PASS WITH NITS.** One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Behavior bullets Sets, Binding, Matrix, Index, Markdown, JSON, Usage and Determinism carry no SHALL and bundle several obligations each, unlike FR-137's one-SHALL-per-bullet style | spec/functional/FR-140-pack-requirements-mapping-report.md:56-79,90-96 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-140, path: spec/functional/FR-140-pack-requirements-mapping-report.md, role: examined, excerpt: "Pack index. A table before the matrices, one entry per set in report order, with the set key, the row count, and the number of rows at each Compliance value."}
```
