---
id: FR-037
title: "Declared-vocabulary completeness and its verdict policy"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quire-rs/FR-059"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-031"
    type: "extends"
---

# FR-037: Declared-vocabulary completeness and its verdict policy

## Description

A bundle can be 100% acceptance-criteria covered and still carry no requirement anywhere for
reliability, security or maintainability. Coverage answers *"is every criterion tested?"*; it cannot
answer *"is anything missing?"*, because a requirement nobody wrote has no criterion to be uncovered.

`quire-rs` **FR-059** answers the second question for any declared vocabulary: given a vocabulary and
a declared projection from documents onto it, which values does no document claim? That is a
deterministic fact about the spec alone, and ADR-0011 places it in the engine.

**This is the other half of that split: what a gap is worth, and whether an excuse was earned.**

### The vocabulary is read, never minted

ISO 25010 is the first instance, and the characteristic list is **module data** — 12 values in
`spec-artifacts-iso`'s NFR frontmatter schema. The original ticket proposed walking a hardcoded
9-item list; a second list is precisely the drift this area exists to prevent, so the values come
from the same declaration the engine reads.

Nothing here is 25010-specific. Test-type coverage over a Test Matrix, STRIDE coverage over declared
threats and `escape_cause` coverage over Findings are the same shape, already visible in module data.

### A justified absence is an answer, and today it costs nothing

*"This is a CLI that controls no physical process, so it has no safety characteristic"* is a real
answer. A check that cannot accept one forces either a permanent false finding or a requirement
fabricated to silence it, and both teach people to ignore the check.

**But the engine accepts a bare list.** Measured on this repository: `quire validate --okf` reported
**7** unowned characteristics; adding one frontmatter line —
`quality_attributes_not_applicable: [safety, compliance]` — reported **5**, with no reason written
anywhere. The cheapest way to make the check quiet is to excuse everything, and nothing notices.

So an exclusion is treated as **a claim about the product** and must carry a written reason, in the
same document, in a table row naming the value. A table rather than free prose because the value name
occurs in passing — "safety" appears in any document discussing safety — and a mention is not a
justification.

A rationale of `-`, `TBD` or `n/a` is not accepted. Those are an author acknowledging the question and
declining to answer it, which is what an unjustified exclusion is. The floor is **three words**, on
structure rather than length: a character count teaches padding, and forty characters saying nothing
is the same problem measured differently.

### The severities are deliberately asymmetric

| Finding | Severity | Why |
|---|---|---|
| `unowned` | medium | An admitted gap. A reader can see it and decide. |
| `unjustified-exclusion` | high | An assertion of completeness with nothing behind it — **and** it removes the finding that would have prompted the work. |
| `undeclared-exclusion` | high | A typo excuses nothing, while reading to its author as handled. The real value keeps reporting and nobody connects the two. |

Excusing a characteristic without a reason is worse than saying nothing about it. That inversion is
the policy this requirement exists to state.

### `UNCHECKED` is not a fourth flavour of pass

A bundle whose module set declares no vocabulary has not been assessed. Reporting `PASS` over it is
the green-matrix-over-dead-links result this program was created to stop — and the first draft of
this feature printed exactly that until `FR-037-AC-12` caught it. It fails only under `--strict`:
installing quoin must not break a repository that has not adopted the vocabulary, but a repository
asking for strict completeness is told that completeness cannot be known.

### Advisory by default

`--strict` promotes an admitted gap to a failure; it does not invent one. A new check landing as a
hard error across a corpus teaches people to disable it, and a disabled check reports nothing forever.
A `high` finding fails regardless, because it is not an admitted gap.

### On quoin reading documents at all

quire is the parser and quoin does not reimplement it. What is read here is the leading `---` block
as YAML, plus raw body text — no archetype resolution, no schema validation, no link walking.

The alternative was one `quire extract` subprocess per document: `extract` takes a single `<DOC>` and
a `--module`, so a bundle sweep is N spawns to answer a question about two frontmatter keys.

**The better fix is upstream.** If the FR-059 diagnostic classified each value as owned / excused /
unowned, quoin would need no reader at all — it currently learns only the unowned set, and cannot
tell an owned value from an excused one without opening documents. Filed as `agent-ix/quire-rs#179`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-037-AC-1 | The vocabulary's values are read from module data, not minted. | Test (TC-206) |
| FR-037-AC-2 | A declaration whose vocabulary cannot be resolved is reported, not dropped. | Test (TC-207) |
| FR-037-AC-3 | A value no document claims is reported as an unowned gap, at `medium`. | Test (TC-208) |
| FR-037-AC-4 | An exclusion carrying no written reason is `high` — above the gap it replaces. | Test (TC-209) |
| FR-037-AC-5 | An exclusion naming a value outside the vocabulary is reported, and excuses nothing. | Test (TC-210) |
| FR-037-AC-6 | A table row naming the value with a real reason accepts the exclusion. | Test (TC-211) |
| FR-037-AC-7 | A non-answer is not a reason, and a mention in prose is not a justification. | Test (TC-212) |
| FR-037-AC-8 | A `high` finding fails; a gap fails only under `--strict`. | Test (TC-213) |
| FR-037-AC-9 | `quoin completeness` reports the gaps over a real bundle and exits 0 while advisory. | Test (TC-214) |
| FR-037-AC-10 | `quoin completeness` exits non-zero on an unjustified exclusion without `--strict`. | Test (TC-215) |
| FR-037-AC-11 | `quoin completeness --strict` exits non-zero on gaps alone. | Test (TC-216) |
| FR-037-AC-12 | A bundle with no declared vocabulary reports `UNCHECKED`, never `PASS`. | Test (TC-217) |
| FR-037-AC-13 | The unowned set quoin reports equals the one the bundle read yields for the same declaration. | Test (TC-218) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-037-CON-1 | quoin SHALL NOT mint a vocabulary. Values come from the artifact type's frontmatter schema, which is where the engine reads them. | Design | Test (TC-206) |
| FR-037-CON-2 | quoin SHALL NOT compute coverage over the corpus. That is quire-rs FR-059; this reads the same declaration to explain and judge the result. | Design | Inspection |
| FR-037-CON-3 | quoin SHALL NOT parse document structure. It reads the frontmatter block and the raw body; every structural question goes to quire. | Design | Inspection |
| FR-037-CON-4 | An absent vocabulary SHALL NOT report `PASS`. Nothing checked is not nothing wrong. | Design | Test (TC-217) |

## Dependencies

- **Upstream**: `quire-rs` [FR-059](ix://agent-ix/quire-rs/FR-059) (computes the unowned set), `spec-artifacts-iso` v0.15.0 (declares the projection)
- **Downstream**: `agent-ix/quire-rs#179` (classify owned / excused / unowned in the diagnostic, retiring quoin's frontmatter reader)
