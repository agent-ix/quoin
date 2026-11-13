---
id: NFR-037
title: "Source profile evaluation is deterministic, offline, and never decides compliance"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-180"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-181"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-182"
    type: "constrains"
---

# NFR-037: Source profile evaluation is deterministic, offline, and never decides compliance

## Statement

`quoin profile evaluate` SHALL write the same bytes for the same installed modules, selection files and context.

It SHALL make no network request and state no verdict that a product complies with a source.

## Scope

- Applies to: the effective profile and the evaluation report.
- Out of scope: the clause-set content and the engine's applicability semantics.

## Rationale

Which sources apply is a statement about the product. A tool that picks a profile or reports "compliant" turns a
recorded fact into an assessment nobody made.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Differing bytes between two runs over the same inputs | 0 | 0 | Test |
| Network requests per run | 0 | 0 | Test |
| Verdict, score or percentage fields in the report | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-037-AC-1 | Two runs over identical inputs produce byte-identical stdout and `--out-dir` files. | Test |
| NFR-037-AC-2 | A run with networking disabled gives the same output as a run with it enabled. | Test |
| NFR-037-AC-3 | The report schema has no field named or typed as a verdict, score, grade or percentage. | Test |

## Verification

All three criteria are tests over fixture modules.

## Dependencies

- **Upstream**: [FR-180](../functional/FR-180-source-profile-selection.md), [FR-181](../functional/FR-181-source-profile-evaluation.md), [FR-182](../functional/FR-182-local-pack-joins-a-profile.md)
