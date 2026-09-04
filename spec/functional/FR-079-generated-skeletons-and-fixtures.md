---
id: FR-079
title: "Generated authoring skeletons, mappings, and compatibility fixtures"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-071"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-072"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-074"
    type: "depends_on"
---

# FR-079: Generated authoring skeletons, mappings, and compatibility fixtures

## Description

The rendered repository SHALL ship, for every exported type, an authoring
skeleton in the mapping forms FR-071 and FR-072 define, together with one
negative fixture per declared failure mode and one legacy-form fixture, so that
the contract's mappings are demonstrated by files a reader can run rather than
described in prose.

## Rationale

The typed `## Properties` table is the default form and the `sysml` fence its
alternate; invariants are `ocl` fences under `### <clauseId>` headings. A template
that shipped only the default form would leave the alternate undemonstrated, and
a template that shipped only positive examples would ship a contract nothing
proves is enforced. Both completed migrations pair every skeleton with a negative
counterpart whose frontmatter names the expected refusal and the reason.

## Inputs

- The exported types of the rendered manifest
- `module_kind`

## Outputs

- `<package>/skeletons/<type>.md`, the typed-table default form
- `<package>/skeletons/<type>.sysml.md`, the fence alternate
- `tests/fixtures/negative/*.md`, one per declared failure mode
- `tests/fixtures/legacy/*.md`, the pre-contract authoring form
- `<package>/mappings.yaml` and `<package>/examples/*.record.json`, where `module_kind` declares artifact types

## Behavior

- Each rendered skeleton SHALL carry frontmatter naming its `id`, `title`, and `type`, and a body comment stating the contract the manifest's `body_extraction` asserts.
- Each rendered skeleton SHALL carry a `## Properties` section whose table header is exactly `Field | Type | Multiplicity | Constraints`, with at least one substantive row.
- Each rendered skeleton SHALL be accompanied by an alternate file declaring the same fields as one ```sysml``` fence under `## Properties`.
- Each rendered skeleton SHALL carry an `## Invariants` section holding at least one ```ocl``` fence under its own `### <clauseId>` heading.
- The rendered repository SHALL carry, for every failure mode its emitted schemas refuse, one negative fixture whose frontmatter names the expected diagnostic and the reason.
- The rendered repository SHALL carry a legacy-form fixture whose `## Properties` section uses the pre-contract free-text form.
- When the legacy-form fixture is validated under the rendered manifest, the rendered suite SHALL assert exactly one legacy warning and no error, so that `legacy_forms: warning` is exercised rather than asserted.
- If a rendered skeleton carries both the table and the fence form in one document, then the rendered suite SHALL assert that the document is refused.
- Where `module_kind` declares artifact types, the rendered repository SHALL carry mapping declarations naming, for every exported model property, the mapping kind and source section that yields it, and one golden record per exported type.
- The rendered emit command SHALL serialize each golden record with sorted keys, two-space indentation, and a trailing newline, so that a golden diff is a content diff.
- The rendered skeletons SHALL carry no vocabulary from an existing module repository, so that a rendered repository does not inherit another module's domain.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-079-CON-1 | A rendered skeleton section SHALL carry substantive content, not a `TODO` or an ellipsis placeholder. | Completeness | Test (TC-1411) |
| FR-079-CON-2 | One rendered artifact SHALL carry one Properties form; the alternate is a separate file. | Contract | Test (TC-1422) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-079-AC-1 | Every exported type of every rendered variant has a skeleton with the exact typed-table header and at least one row. | Test (TC-1420) |
| FR-079-AC-2 | Every rendered skeleton has a `sysml`-fence alternate declaring the same field names, types, and multiplicities. | Test (TC-1421) |
| FR-079-AC-3 | Every rendered skeleton carries at least one `ocl` fence under its own `### <clauseId>` heading. | Test (TC-1423) |
| FR-079-AC-4 | Every rendered negative fixture is refused, and each is refused for its own distinct reason. | Test (TC-1424) |
| FR-079-AC-5 | The rendered legacy-form fixture validates with exactly one legacy warning and zero errors. | Test (TC-1425) |
| FR-079-AC-6 | A document carrying both Properties forms is refused. | Test (TC-1422) |
| FR-079-AC-7 | No rendered skeleton contains a placeholder body; each section's content is substantive. | Test (TC-1411) |
| FR-079-AC-8 | For an artifact or mixed rendering, every property of every exported model has exactly one mapping entry, and every golden record round-trips from its skeleton. | Test (TC-1426) |

## Dependencies

- **Upstream**: [FR-071](./FR-071-typed-properties-mapping.md), [FR-072](./FR-072-invariants-and-operations-mapping.md), [FR-074](./FR-074-legacy-authoring-forms.md), [FR-078](./FR-078-generated-manifest-semantic-block.md)
- **Downstream**: [FR-080](./FR-080-generated-verification-suite.md)
