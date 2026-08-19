---
id: FR-039
title: "Mutation score as a declared verification threshold"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "requires"
---

# FR-039: Mutation score as a declared verification threshold

## Description

Every quality signal this program has built over acceptance criteria is a **proxy**: does the
criterion use a vague verb, does it name a concrete object, is it shaped as an assertion. Those are
word lists and regexes over an open vocabulary, and the corpus already showed what that is worth —
**1,201 distinct verb stems**, of which a built-in list of 13 covered **14.5%**.

A mutation score is the direct answer to the question those proxies approximate: *does the test
discriminate the behaviour the criterion describes?* A surviving mutant means either the tests do not
check that behaviour or the criterion never demanded it, and both are findings with evidence
attached.

This states where that score is **obligatory** and what happens when it is missed. It does not run
anything: mutation testing is L2 for consumers, their CI executes it, and quoin's part is to advise
and to audit (ADR-0011).

### The floor is declared, never assumed

`--mutation-floor <criticality>=<ratio>` is **unset by default**, for the reason CR-008 deleted the
hardcoded `multiplicityRequires: ["P0"]`: a built-in floor is a rule nobody chose that would fire on
everything the moment a criticality column appeared. Measured then, and still true: 2,304 of 2,304
`Acceptance Criteria` tables in the ecosystem carry no priority column.

### Two findings, because the failures are different

**`insufficient-mutation-score`** — a score exists and is below the floor. The tests ran and did not
discriminate.

**`unmeasured-mutation-score`** — the floor is demanded and no bound run records a score. This is not
`undischarged`: the obligation may be thoroughly tested, with a passing run bound to it, and still
have nothing saying the tests detect anything. **A threshold that cannot be evaluated is not a
threshold met**, and collapsing it into the undischarged bucket would hide it behind evidence that
already exists.

### The worst score, not the mean

Averaging lets a well-tested symbol carry one whose mutants all survive — which is precisely the case
a threshold exists to find. The finding names the weakest bound symbol's score, so the number in the
report is the number to act on.

### A skipped symbol's absent score is not zero

Treating it as zero fails the obligation for a test **nobody ran**, rather than for a test that failed
to discriminate. Those have different remedies, and `vacuous-evidence` already reports the first.

### A percentage is refused, not accepted

`--mutation-floor P0=80` is the natural thing to type and is a floor nothing can reach. Accepted
silently it would report every `P0` obligation as failing, in a form indistinguishable from a real
finding. The flag rejects anything outside `[0, 1]` and says so.

The same reasoning covers a malformed pair: an ignored `--mutation-floor` means the operator asked
for a threshold and gets a clean report saying nothing was below it. **A floor that silently does not
apply is worse than no floor**, because it reads as a passing gate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-039-AC-1 | With no floor declared, no mutation finding is produced whatever the recorded score. | Test (TC-219) |
| FR-039-AC-2 | A bound score below the declared floor is reported at `medium`. | Test (TC-219) |
| FR-039-AC-3 | A score exactly at the floor is accepted. | Test (TC-219) |
| FR-039-AC-4 | The judgement is on the weakest bound symbol, not the mean. | Test (TC-219) |
| FR-039-AC-5 | A demanded floor with nothing measured is reported, distinctly from `undischarged`. | Test (TC-219) |
| FR-039-AC-6 | A skipped symbol's absent score is not read as zero. | Test (TC-219) |
| FR-039-AC-7 | A floor applies only to the criticality it names. | Test (TC-219) |
| FR-039-AC-8 | `quoin evidence audit --mutation-floor P0=0.8` parses the pair and applies it. | Test (TC-220) |
| FR-039-AC-9 | A score outside `[0, 1]`, or a malformed pair, is refused rather than ignored. | Test (TC-220) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-039-CON-1 | quoin SHALL NOT execute a mutation run. The consumer's CI does (ADR-0011 invariant 1). | Design | Inspection |
| FR-039-CON-2 | quoin SHALL NOT declare a default floor. An unset floor produces no finding. | Design | Test (TC-219) |
| FR-039-CON-3 | quoin SHALL NOT infer a score from coverage or from a passing run. Only a recorded score counts. | Design | Test (TC-219) |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md) (the `cargo-mutants` adapter that records the score), [FR-032](./FR-032-evidence-auditor.md) (the auditor this extends)
- **Downstream**: `agent-ix/quoin#128` — `mutation-testing` is keyed on `high-criticality` and `suite-quality-unknown`, neither of which the advisor mints, so the method cannot yet be *recommended* even where this threshold would demand it
