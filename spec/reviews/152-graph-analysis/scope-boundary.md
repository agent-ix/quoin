---
id: SR-088
title: "Scope and boundary review of issue 152 graph-analysis requirements"
type: SpecReview
analysis: scope-boundary
scope: "US-018, FR-062"
review_set: all
---

# Scope and boundary review of issue 152 graph-analysis requirements

## Summary

Quoin owns validation of its accepted export premises, joins to its evidence/auditor data, graph
analysis, and rendering. Quire owns the authoritative corpus/export; producers, Git, inherited
update checks, network, frontmatter parsing, portfolio policy, and assurance verdicts remain
outside this feature.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved during review: required `--export`, `--premises`, and source-bound `--audit` inputs make the Quire/Quoin/auditor handoff explicit; `--repo` may locate retained Quoin state but cannot invoke Quire, recompute an audit, or discover ambient module premises. | FR-062; quire-rs FR-067/FR-068 |

## External dependencies

| Dependency | Type | Assumed or Guaranteed | Contract |
| --- | --- | --- | --- |
| Quire assurance export | Offline JSON artifact | Guaranteed | quire-rs assurance-v1 schema, FR-067/FR-068, and IT-001 |
| Quoin evidence store | Local retained files | Guaranteed | FR-030 store readers and TC-1250/TC-1254/TC-1255 |
| Quoin auditor verdicts | Source-bound offline JSON artifact | Guaranteed | FR-032 and TC-1253/TC-1257 |

## Responsibility allocation

| Requirement | Owning Component | Class |
| --- | --- | --- |
| FR-062 import validation | Quoin Quire adapter | infrastructure |
| FR-062 graph projections | Quoin graph-analysis library | core |
| FR-062 command/rendering | Quoin CLI | core |
| FR-062 read-only/non-scoring rules | Quoin static boundary gate | cross-cutting |

Issue #281 is downstream portfolio presentation. Quoin #286 and `filament-ide-rs` are outside the
reviewed boundary.
