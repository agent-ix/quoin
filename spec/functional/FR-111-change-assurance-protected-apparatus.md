---
id: FR-111
title: "Change-assurance refuses credit for a diff that touches protected measurement apparatus"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-065"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-110"
    type: "extends"
---

# FR-111: Change-assurance refuses credit for a diff that touches protected measurement apparatus

## Description

When a change-assurance verification is linked to a `MeasurementPlan`
(engineering-assurance FR-024), Quoin SHALL refuse the change credit toward
that plan's objective with `apparatus_touched` if the retained diff touches a
path the plan's `protected_apparatus` names, SHALL report `diff_missing` if
the plan protects apparatus and no diff was retained, and SHALL report
`negative_control_uncaught` if the plan declares a `negative_controls` entry
this verification has no rule to evaluate.

## Rationale

FR-110 resolves a plan's protected apparatus at intake and compares the
*recorded* set across stored measurement collections — a collection has no
diff, so that is the whole of what a collection can be checked against. A
change-assurance record is checked against a proposed change instead, and a
proposed change has exactly the thing a collection lacks: a diff. Without
this requirement, a change that edits the harness, the answer key, the
population, or the checker configuration beside the change it grades could
still receive change-assurance credit toward the plan's objective, because
nothing between the diff and the plan's protection was ever compared.

A plan may also declare a negative control this verification has no rule for
— `suppressed-observation`, `gain-within-noise`, `stale-evidence`, or
`selective-reporting` are about a population, a margin, evidence age, or a
choice among several runs, none of which one candidate revision's retained
evidence carries. Silently ignoring a declared control a verification cannot
evaluate would let a receipt claim more than it checked; instead each such
control is its own incompleteness.

## Inputs

- `VerificationInput.diff_paths`: the repository-relative, `/`-separated
  paths the candidate change's diff touches (as `git diff --name-only` prints
  them), as retained by the caller. Absent when no diff was retained, which
  is distinct from a retained diff that touched nothing. A verification runs
  no producer and computes no diff itself — this is exactly what the caller
  supplied, like every other member of `VerificationInput`.
- `VerificationInput.governing_plan`: the `MeasurementPlan` the record's
  objective is linked to, when the caller asks this verification to check
  one. Carries only the two members this requirement reads —
  `protected_apparatus` and `negative_controls`, both engineering-assurance's
  own types (its FR-024) — because `quoin-measurement` owns the rest of a
  plan's shape and a verification is not a plan load. `None` when the record
  supports no plan, or the caller did not resolve one.

## Outputs

- The `apparatus_touched` reason (invalid), when `diff_paths` names a path
  inside `governing_plan.protected_apparatus`: an exact, case-sensitive
  match for a file entry, or a path under (or equal to) a `<directory>/**`
  entry's directory — the entry semantics FR-110 resolves at intake.
- The `diff_missing` reason (incomplete), when
  `governing_plan.protected_apparatus` is present and no diff was retained.
- The `negative_control_uncaught` reason (incomplete), when at least one
  declared `negative_controls` entry is uncaught. The receipt's reasons are a
  set, so it appears once however many entries are uncaught.
- All three reasons are folded into the receipt's overall `reasons`/`outcome` the
  way `proof_id_mismatch` already is (`verify/mod.rs`'s `global_reasons`):
  none has a dedicated member under `checks`, because the judgment is
  about the change proposed against a linked plan, not about the record's
  own shape.

## Behavior

- When `governing_plan` is `None`, this requirement adds no reason: a record
  that supports no plan's objective has nothing here to refuse.
- When `governing_plan.protected_apparatus` is `Some` and no diff was
  retained, Quoin SHALL add `diff_missing`: a diff that was never compared
  cannot be reported clean.
- When `governing_plan.protected_apparatus` is `Some` and any entry of
  `diff_paths` matches one of its entries, Quoin SHALL add
  `apparatus_touched`. This holds whether or not the plan declares the
  `apparatus-edit` negative control — the protection is unconditional, the
  same way FR-110-AC-7's checker rejects a changed recorded set with or
  without that control declared.
- When `governing_plan.negative_controls` is `Some`, Quoin SHALL add
  `negative_control_uncaught` when any declared entry's kind is not
  `apparatus-edit`, or is `apparatus-edit` with no `protected_apparatus` for
  it to guard (a combination `quoin-measurement`'s plan intake refuses). A
  declared `apparatus-edit` entry over a protected list is evaluated by the
  `apparatus_touched` rule above and adds nothing on its own.
- An untouched diff under a plan that protects apparatus adds neither reason:
  the receipt is exactly what it would be with no plan linked.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-111-AC-1 | A diff touching a path inside `protected_apparatus` — a protected harness file, a labels directory entry, a population file, or a checker-configuration file — is `apparatus_touched` and the receipt is `invalid`, whether or not `apparatus-edit` is declared. | Test (TC-1868) |
| FR-111-AC-2 | A diff naming no path inside `protected_apparatus` adds neither reason; the receipt is unchanged from an unlinked verification. | Test (TC-1869) |
| FR-111-AC-3 | A declared `negative_controls` entry whose kind is not `apparatus-edit`, or an `apparatus-edit` entry with no protected list, is `negative_control_uncaught`, and this alone leaves the outcome `incomplete` rather than `invalid`; a declared `apparatus-edit` entry over an untouched diff adds nothing. | Test (TC-1870) |
| FR-111-AC-4 | A plan that protects apparatus, verified with no diff retained, is `diff_missing` and the receipt is `incomplete`; with no plan linked, a missing diff adds nothing. | Test (TC-1871) |

## Constraints

- **FR-111-CON-1**: The entry grammar, list rules and control kinds are
  engineering-assurance's (FR-024); this requirement states none of them a
  second time, matching FR-110-CON-3.
- **FR-111-CON-2**: This verification computes no diff and loads no plan: both
  arrive as `VerificationInput` members the caller resolved, the same
  boundary FR-065 already holds for every other member of that input.
  `quoin-core`'s `change_assurance.receipt` operation extends this same
  boundary to its wire request (PLAT-997): `diff_paths` crosses as the
  caller's own claim about what the candidate revision changed — `quoin
  change-assurance receipt` does not shell out to `git` to compute or check
  it, so a caller that supplies the wrong list, or one narrower than the
  real diff, gets the judgment that list implies. `plan` is resolved
  differently: it is a `MeasurementPlan` id, not a document, and the
  operation resolves it itself through `quoin-measurement`'s own plan intake
  (the load PLAT-975 already governs) so `protected_apparatus` and
  `negative_controls` are read by that one parser rather than restated on
  the wire. It is read from `repo`'s plan documents as they stand on disk
  when the receipt is sealed, not from `candidate_revision`, so a checkout
  whose change edits the plan's own `protected_apparatus` is judged against
  the edited list. Whether a plan is linked at all is likewise the caller's
  choice: a request naming no `plan` seals an unlinked receipt, to which this
  requirement adds no reason. This requirement therefore guards a caller that
  asks the question honestly; it does not detect a caller that omits the
  plan, or the paths, it would be refused for.

## Dependencies

- engineering-assurance FR-024 defines `protected_apparatus`,
  `negative_controls`, and their entry grammar.
- FR-065 defines change-assurance verification and the reason/outcome
  precedence this requirement's reasons fold into.
- FR-110 defines the same protection at measurement intake and comparison;
  this requirement is its change-assurance-side counterpart, applied to a
  diff rather than a recorded set.
