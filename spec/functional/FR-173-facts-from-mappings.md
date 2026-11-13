---
id: FR-173
title: "quoin evidence facts-from-mappings: discharge facts proposed from effective controls"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-133"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "extends"
  - target: "ix://agent-ix/spec-objects-security/FR-014"
    type: "references"
---

# FR-173: quoin evidence facts-from-mappings: discharge facts proposed from effective controls

## Description

Control mappings feed `quoin evidence record-discharge` through one new read-only command that turns a corpus's mappings
into the JSON array of facts that `record-discharge --facts` takes:

```
quoin evidence facts-from-mappings --binding <path|-> --attestation <path> [--spec <dir>]
```

The command writes the array to stdout and writes nothing else. A person then stores it with
`quoin evidence record-discharge --binding B --facts -`, which is unchanged.

## Rationale

The mapping says a control answers a clause; the discharge fact is the attested statement that it does. Everything the
fact needs that is not a person's decision comes from the mapping (the clause id and the evidence references), and
everything that is a person's decision comes from the person.

## Behavior

- For each clause of the binding report that binds, the command SHALL emit one `direct` fact when at least one `control` in `--spec` maps that clause of that set and states `effectiveness: effective`.
- A direct fact's `evidenceRefs` SHALL be the `ix://` ids of those controls, sorted and unique.
- The command SHALL copy a fact's `attestation` unchanged from the `--attestation` file, which a person authors with `attestedBy`, `authority`, `attestedAt`, `expiresAt`, `sourceRevision` and `evidenceDigest`.
- The command SHALL emit no fact for a clause whose mapping controls are all `not_assessed`, `ineffective` or `partially_effective`.
- The command SHALL name such clauses and their control ids on stderr or in `--json` output as left open.
- The command SHALL emit no fact for a mapping to a clause that does not bind, or whose set is not the binding report's set.
- The command SHALL list such a mapping as unused.
- The command SHALL emit a `disposition` fact with `decision: delegated` for a clause that has a `scope_exclusion` and binds, with the exclusion's rationale and owner as the rationale and with `approvalRef` taken from a required `--approval-ref` value.
- When `--approval-ref` is absent, the command SHALL emit no disposition.
- When `--approval-ref` is absent, the command SHALL list the exclusion as needing approval.
- The command SHALL exit non-zero, emitting nothing, when the attestation file is missing or incomplete, when a clause is both mapped and excluded, or when the corpus fails validation.
- The command SHALL emit the same bytes for the same inputs.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-173-AC-1 | Over a synthetic set `example.invalid/std@1` with two binding clauses and a corpus where an `effective` control maps clause C1, the output holds one `direct` fact for C1 with that control's id in `evidenceRefs` and the attestation byte-for-byte as supplied. | Test |
| FR-173-AC-2 | A clause mapped only by `not_assessed`, `ineffective` or `partially_effective` controls yields no fact and is reported as left open with the control ids. | Test |
| FR-173-AC-3 | A mapping to a non-binding clause, and one to another set, yield no fact and are listed as unused. | Test |
| FR-173-AC-4 | A binding clause with a `scope_exclusion` and `--approval-ref X` yields a `delegated` disposition fact with `approvalRef` X; without `--approval-ref` it yields none and is listed as needing approval. | Test |
| FR-173-AC-5 | A missing or incomplete attestation file, a clause both mapped and excluded, and an invalid corpus each exit non-zero with empty stdout. | Test |
| FR-173-AC-6 | Two runs over identical inputs, with the corpus files in different directory order, produce byte-identical output. | Test |
| FR-173-AC-7 | The output of the command is accepted by `quoin evidence record-discharge --facts -` against the same binding report. | Test |

## Dependencies

- **Upstream**: [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md), [FR-046](./FR-046-clause-discharge.md); spec-objects-security FR-014, FR-017, FR-018
- **Downstream**: [FR-174](./FR-174-per-clause-compliance-from-mappings.md), [NFR-038](../non-functional/NFR-038-mapping-facts-state-and-never-decide.md)
