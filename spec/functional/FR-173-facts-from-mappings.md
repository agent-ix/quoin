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
  - target: "ix://agent-ix/spec-objects-security/FR-016"
    type: "references"
---

# FR-173: quoin evidence facts-from-mappings: discharge facts proposed from effective controls

## Description

Control mappings feed `quoin evidence record-discharge` through one new command that turns a corpus's mappings into the JSON
array of facts that `record-discharge --facts` takes:

```
quoin evidence facts-from-mappings --binding <path|-> --attestation <path>
                                   [--spec <dir>] [--approval-ref <ref>] [--repo <dir>]
```

- `--binding` is the `clause-binding-v1` report of one clause set, as for `record-discharge`.
- `--attestation` is a JSON object with exactly `attestedBy`, `authority`, `attestedAt` and `expiresAt`, the FR-046 attestation, written by a person.
- `--spec` is the directory of spec artifacts to read; it defaults to `spec` under the working directory.
- `--approval-ref` is optional; without it no disposition is emitted.
- `--repo` is optional; with it the command also reads the stored facts (FR-133) to name stale ones.

The command writes only the facts array to stdout, as JSON, and writes no file. Everything else it has to say goes to stderr as
a report, in the fixed form below. A person then stores the array with `quoin evidence record-discharge --binding B --facts -`,
which is unchanged.

"The clause set" is the one the binding report names. Quire derived that report from an installed module or pack, so a licensed
set is available here exactly when it is installed (FR-137), and the command needs no clause text.

## Rationale

The mapping says a control answers a clause; the discharge fact is the attested statement that it does. Everything the fact
needs that is not a person's decision comes from the mapping (the clause id and the evidence references); everything that is a
person's decision comes from the person. Cross-document checks over the corpus belong here, where the corpus and the binding
report are both read, and not in Quire.

## Behavior

Facts:

- For each clause of the binding report that binds, the command SHALL emit one `direct` fact when at least one `control` in `--spec` maps that clause of that set and has the extracted record key `effectiveness` equal to `effective` (spec-objects-security FR-014).
- A direct fact's `evidenceRefs` SHALL be the `ix://` ids of those controls, sorted and unique.
- The command SHALL copy the `--attestation` object unchanged as each fact's `attestation`.
- The command SHALL emit no fact for a clause whose mapping controls have no `effectiveness` key or one other than `effective`.
- The command SHALL emit no fact for a mapping to a clause that does not bind.
- The command SHALL emit no fact for a mapping to a set other than the binding report's set.
- For a binding clause that has a `scope_exclusion`, the command SHALL emit one `disposition` fact with `decision: delegated`.
- The `rationale` of that fact SHALL be the exclusion's `rationale`, then ` (responsible: `, then the exclusion's `responsible`, then `)`.
- The `approvalRef` of that fact SHALL be the `--approval-ref` value.
- When `--approval-ref` is absent, the command SHALL emit no disposition fact.

Refusals; each exits non-zero with empty stdout and names the control or exclusion, the set key and the clause id:

- If the attestation file is missing, is not an object, or does not hold exactly the four FR-046 attestation fields, then the command SHALL refuse.
- If `--spec` fails `quire validate`, then the command SHALL refuse.
- If a control maps, in the binding report's set, a clause that the binding report does not list, then the command SHALL refuse.
- If one control holds the same set and clause twice, then the command SHALL refuse.
- If two `scope_exclusion` artifacts name one set and clause, then the command SHALL refuse.
- If a clause is both mapped by a control and named by a `scope_exclusion`, then the command SHALL refuse.
- If an identifier of an emitted fact breaks the FR-133 identifier rules, then the command SHALL refuse with the FR-133 status (`invalid-identifier` or `case-collision`).

Report (stderr, one line each, sorted, on success):

- `open <clause> <control-id>...` for a binding clause mapped only by controls that are not `effective`.
- `unused <control-id> <set> <clause>` for a mapping that emitted no fact because the clause does not bind or the set is another.
- `needs-approval <clause> <exclusion-id>` for an exclusion with no disposition emitted.
- `stale <clause>` for each stored `direct` fact (with `--repo`) of a binding clause for which the command emitted no `direct` fact.

Determinism:

- The command SHALL emit the same bytes on stdout and stderr for the same inputs, whatever the order of the files in `--spec`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-173-AC-1 | Over a synthetic set `example.invalid/std@1` with two binding clauses and a corpus where a control with `## Effectiveness` `effective` maps clause C1, stdout holds one `direct` fact for C1 with that control's id in `evidenceRefs` and an `attestation` equal to the supplied object. | Test |
| FR-173-AC-2 | A clause mapped only by a control that is `not_assessed`, `ineffective`, `partially_effective` or has no `## Effectiveness` section yields no fact and an `open` line naming the control ids. | Test |
| FR-173-AC-3 | A mapping to a non-binding clause and one to another set yield no fact and an `unused` line each. | Test |
| FR-173-AC-4 | A binding clause with a `scope_exclusion` and `--approval-ref X` yields a `delegated` disposition fact with `approvalRef` X and a `rationale` of the form `<rationale> (responsible: <responsible>)`; without `--approval-ref` it yields none and a `needs-approval` line. | Test |
| FR-173-AC-5 | Each refusal case above, namely a missing attestation, an attestation with an extra field, an invalid corpus, an unknown clause in the binding report's set, a duplicate row in one control, a duplicate exclusion, a mapped-and-excluded clause and an identifier that folds onto another, exits non-zero with empty stdout and a message naming the control or exclusion, set and clause. | Test |
| FR-173-AC-6 | Two runs over identical inputs, with the corpus files in different directory order, produce byte-identical stdout and stderr. | Test |
| FR-173-AC-7 | The stdout of a successful run is accepted by `quoin evidence record-discharge --facts -` against the same binding report. | Test |
| FR-173-AC-8 | A run writes no file: a listing of the working directory, `--spec` and `--repo` is identical before and after. | Test |
| FR-173-AC-9 | With `--repo` holding a stored `direct` fact for clause C1 and a corpus in which C1's only control is now `ineffective`, stdout has no fact for C1 and stderr has `stale C1`. | Test |

## Dependencies

- **Upstream**: [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md), [FR-046](./FR-046-clause-discharge.md); spec-objects-security FR-014, FR-016
- **Downstream**: [FR-174](./FR-174-per-clause-compliance-from-mappings.md), [NFR-038](../non-functional/NFR-038-mapping-facts-state-and-never-decide.md)
