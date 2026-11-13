---
id: NFR-035
title: "The anomaly report states what people recorded and never decides acceptance"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-162"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-163"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-164"
    type: "constrains"
---

# NFR-035: The anomaly report states what people recorded and never decides acceptance

## Statement

`quoin anomaly report` SHALL print only values that a person wrote in an `Anomaly` document, values git records
about its commits, and the group and gate facts derived from them. It SHALL NOT originate a severity, a
safety impact, a rationale or a verdict that an anomaly or release is acceptable.

## Scope

- Applies to: `quoin anomaly report`, with and without `--fail-on`, in both formats.
- Out of scope: the content of the anomaly records, which `agent-ix/spec-artifacts-process` governs.

## Rationale

A report that words an open safety-impacting anomaly as "accepted", or that fills a missing rationale with a
sentence, reads as a judgement nobody made, and a reviewer defers to it. Every rationale must be a named person's,
and every absent value must read as absent.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Authored files changed or created by a run | 0 | 0 | Test |
| Output sentences stating a risk, anomaly or release accepted or acceptable | 0 | 0 | Test |
| Score, rank, verdict, count-as-grade or priority keys emitted | 0 | 0 | Test |
| Differing output bytes across two runs on identical inputs | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-035-AC-1 | Over every fixture of FR-162 to FR-164, every file's bytes are identical before and after a run, and no file is created. | Test |
| NFR-035-AC-2 | In the Markdown over every fixture, the stem "accept" appears only in values copied from a record. | Test |
| NFR-035-AC-3 | The JSON over every fixture holds no key named or typed as a score, rank, verdict, grade, recommendation or priority. | Test |
| NFR-035-AC-4 | Over a fixture with every optional value absent, each printed value is `not stated`, `null`, `none`, an absent marker or copied from a named record. | Test |
| NFR-035-AC-5 | Two runs over the same working tree and history give byte-identical output in both formats, with and without `--fail-on`. | Test |

## Verification

All criteria are tests over fixtures.

## Dependencies

- **Upstream**: [FR-162](../functional/FR-162-anomaly-report.md), [FR-163](../functional/FR-163-anomaly-report-release-rationale.md), [FR-164](../functional/FR-164-anomaly-report-gate.md).
