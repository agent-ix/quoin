<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Where this crate and the retained TypeScript differ

`src/change-assurance/` is the oracle for this port, and
`tests/fixtures/oracle.json` is what it produced. Every scenario in that
capture is reproduced byte for byte — 38 verifications, 4 records, 4
attestations and 6 ix-flow events. What follows is everything outside the
capture where the two trees can be told apart, and why each one is what it is.
None of these is a defect the port hides: each is either a refusal the oracle
cannot express, or a case the oracle answers by throwing.

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

## §4 — a receipt the oracle cannot build

Three inputs make `verifyChangeAssurance` **throw while building its own
receipt**, because the receipt it assembled fails `validateReceipt`:

1. `genesis-with-a-parent`. The lineage check is built with
   `check(reasons, "invalid")` while `parent_missing` is in the _incomplete_
   precedence set, so the receipt disagrees with its own reason precedence.
   The consequence is that **`parent_missing` is unreachable as a receipt
   reason in the retained TypeScript**, and the reason census in
   `tests/tc_455_verify.rs` records 34 of 35 reasons reached for that
   reason.
2. A retained attestation whose `retained_output.digest` is malformed. The
   oracle copies it into the receipt unvalidated and `validateReceipt` then
   refuses the receipt.
3. The same for a malformed `attestation_digest`.

Case 1 is reproduced exactly: this crate builds the same check, seals it, and
`verify_receipt` refuses it, so the refusal happens where the oracle's does and
`tc_455_refusals_happen_where_the_oracle_refuses` asserts it.

Cases 2 and 3 are **not** reproduced. This crate emits `null` for a digest it
cannot parse and still returns a receipt. Throwing there would mean that one
malformed member inside one retained attestation destroys the verification of
every other proof in the record, and the receipt is the only artifact that
would have recorded why. The receipt still cites the attestation by its
selection digest, so nothing is lost from the trail.
