---
id: FR-086
title: "Assign exactly one measurement state to every corpus document"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-084"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-085"
    type: "depends_on"
---

# FR-086: Assign exactly one measurement state to every corpus document

## Description

The corpus measurement SHALL assign every enumerated corpus document exactly one state from
`measured`, `out-of-model`, `unreadable` and `contested`, so that no document leaves the population
without an explicit disposition.

## Rationale

The acceptance criterion of the campaign is that every corpus document is validated or explicitly
classified. A document that is neither is a silent exclusion, and a silent exclusion is how a pass
rate is inflated without anybody choosing to inflate it. Making the four states exhaustive and
mutually exclusive turns "we did not look at it" into a number the report has to print.

## Inputs

- The enumerated corpus documents of FR-084.
- The resolved module set of FR-085, with its declared artifact and object type vocabulary.

## Outputs

- One state record per document: repository, repository-relative path, state, declared `type` when
  the document has one, the resolving module when the type resolves, and a reason for every state
  other than `measured`.
- One finding of class `contract-defect` per contested type, naming the modules that contest it.

## Behavior

- When a document's frontmatter `type` resolves to a declared type of exactly one resolved module,
  the measurement SHALL assign that document the state `measured`.
- When a document's frontmatter `type` resolves to exactly one resolved module, the measurement SHALL
  record that module as the document's resolving module.
- When a document declares no frontmatter `type`, the measurement SHALL assign that document the
  state `out-of-model` with reason `no-declared-type`.
- When no resolved module declares a document's frontmatter `type`, the measurement SHALL assign that
  document the state `out-of-model` with reason `type-not-declared-by-any-module`.

A document nobody's vocabulary claims is a different finding from a document carrying no frontmatter
at all, so the two `out-of-model` reasons stay apart.

- If a document's bytes cannot be read, then the measurement SHALL assign that document the state
  `unreadable` carrying the read failure.
- If a document's frontmatter block is not terminated, then the measurement SHALL assign that
  document the state `unreadable` carrying the parse failure.
- While a document is in the `unreadable` state, the measurement SHALL continue to enumerate and
  state the sibling documents of that document.
- When two or more resolved modules declare a document's frontmatter `type`, the measurement SHALL
  assign that document the state `contested` carrying the competing module names.
- The measurement SHALL raise one finding of class `contract-defect` for each contested type, so that
  a contested type reaches the partition rather than disappearing between the states.
- The measurement SHALL retain every `contested` state in every published output.
- The measurement SHALL resolve a document's frontmatter `type` case-sensitively against the declared
  vocabulary.

The owner of a contested type is not determined by the data available, and guessing one would
fabricate the answer this measurement exists to obtain. Two modules claiming one type name is a defect
in the declarations, which is why the contested document raises a finding against the modules rather
than a failure against itself.


## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-086-CON-1 | The four states SHALL be exhaustive and mutually exclusive over the enumerated documents. | Interface | Test |
| FR-086-CON-3 | A type key SHALL be the pair of module name and type name, never the type name alone. | Interface | Test |
| FR-086-CON-2 | `out-of-model` SHALL NOT be reported as a failure of the document or of a module. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-086-AC-1 | A document whose `type` resolves to exactly one module is `measured` and names that module. | Test (TC-1512) |
| FR-086-AC-2 | A document with no frontmatter `type` is `out-of-model` with reason `no-declared-type`, and a document with an unrecognised `type` is `out-of-model` with reason `type-not-declared-by-any-module`. | Test (TC-1513) |
| FR-086-AC-3 | A document whose frontmatter fence is unterminated is `unreadable` with the failure recorded, and the documents beside it are still stated. | Test (TC-1514) |
| FR-086-AC-4 | A `type` declared by two resolved modules yields `contested` naming both modules, no aggregate reassigns it, and one `contract-defect` finding names the contesting modules. | Test (TC-1515) |
| FR-086-AC-5 | The four state counts sum to the enumerated document count of FR-084. | Test (TC-1516) |
| FR-086-AC-6 | No document appears in more than one state record. | Test (TC-1517) |
| FR-086-AC-7 | A document whose `type` differs from a declared type only by letter case is `out-of-model` with reason `type-not-declared-by-any-module`, not `measured`. | Test (TC-1573) |
| FR-086-AC-8 | Two modules declaring the same type name yield two distinct type keys in every breakdown, and neither type's documents are attributed to the other. | Test (TC-1574) |

## Dependencies

- **Upstream**: [FR-084](./FR-084-pin-and-enumerate-the-governed-corpus.md), [FR-085](./FR-085-resolve-the-completed-module-set.md)
