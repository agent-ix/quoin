# Writing Good Requirements

Load this when authoring StR/US/FR/NFR artifacts. The templates and Quire enforce
*shape*; this file carries the *judgment* that makes a requirement good, not just
well-formed. Authoring well here means `spec-review` has less to send back.

## Scope: tight beats broad

A narrow spec that fully defines its slice beats a sprawling one full of gaps.
Resist widening scope to "cover everything" — an expansive, vague spec is worse than
a tight, sharp one.

Treat the `spec.md` §2.2 **Out of Scope** list as first-class. Deliberately decide
what you are **not** building and write it down — an empty Out-of-Scope section is a
smell, not a default. Naming non-goals is how you prevent scope creep during
implementation.

> **Bad:** §2.2 left empty; §2.1 In Scope reads "all user-facing features".
> **Good:** In Scope: "CSV import of contacts". Out of Scope: "XLSX/JSON import,
> de-duplication, field mapping UI — see future work."

## User stories: INVEST + anti-patterns

Aim for **INVEST** stories: **I**ndependent, **N**egotiable, **V**aluable,
**E**stimable, **S**mall, **T**estable.

Catch these anti-patterns while authoring, not in review:

- **Vagueness** — "make it faster" says nothing testable. Name the user, the want,
  and a measurable outcome.
- **Solution-prescribing** — "add a dropdown menu" prescribes a mechanism. A user
  story describes the *need*; the FRs choose the mechanism. If the US already names
  the implementation, you have skipped the problem.
- **Missing rationale** — every US carries a real "so that …". If you can't state the
  value, question whether the story belongs in the spec.
- **Internal focus** — "so that the team has less to maintain" benefits the team, not
  the user. Frame the value around the user/stakeholder outcome.

> **Bad:** "As a user, I want a dropdown to pick a timezone."
> **Good:** "As a traveling user, I want event times shown in my current timezone so
> that I don't miss meetings after I change locations." (The picker, if any, is an FR.)

## Acceptance criteria: cover the unhappy paths, be concrete

ACs are authored *here*, with the US/FR — there is no later matrix-writing step to
catch what they miss. The computed test matrix has one row per criterion, so a case with
no criterion has no row at all. Front-load it:

- **Happy / error / edge from the start.** Cover error paths, constraint boundaries,
  state transitions and edge cases while the behavior is fresh.
- **Negative cases.** State what must *not* happen, not just what should: rejects,
  refuses, never persists, leaves state unchanged.
- **Concrete and measurable.** "Returns HTTP 422 within 200ms", not "handles errors
  gracefully". Avoid subjective language (fast, robust, user-friendly) — if it can't
  be measured, it can't be verified.
- **Behavior, not implementation.** Assert the observable outcome, not how the code
  achieves it.

> **Bad:** "The system handles invalid input gracefully."
> **Good:** "Given a payload missing `email`, when POST /contacts is called, then the
> system returns HTTP 422 with error code `MISSING_FIELD` and persists no record."
