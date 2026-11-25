---
id: SR-967175026302197768900474181941935084775
title: "Scope-boundary review — PLAT-1449 quoin"
type: SpecReview
analysis: scope-boundary
scope: "PLAT-1449 quoin PR #1069 against origin/main"
review_set: subset
---

## Summary

FR-235 includes a quire-cli entry point and validation-kit behavior in a quoin requirement, while assigning implementation ownership to external repos/tickets; boundary is described but cross-repo acceptance is indirect.

## Verdict

**CONDITIONAL** — FR-235 includes a quire-cli entry point and validation-kit behavior in a quoin requirement, while assigning implementation ownership to external repos/tickets; boundary is described but cross-repo acceptance is indirect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR says quire-cli shall provide its corresponding entry point and assigns the trace coverage check to PLAT-1640; those obligations cannot be accepted by quoin’s self-test implementation alone. | spec/functional/FR-235-quoin-selftest.md:81-90; 109-116 |

## Coverage

Reviewed frozen PR head a50fe034ecf05d405658c80e8b582c04940103fa against origin/main. Ratings remain proposed and unsigned; no acceptance decision is inferred. Plan completion: not assessed.


## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The fix addresses the finding in the current diff. |

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      - Quoin owns the `quoin selftest` entry point and its cases. The quire-cli
        repository provides the corresponding `quire selftest` entry point for the
        cases it owns. Its binary may embed quire-rs-owned case assets exposed by
        the linked quire-rs crate at build time; quire-cli shall not copy or maintain
        those cases.
```

+++

## Current diff review — PR head 965259ca6f32889e6f8b763d085cd5fd790a13f1

The new intended-use record limits Quoin's role to creating, validating, and presenting repository engineering records; it explicitly disclaims compliance and product/clinical safety determinations, and assigns acceptance to people. Its operating context names the filesystem and configured executables as external inputs and identifies Git, Quire, Engineering Assurance, and configured producer/analysis programs. This matches the stated tool scope; no new boundary-allocation defect was found in the added documents.
