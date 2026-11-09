<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Where this crate and the retained TypeScript differ

`src/change-assurance/` **was** the oracle for this port; quoin#457 cut the CLI
surface over to this crate and deleted it. What follows is where the two trees
could be told apart outside the behaviour this crate now specifies for itself
(FR-063, FR-064, FR-065: identity-addressed records and attestations, events
ordered by `sequence`, receipts re-derived and compared byte for byte), and why
each difference is what it is. Each is a refusal the oracle cannot express, or
a case the oracle answers by throwing.

## §1 — `exact()` names the first undeclared member in a different order

The oracle walks the retained object in JavaScript insertion order and reports
the first undeclared member it meets. `JsonObject` is a `BTreeMap`, so this
crate reports the first in code-point order. Both refuse the same records; only
the member named in the message differs when an object carries two undeclared
members at once.

The same fact makes `normalizeAttestation`'s environment sort a no-op here: the
environment arrives sorted because it is a map, not because anything sorted it.

## §2 — a revision or size beyond 2^63

`Revision` and `retained_output.size_bytes` are `u64`. The oracle holds them as
doubles and accepts any integral value up to 2^53 exactly and, beyond that,
whatever the double rounds to. A record declaring a revision of 1e30 is
accepted by the oracle and refused here.

The refusal is the stricter answer and it is kept: a revision that cannot be
represented exactly is a revision that cannot be compared, and lineage is
nothing but comparison. No such record exists in any retained store — revisions
are written by this family, starting at 1.

## §3 — a parent whose `revision` is not a number

`verifyLineage` sorts parents with `a.revision - b.revision` **before** any
parent is validated. When `revision` is not a number that subtraction is `NaN`,
and `Array.prototype.sort` with a comparator returning `NaN` has
implementation-defined results. This crate treats a non-numeric revision as
`0.0`, so such a parent sorts to the front.

Where it lands does not change the verdict: the parent is validated inside the
walk and a non-numeric revision is refused there, so the chain fails either
way. Only the _position_ reported in `LineageParent { index }` can differ, and
the receipt does not carry it.
