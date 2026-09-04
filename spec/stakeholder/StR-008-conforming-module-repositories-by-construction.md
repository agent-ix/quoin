---
id: StR-008
title: "Module owners need new semantic-module repositories to conform by construction"
type: StR
verification_method: test
evidence:
  - kind: test_case
    ref: tests/semantic-module-template.test.ts
relationships:
  - target: "ix://agent-ix/quoin/FR-076"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-083"
    type: "satisfied_by"
---
# StR-008: Module owners need new semantic-module repositories to conform by construction

## Stakeholder Need

Module owners require that a new Quire semantic-module repository shall be
generated from one maintained template that already carries the accepted
semantic-module contract, so that adopting the contract is a matter of filling
in a module's own vocabulary rather than porting scaffolding, licensing,
packaging, and verification decisions by hand from an existing repository.

## Rationale

The default catalog already carries ten module repositories whose scaffolding,
licenses, manifests, schema directories, tests, packaging, and release metadata
are maintained independently. Two of them have now completed the semantic-module
migration by hand and five more are in flight. Creating the next repository by
copying one of those would carry that module's accidental history — its
vocabulary, its navigation category, its frozen compatibility baseline — into a
repository that has no history of its own, and would carry the pre-contract
`data_schema: {type: object}` placeholder wherever the copy source predates the
migration. Owners need the conforming shape to be the default output, not the
result of a review that catches omissions after the fact.

## Validation Criteria

| ID | Criteria | Validation |
| --- | --- | --- |
| StR-008-VC-1 | A repository rendered from the template with no hand editing carries a complete `semantic` manifest block, real emitted JSON Schemas, typed authoring skeletons, and a verification suite that fails rather than skips without the engine. | Demonstration |
| StR-008-VC-2 | A rendered repository passes its own clean-checkout gate — spec validation, lint, schema-drift check, and unit tests — before any module vocabulary is authored. | Test (TC-1410) |
| StR-008-VC-3 | A required surface that a maintained module repository carries and the template omits is reported by a conformance gate rather than discovered during the next migration. | Test (TC-1418) |

## Stakeholders

The primary stakeholders are Quire semantic-module owners creating a new module
repository, and the module-contract owners accountable for the fleet conforming
to one contract. Affected parties are Quoin catalog maintainers, who install
whatever these repositories publish, and Quire, which validates the artifacts
they declare.

## Context and Assumptions

The semantic-module contract is settled (FR-070 through FR-075). TypeSpec is the
structural schema source by ADR-0005, and `@agent-ix/semantic-core` supplies the
shared grammar. It is assumed that a rendered repository is a starting point that
its owner then fills with module vocabulary — the template does not attempt to
generate a module's types.

## Dependencies

**Upstream**: the semantic-module contract (FR-070 through FR-075) and the
semantic-core grammar. **Downstream**: each module repository's own
schema-completion work, which is out of scope here.

## Priority and Risk (Informative)

P0. The risk if unmet is that each new module repeats a multi-week hand
migration, and that the fleet's contract conformance keeps depending on whichever
repository was most recently copied.
