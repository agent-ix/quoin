---
id: NFR-011
title: "Bundle-scale commands stay within a stated time budget"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-037"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-031"
    type: "constrains"
---

# NFR-011: Bundle-scale commands stay within a stated time budget

## Statement

The Quoin-owned completeness phase SHALL process 250 documents within **5
seconds** under the workload stated below.

The in-process advisor phase SHALL process 10,000 derived obligations within
**1 second** under the workload stated below.

The in-process auditor phase SHALL process 10,000 derived obligations within
**1 second** under the workload stated below.

For each in-process phase, the median elapsed time SHALL grow by no more than
**3×** from 5,000 to 10,000 obligations under the workload stated below.

## Scope

- Applies to Quoin-owned work in `quoin completeness`, `quoin advise`, and
  `quoin evidence audit`.
- Operational context: a CI gate and an interactive terminal run.
- Excluded: generated-fixture setup and `quire` subprocess time, which the
  engine's own NFRs govern.
- Excluded from the in-process timer: Node/oclif startup. It is an independently
  measured boundary cost, not algorithmic work by the advisor or auditor.
- Excluded: `quoin matrix`, which only launches an `ix-flow` workflow and does
  not walk the bundle in Quoin. The workflow and agent own that elapsed time.

## Rationale

`quoin evidence audit` and `quoin completeness` are merge gates, and a gate
people wait on is a gate people route around. A measured budget turns an
order-of-magnitude regression into a failing test rather than "CI got slower".

The numbers are **budgets, not predictions**. Their purpose is that a change
making a bundle walk an order of magnitude slower fails a test instead of being
absorbed as "CI got slower". A single 250-row timing point did not constrain the
algorithm: a quadratic implementation still passed it comfortably. The
5,000→10,000 ratio exposes that shape, while the absolute ceiling catches a
uniform slowdown.

The filesystem constraint is narrower and directly observable: completeness
gets one pass over the document set and one read attempt per Markdown document.
Advisor and auditor cores accept already-derived obligations and evidence; they
do not own a document walk after Quire produces those inputs. The auditor
fixture intentionally binds all obligations to one large shared suite. That is
the adverse normal case for evidence lookup: a linear search of every run entry
for every obligation becomes quadratic.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Quoin-owned completeness work over 250 Markdown documents | < 2s | < 5s | Test (TC-290) |
| In-process advisor work over 10,000 derived obligations | < 500ms | < 1s | Test (TC-290) |
| In-process shared-suite auditor work over 10,000 obligations | < 500ms | < 1s | Test (TC-290) |
| Advisor/auditor median growth, 5,000→10,000 obligations | ≤ 2.5× | ≤ 3× | Test (TC-290) |
| Completeness passes/read attempts over the document set | 1 pass / 250 reads | 1 pass / 250 reads | Test (TC-290) |

## Verification

A generated bundle of 250 documents exercises the owned completeness phase. A
read observer counts the pass and document read attempts rather than inferring
them from timing, so a second pass fails even on hardware fast enough to stay
inside the budget. Generated payloads at 250, 1,000, 5,000 and 10,000
obligations separately exercise the advisor and auditor after the excluded
Quire extraction boundary.

Fixture creation occurs before each timer. In-process measurements use one
warm-up execution and the median of three samples. Each scale point is checked
against the absolute ceiling, and the 5,000→10,000 median ratio is checked
against the growth ceiling.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-011-AC-1 | The owned completeness phase processes a generated 250-document bundle in less than 5 seconds. | Test (TC-290) |
| NFR-011-AC-2 | The in-process advisor and shared-suite auditor each process 10,000 derived obligations/evidence records in less than 1 second, with no more than 3× median growth from 5,000 to 10,000. | Test (TC-290) |
| NFR-011-AC-3 | Completeness emits exactly one pass event and 250 document-read events for the generated bundle. | Test (TC-290) |
| NFR-011-AC-4 | The budget excludes Quire subprocess work and the delegated matrix workflow rather than attributing their time to Quoin. | Test (TC-290) |

## Dependencies

- **Upstream**: [FR-037](../functional/FR-037-declared-vocabulary-completeness.md),
  [FR-031](../functional/FR-031-catalog-driven-advisor.md), and
  [FR-032](../functional/FR-032-evidence-auditor.md) — the owned scale-sensitive phases.
- **Downstream**: [NFR-012](./NFR-012-ecosystem-compatibility.md), whose corpus-wide
  sweeps multiply this cost by the repository count.
