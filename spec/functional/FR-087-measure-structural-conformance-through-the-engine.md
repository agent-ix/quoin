---
id: FR-087
title: "Measure structural conformance through the Quire engine"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-086"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-085"
    type: "depends_on"
---

# FR-087: Measure structural conformance through the Quire engine

## Description

The corpus measurement SHALL obtain each measured document's conformance to its module's declared
contract by running the Quire validation engine over that document against the resolved module set,
and SHALL record exactly one outcome from `pass`, `fail` and `could-not-run` per document.

## Rationale

Quire owns parsing, validation and extraction; the epic forbids a parallel replacement. A rate
measured by a second checker written inside Quoin would not describe the checker the promotion gate
is asked to promote, and the mapping semantics that a reimplementation gets wrong are stated as
English prose in the modules — `spec-artifacts-iso` alone states six parse rules as paragraphs and
ships an 871-line reference mapper as their only executable form. Driving the engine makes the number
a statement about the real contract, and makes everything the engine cannot do visible as
`could-not-run` instead of invisible as agreement.

## Inputs

- The `measured` documents of FR-086.
- The resolved module set and toolchain record of FR-085.

## Outputs

- One evaluation record per document: outcome, the engine's diagnostic code, severity, reason,
  document line and message for each diagnostic the engine reported against it.
- A per-run record of the engine identity the outcomes were produced by.

## Behavior

- The measurement SHALL run the engine with the resolved module set as its only module source, so
  that no installed catalog copy or repository-local module can supply a contract the run did not
  pin.
- The measurement SHALL record a document `pass` when the engine reports no diagnostic of severity
  `error` against it.
- The measurement SHALL record a document `fail` when the engine reports at least one diagnostic of
  severity `error` against it.
- The measurement SHALL record every diagnostic the engine reported against a failing document in
  that document's single evaluation record.
- The measurement SHALL record a document `could-not-run` when the engine exits without reporting an
  outcome for that document.
- If running the engine over a batch of documents terminates abnormally, then the measurement SHALL
  record every document of that batch `could-not-run` naming the termination.
- The measurement SHALL record a diagnostic of severity `warning` as an advisory finding against its
  document and SHALL NOT count it as a failure, because a module that declares a check advisory has
  not asked for it to be enforced.
- The measurement SHALL record the engine's version and source revision alongside the outcomes.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-087-CON-1 | A `could-not-run` outcome SHALL NOT be counted in the numerator or the denominator of a pass rate. | Interface | Test |
| FR-087-CON-2 | The measurement SHALL NOT implement its own parser, extractor or record builder for a module's declared mappings. | Interface | Inspection |
| FR-087-CON-3 | The measurement SHALL NOT rewrite the bytes of a corpus document. | Safety | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-087-AC-1 | A document the engine reports no error against records `pass`. | Test (TC-1518) |
| FR-087-AC-2 | A document the engine reports a missing required section against records `fail` carrying the engine's diagnostic code, reason and line. | Test (TC-1519) |
| FR-087-AC-3 | The module contract applied is the resolved module set, demonstrated by a repository-local module declaring the same type having no effect on the outcome. | Test (TC-1520) |
| FR-087-AC-4 | A batch whose engine invocation terminates abnormally records every document of that batch `could-not-run` naming the termination, and none of them enters the pass numerator or denominator. | Test (TC-1521) |
| FR-087-AC-5 | A document the engine reports only warnings against records `pass` and carries those warnings as advisory findings. | Test (TC-1522) |
| FR-087-AC-6 | A document carrying three engine errors records all three in one evaluation record. | Test (TC-1523) |
| FR-087-AC-7 | Changing only the module declaration in the resolved set changes the recorded outcome of an unchanged document. | Test (TC-1524) |
| FR-087-AC-8 | The evaluation output names the engine version and source revision the outcomes were produced by. | Test (TC-1572) |
| FR-087-AC-9 | No source file of the measurement parses a module's `mappings.yaml` parse rules or builds a semantic record from Markdown. | Inspection |

## Dependencies

- **Upstream**: [FR-085](./FR-085-resolve-the-completed-module-set.md), [FR-086](./FR-086-assign-one-measurement-state-per-document.md)
- **Downstream**: [FR-089](./FR-089-partition-every-failure.md), [FR-090](./FR-090-publish-rates-with-unit-population-and-method.md)
