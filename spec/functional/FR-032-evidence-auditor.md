---
id: FR-032
title: "Suspect-link, freshness and vacuous-evidence auditor"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
  - target: "ix://agent-ix/quire-rs/FR-053"
    type: "traces_to"
---

# FR-032: Suspect-link, freshness and vacuous-evidence auditor

## Description

`quoin` SHALL audit the evidence store against the obligations of the day and
report the ways evidence has rotted, without running anything.

A trace link is currently a string match that never expires. Evidence rots in
three ways, none of them detected before this:

1. **Suspect links** — the statement changed after the evidence was bound. The
   highest-value single check in the traceability design, stolen deliberately
   from DOORS: the requirement moved and the evidence did not follow, while the
   matrix still reads as covered.
2. **Stale evidence** — bound to a missing run, a failed run, or a run behind
   HEAD.
3. **Vacuous evidence** — a tagged symbol that was skipped or never reported.
   quire-rs#72's 1,014 dead tags are the extreme case; this owns the family.

### The auditor runs nothing

It reads the store and reports (ADR-0011 invariant 1); the consumer's CI
refreshes. That separation is what makes the report trustworthy — an auditor
that could re-run a suite could also make a finding disappear by re-running it.

### Severity says what kind of wrong

- **High** is evidence that *claims to exist and does not hold*: a suspect link,
  a binding naming a run that is not in the store, a binding every one of whose
  symbols was skipped.
- **Medium** is ordinary work in progress: an obligation with no evidence yet, a
  run behind HEAD, a method mismatch, insufficient multiplicity.

An unwritten test and a lie about a written one are different problems, and
grading them the same teaches readers to skim both.

### Independent means a different suite

Criticality can demand two independent methods. Two symbols in **one** suite
share a harness, a fixture set and a failure mode, so counting them as two would
let one broken assumption look like corroboration. Independence is measured in
suites.

### Method conformance is asked through the catalog

Only the catalog knows which class a method belongs to, so conformance is
checked against it rather than by name matching. **Without a catalog the check
is skipped**: an absent catalog means the question cannot be asked, which is
different from the answer being yes.

### Ratchet, because a gate that fails on the backlog gets disabled

`--ratchet` compares against the accepted baseline and reports only new
violations. The per-PR delta names what a change added and resolved.

## Inputs

- Obligations from a validated `quire coverage --json` payload
- The binding graph and the newest run per suite from the store
- The merged verification-method catalog
- The accepted baseline, under `--ratchet`

## Outputs

- Findings, each with a kind, an obligation, a severity and a summary naming
  what to do
- The set of obligations whose evidence is healthy
- Under `--strict`, a non-zero exit when any reported finding remains

## Behavior

- The auditor SHALL check each obligation in id order, emitting findings ordered
  by obligation then kind, so the same input yields the same report.
- The auditor SHALL stop at the first disqualifying finding per obligation. A
  suspect link makes every downstream question about that obligation moot, and
  reporting four consequences of one cause is how a report becomes unreadable.
- Vacuity SHALL fire only when **every** bound symbol was skipped or absent. One
  passing symbol is evidence, even beside a skipped sibling.
- The auditor SHALL treat a symbol the run never reported as vacuous in the
  strongest sense: the binding names evidence the suite did not produce.
- The auditor SHALL NOT execute any suite, spawn any test runner, or modify the
  store.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-032-CON-1 | The auditor SHALL run nothing and write nothing. It reads and reports; the consumer's CI refreshes (ADR-0011 invariant 1). | Architecture | Inspection |
| FR-032-CON-2 | Every check SHALL be a pure function of its inputs — no clock, no filesystem walk, no subprocess inside the audit itself. The caller assembles the inputs, which is what makes the whole thing testable without a repository. | Architecture | Test |
| FR-032-CON-3 | The auditor SHALL skip a check it cannot perform rather than guessing at it. An absent catalog, an absent HEAD, an absent baseline each remove a question rather than answering it. | Architecture | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-032-AC-1 | An obligation whose binding hash matches, whose suite has a run at HEAD, and whose symbols passed produces no finding and is reported healthy. | Test (TC-137) |
| FR-032-AC-2 | A statement reworded after binding produces a high-severity suspect-link finding naming both hashes. | Test (TC-138) |
| FR-032-AC-3 | A binding naming a suite with no recorded run is high severity, while a run merely behind HEAD is medium. | Test (TC-139) |
| FR-032-AC-4 | Vacuity fires when every bound symbol was skipped or absent from the run, and does not fire when at least one passed. | Test (TC-140) |
| FR-032-AC-5 | An obligation with no binding is reported undischarged at medium severity. | Test (TC-141) |
| FR-032-AC-6 | A non-test-class method discharged by a test run is flagged; a test-class one is not; and with no catalog the question is not asked. | Test (TC-142) |
| FR-032-AC-7 | A criticality demanding two independent methods is satisfied by two suites and not by two symbols in one, and does not apply below the threshold. | Test (TC-143) |
| FR-032-AC-8 | Every check folds over **all** of an obligation's bindings: a suspect link is reported when any binding predates the current statement, evidence is vacuous only when every symbol in every suite was skipped or absent, and multiplicity counts the distinct suites actually bound. | Test (TC-145) |
| FR-032-AC-8 | `ratchet` reports only violations absent from the baseline, and `delta` names what a change added and resolved. | Test (TC-144) |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) (the store it reads), [FR-031](./FR-031-catalog-driven-advisor.md) (the catalog method conformance is checked against), quire-rs [FR-053](ix://agent-ix/quire-rs/FR-053) (the obligations and hashes it compares)
- **Downstream**: the consuming workflow decides whether a finding blocks; this command reports and, under `--strict`, exits non-zero
