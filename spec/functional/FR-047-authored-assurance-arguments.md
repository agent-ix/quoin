---
id: FR-047
title: "Authored assurance arguments and explicit sufficiency decisions"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-040"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-046"
    type: "requires"
---

# FR-047: Authored assurance arguments and explicit sufficiency decisions

## Description

Quoin SHALL render the module-owned `AssuranceArgument` contract without
manufacturing, narrowing, or rewriting its top claim. Each authored
sufficiency criterion becomes supported only through a current decision that
names the decision maker, authority, source revision, evidence digest, and
evidence references.

Assumptions, participants, authority, independence, challenges, accepted
risks, and expiries SHALL remain visible as separate facts. Missing decisions,
invalidated or overdue assumptions, unresolved challenges, and expired risk
acceptances SHALL keep the top claim open. The view SHALL NOT emit an aggregate
score or claim compatibility with an external argument notation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-047-AC-1 | The view preserves the authored claim, subject, owner, participants, authority, and independence, and emits no score. | Test (TC-1131) |
| FR-047-AC-2 | A criterion without an explicit sufficiency decision remains open even when evidence references exist elsewhere. | Test (TC-1132) |
| FR-047-AC-3 | Expired or future decisions and accepted assumptions whose review is due reopen their branch and the top claim. | Test (TC-1133) |
| FR-047-AC-4 | A resolved challenge requires resolution references; accepted risk additionally requires a current expiry. | Test (TC-1134) |
| FR-047-AC-5 | The closed authored contract, decision shape, uniqueness, timestamps, and digests are validated before rendering. | Test (TC-1135) |
| FR-047-AC-6 | Markdown and JSON preserve every open reason and render unchanged input deterministically. | Test (TC-1136) |
| FR-047-AC-7 | An authored instant naming a day, hour, minute or second that does not exist is refused, rather than rolled forward into a different instant that then decides a reported status. | Test (TC-1712) |

## Constraints

- A test result or coverage result is evidence, not a sufficiency decision.
- The evaluator reads no wall clock; callers provide the `asOf` instant.

An instant is validated in full, not merely matched for shape. `Date.parse` does
not reject an impossible value — it rolls it, so `2026-02-30T00:00:00Z` becomes
March 2 and `T24:00:00Z` becomes the following day. The parsed number is then
compared against `asOf` to decide whether an assumption is due for review or an
accepted risk has expired, which makes a date nobody can have meant into a
reported status rather than into an error (`agent-ix/quoin#436`). The strict
reader is shared with the measurement surfaces rather than written a third time.
- The authored contract remains owned by the separately installed private
  engineering-assurance module.
- No external argument notation is emitted or claimed.

## Dependencies

- FR-040 supplies the existing read-only view and CLI surface.
- FR-046 supplies clause discharge populations when an argument cites them.
