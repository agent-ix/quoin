---
id: SR-20261114002
title: "Dependency review of PLAT-1707 adapter protocol migration"
type: SpecReview
analysis: dependency
scope: "FR-151, FR-152, FR-184, TC-001, TC-002, spec/spec.md"
review_set: subset
---

## Summary

Dependency analysis for the changed FR-151/FR-152/FR-184 chain found an acyclic order. Upstream producer contracts and adapter infrastructure enable FR-151; FR-151 plus the store and recorder contracts enable FR-152; FR-116, FR-151/152/153, NFR-033 and FR-093 are prerequisites for the FR-184 qualification definition. AP-201 and AP-202 are active but do not apply to this adapter protocol migration.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | No dependency ordering defect or cycle found in the reviewed change. | FR-151; FR-152; FR-184 |

### Classification

| Requirement | Class | Rationale |
|-------------|-------|-----------|
| FR-151 | Feature | Reads and transcribes producer reports through the registered differential-report adapter. It depends on filament-analyze FR-005/FR-009 and existing adapter infrastructure FR-033/FR-069. |
| FR-152 | Feature | Records explicit obligation bindings from `--discharges`. It depends on FR-151, the FR-030 store, FR-033, FR-041, and the execution evidence contract in ADR-100. |
| FR-184 | Feature | Defines when analysis evidence qualifies for a positive decision. It depends on FR-116, FR-151/FR-152/FR-153, NFR-033, FR-093, and the referenced upstream producer/executor contracts. |

### Dependency Graph

```mermaid
graph TD
  FA[filament-analyze FR-005 and FR-009] --> FR151[FR-151 adapter]
  FR033[FR-033 adapter infrastructure] --> FR151
  FR069[FR-069 campaign adapters] --> FR151
  FR030[FR-030 evidence store] --> FR152[FR-152 explicit binding]
  FR041[FR-041 discharges option] --> FR152
  FR151 --> FR152
  FR151 --> FR184[FR-184 qualification]
  FR152 --> FR184
  FR153[FR-153 counting] --> FR184
  FR116[FR-116 owner-qualified evidence] --> FR184
  NFR033[NFR-033 no inferred binding] --> FR184
  FR093[FR-093 producer trust] --> FR184
```

### Topological Order

1. Producer contracts (filament-analyze FR-005/FR-009) and existing enablement (FR-030, FR-033, FR-041, FR-069, FR-093, FR-116, FR-153, NFR-033).
2. FR-151, once its producer and adapter prerequisites are available.
3. FR-152, after FR-151 and its store/recorder prerequisites.
4. FR-184, after FR-151/152/153 and its assurance prerequisites.

### Cycles

None detected.
