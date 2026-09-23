<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Where this crate and the retained TypeScript differ

`src/change-assurance/` **was** the oracle for this port; quoin#457 cut the CLI
surface over to this crate and deleted it. `tests/fixtures/oracle.json` is what
it produced and is now the only witness to what it said — so every statement
below that used to be checkable against that tree has been restated as
something checkable against this one, or is marked as no longer testable. Every scenario in that
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
   reason**, and the reason census in `tests/tc_455_verify.rs` records 34 of
   35 reasons reached for that reason.

   This is not a property of the TypeScript. The port inherited it exactly —
   `Reason::ParentMissing` declares its precedence class `incomplete` while
   `verify::verify_change_assurance` builds the lineage check with
   `Check::from_reasons(.., Outcome::Invalid)` — so any input reaching the
   reason is refused, not only the one the capture holds.
   `tc_455_parent_missing_is_structurally_unreachable_in_this_crate` asserts
   both halves over this crate's own code, so the census figure stays
   falsifiable now that the TypeScript is gone. **It is deliberately not
   fixed here**: quoin#457 is a cutover, and changing the verdict for an input
   both trees refuse would be a divergence introduced by a deletion.

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

## §5 — the cutover (quoin#457)

`src/commands/change-assurance/` now reaches these contracts through
`quoin-core change_assurance.*`. The following differ from what the deleted
TypeScript did, and each is an accepted, declared consequence of the move.

1. **A malformed `retained_output.digest` or `attestation_digest` changes the
   exit status.** Per §4 cases 2 and 3, the TypeScript threw; through the
   engine the request is refused with `Invalid` (exit 3), regraded by the
   command to exit 2 — never `Internal` (exit 4). Asserted by
   `quoin-core` `tc_457_change_assurance_boundary.rs` ::
   `tc_457_608`, which requires exit 3 and forbids 4.

2. **Every operation now carries a declared input ceiling.** `seal_attestation`
   and `intake` read their bytes with `readFileSync`, which had no bound; each
   `change_assurance.*` operation now applies a `MAX_*_BYTES` before any host is
   consulted. An input larger than the bound is refused where it used to be
   read. All six bounds are entered in
   `tc_412_the_transport_ceiling_stays_above_every_domain_bound`.

3. **`--audits` entries are shape-checked.** The request type is a serde struct
   with `deny_unknown_fields`, so an audit entry carrying an undeclared member
   is refused instead of ignored.

4. **Documents cross the boundary as lowercase hex of the producer's exact
   bytes**, never re-serialized JSON, so `parse_strict_json`'s decisions
   (duplicate members, BOM, non-finite numbers, trailing content) are taken over
   the bytes the caller held.

5. **`seal-record` refuses a body supplying `digest` in the engine.** The
   TypeScript refused it in `seal-record.ts`; the check moved into
   `ops::change_assurance::seal_record`, where it applies to every caller of the
   operation and not only to the command.

6. **FR-068-CON-1 was amended.** The surface used to spawn nothing at all. It
   now starts exactly one process — the engine — through `src/core/exec.ts`.
   `tests/change-assurance-command-surface.test.ts` asserts that positively
   (every engine-reaching command goes through that one module) as well as
   negatively (no command reaches `child_process`, Git, or the network).

## §6 — a required `strength` member the oracle never carried (PLAT-972)

`ProofAttestationV1` now requires `strength`, one of Engineering Assurance's
four `ClaimStrength` wire names (FR-022, PLAT-971): `proven`,
`bounded-checked`, `tested` or `observed`. The oracle predates the EA
claim-strength vocabulary and never emitted or read this member; there is
nothing to diverge from, only a member to add.

The captured `tests/fixtures/oracle.json` was amended in place to carry it:
`strength` was added to every attestation the file captured (cycling through
all four values so all four are exercised by the existing "reproduces the
oracle" tests, not only by `tc_455_every_claim_strength_round_trips`), each
digest-self-consistent attestation was resealed so its `digest` covers the new
member, and every `verifications[].receipt` was regenerated with this crate's
own `verify_change_assurance` against the amended inputs. Two attestations
were deliberately left digest-inconsistent — `attestation-digest-mismatch`'s
own attestation, and the fixture whose `result` is the invalid `"maybe"` — so
the negative-test cases they exist to exercise (`attestation_digest_mismatch`,
a schema refusal unrelated to `strength`) still fire exactly as before.

Regenerating receipts with this crate's own verifier would hide a port defect
if anything but digests had moved, so that was measured, not assumed: a
structural diff of the amended file against its pre-PLAT-972 capture shows
only added `strength` members and changed digest members (`digest`,
`attestation_digest`, and each receipt's `digest`, which cover the attestation
digests). No receipt outcome, reason, proof state or selection moved — every
verdict in the file is still the oracle's.

The `strength` type itself carries no ordering (deliberately — see
`engineering_assurance::claim_strength`'s compile-time probe): this crate
never compares, ranks or defaults it. A missing `strength` is refused by name
(`FieldFailure::Missing`), never inferred from `result` or defaulted to one of
the four values.

## §7 — `governing_plan_id` and `diff_paths_supplied`, two required members the oracle never carried (PLAT-1015)

`VerificationReceiptV1` now requires `governing_plan_id` and
`diff_paths_supplied`, recording which `MeasurementPlan` (if any) and whether
`diff_paths` (even an empty one) a receipt was actually judged against —
PLAT-997 wired `diff_paths`/`plan` through `change_assurance.receipt` but left
the sealed receipt unable to say whether either was ever supplied, versus
genuinely not applicable. The oracle predates both PLAT-964's plan link and
this ticket, so every captured `verifications[].input` carries neither, and
the same "nothing to diverge from, only a member to add" situation as §6
applies.

`tests/fixtures/oracle.json`'s `verifications[].receipt` entries were
regenerated in place the same way, restricted to the `receipt` member alone
(`records`, `attestations` and every `verifications[].input` are untouched):
each was rebuilt with this crate's own `verify_change_assurance` against its
existing (unmodified) input, which for every captured scenario means
`governing_plan_id: null` and `diff_paths_supplied: false` — none of the 37
regenerated receipts differed from the oracle's on any member but `digest`,
`governing_plan_id` and `diff_paths_supplied`, checked structurally rather
than assumed, exactly as §6 checked for `strength`. No outcome, reason, proof
state or selection moved.

`quoin-change-assurance`'s `read_sealed` reads both members as optional
(`Fields::exact_with_optional`), not required, precisely so a receipt sealed
before this ticket — this fixture's own pre-PLAT-1015 form, or anything a
caller already retained — still re-verifies: absence reads back as `None`
governing plan and `diff_paths_supplied: false`, the fact that receipt
genuinely never tracked either. Only `write_receipt` (what this crate seals
from here on) always emits both.
