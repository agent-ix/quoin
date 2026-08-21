---
id: SR-011
title: "Gap analysis — v0.41.0 contract adoption (TC-116 oracle drift, TC-118 vacuity, release state)"
type: SpecReview
analysis: gap-analysis
scope: "spec/matrix.md, spec/functional/FR-029-consume-the-quire-json-contract.md, tests/quire-contract.test.ts, default-modules.yaml, .github/workflows/release-drift.yml"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-029"
    type: "reviews"
  - target: "ix://agent-ix/quoin/NFR-012"
    type: "references"
---

## Summary

Gap analysis of the v0.41.0 contract adoption (PR #173, with #163/#175 context) against the
Test Matrix, the FR-029 acceptance criteria, and the release machinery. Companion to SR-010
(retroactive code review of the same batch); findings there are cross-referenced, not
repeated.

Three gaps, none visible to the reconciler: an id reused across two oracles that the matrix
cannot distinguish (backed is backed), an end-to-end guard whose vacuity mode is now a known,
ticketed failure with machine-wide blast radius, and a release-drift guard that is dead at
exactly the moment main has unreleased, defective work — the state it exists to detect.

## Verdict

**FAIL** — the matrix asserts ✅ over a claim (TC-116/FR-029-AC-7) narrower than what the tag
now backs, and the repo's only release-lag gate cannot run. All gaps are ticketed (#174,
#176, #178); nothing is silently dropped.

## Findings

| ID      | Severity | Summary                                                                                         | Refs                                          |
| ------- | -------- | ----------------------------------------------------------------------------------------------- | --------------------------------------------- |
| FND-001 | medium   | TC-116 backs FR-029-AC-7 with two different oracles; matrix row and AC text describe only one   | spec/matrix.md:263 → #178                     |
| FND-002 | medium   | TC-118's vacuity mode is live under the intended pin bump; reconcile writes a shared directory  | default-modules.yaml:13 → #174                |
| FND-003 | high     | release-drift is unrunnable on current main; the drift it guards is exactly the current state   | .github/workflows/release-drift.yml:32 → #176 |
| FND-004 | low      | The version premise floor is sound, but the installed binary's version stamp is not trustworthy | src/quire/contract.ts:44                      |

## Release state (recorded as fact, not finding)

- Latest quoin tag: **v0.21.2**, peeling to PR #162 (`f4fd78f`, 2026-08-20, the AGPL
  relicense). `git log v0.21.2..main` is exactly `455b310` (#163), `78e6287` (#173),
  `bccf21a` (#175).
- Therefore **all three PRs are unreleased**. No published quoin package, no installer, and no
  plugin consumer carries the unresolvable `^0.28.0` pin, the stale lockfile, the reused
  TC-116 id, or the vocabulary changes. Every fix on #174/#176/#177/#178/#179 can land on
  main **before any quoin tag**, and the eventual release ships the corrected stack in one
  step. The remediation plan (WP2 interim `^0.27.0` pin → WP5 `^0.29.0` re-bump after the
  consolidated quire-cli release) is compatible with this window; nothing forces an
  intermediate tag.

## Detail

### FND-001 — one tag, two oracles, one row (#178)

The matrix row (spec/matrix.md:263):

> TC-116 | A payload omitting every optional key validates, one carrying every optional key
> validates, and a malformed statement hash is rejected | Unit | P0 | FR-029-AC-7 | ✅

FR-029-AC-7 (spec/functional/FR-029-consume-the-quire-json-contract.md:107) carries the same
sentence, verification `Test (TC-116)`.

What the tag binds on main now:

1. `describe("TC-116 optional keys are optional and absence is not emptiness")`
   (tests/quire-contract.test.ts:230) — the original: omit-all validates, carry-all validates
   (`no_symbol_rows`, `criteria`, `diagnostics`, `obligations`), reject a malformed
   **statement hash**. This is the oracle the row and the AC describe.
2. `it("TC-116 accepts the v0.41.0 optional keys, and rejects a malformed one")`
   (tests/quire-contract.test.ts:93, inside TC-111's describe — SR-010 FND-003) — the new:
   `undeclared_statuses` + `implements` accepted, reject an `undeclared_statuses` entry
   missing its required **`status`** key.

The mismatch is threefold:

- **The AC oracle is stale.** FR-029-AC-7 enumerates "every optional key", but the sentence's
  concrete rejection oracle (statement hash) belongs to the v0.39 key set. The v0.41.0 keys
  have a _different_ rejection oracle (required-key omission) that no spec text names.
- **The matrix cannot see it.** `quire coverage` reconciles row → tag → symbol. Both symbols
  mint `TC-116`, so the row is backed regardless of which oracle runs, and ✅ stands over an
  underspecified claim. Duplicate-id detection is a quire-rs gap (NR-5 in the QA plan; same
  class as the shipped TC-943 ×2 / TC-944 ×2 there) — quoin cannot lint its way out locally,
  but it can stop reusing ids.
- **Notably, the original describe (line 236-281) was NOT extended** to carry the new keys in
  its "carrying every optional key" payload — that payload still omits `undeclared_statuses`
  and `implements`, so the test named "carrying every optional key" no longer does. The
  claim's own text has silently narrowed relative to the schema, which is the precise drift
  shape FR-029 exists to prevent. (New finding; folded into #178's remit since the fix is the
  same edit.)

Remedy shape (on #178): move the new assertions to their own TC id (or into the TC-116
describe with its payload extended), update spec/matrix.md:263 and FR-029-AC-7 to name both
oracles, one id per oracle.

### FND-002 — TC-118's vacuity is a known, live failure mode with a shared-state blast radius (#174)

TC-118 (spec/matrix.md:265, FR-029-AC-9) is the repo's only contract check against the **real
emitter** — added by CR-027 precisely because every other coverage assertion validates a
hand-built fixture that by construction carries whatever the schema already allows. Its
anti-vacuity guard (tests/quire-contract.test.ts:374-378: the fixture **must** mint an
`implements` edge, or the run proves nothing) is well designed, and it worked: bumping
`default-modules.yaml` to iso 0.18.0 / process 0.23.0 makes it fail with
`model-mints-nothing` — `reconcile(..., { mode: "lazy" })` materializes the changed refs into
a module set that loads no `trace_targets`, `quire coverage` mints nothing, and a payload
without `implements` would validate against the _old_ schema too. The guard converted a
would-be silent vacuity into a loud failure; #173 excluded the bump and filed #174.

The gaps that remain, beyond the ticketed reconcile defect:

- **Shared mutable state.** The reconcile runs in `tests/global-setup.ts` and rewrites
  `~/.ix/filament/modules` — the directory every quire/quoin invocation on the machine reads.
  A quoin test run after a pin change can leave _every other repo's_ validation running
  against a module set that loads no traceability model. Until #174 is fixed, a pin bump on a
  branch is a machine-global hazard, not a repo-local one. (Supporting evidence: coverage runs
  on this machine currently emit `DuplicateArchetype: … contributed by modules
["spec-artifacts-process", "spec-artifacts-process"]; first-wins` — the shared module set
  already carries reconcile residue of some form.)
- **The intended state is unshipped.** Defaults still pin iso `v0.17.0` / process `v0.21.1`,
  so quoin ships module defaults that predate the `⚠️` retirement and the `source_exclude`
  schema key (two and five releases behind respectively, per #174). #163's skills and the
  shipped module set currently teach different vocabularies — the concrete instance of the
  coupling gap (#177, SR-010 FND-007).
- **TC-118 skips silently when quire is absent** (`ctx.skip()` at lines 326/332/386). Correct
  for a unit-test environment, but it means the one real-emitter check is environmental: a CI
  or clean-env run without the binary reports green with the contract unexercised. NFR-012's
  matrix row is already ⚠️/Partial on the module half (spec/matrix.md:99); the quire half is
  conditional in a way no matrix annotation records. Low, but worth a note on #174 or the
  matrix row when touched.

### FND-003 — the drift guard is dead in the exact state it guards against (#176)

`.github/workflows/release-drift.yml` exists to catch "work merged to main that never gets
released, so the published @agent-ix/quoin silently lags its source." Current state:

- Trigger is `workflow_dispatch` only (manual-CI policy). Last run: 2026-06-29, success —
  **before this batch merged**; the breakage is latent, not a red run.
- The next dispatch fails at step one: `pnpm install --frozen-lockfile` (line 32) dies on the
  lockfile/manifest mismatch (lockfile resolves quire-cli 0.26.0, manifest demands
  `^0.28.0`), and regeneration is also impossible while `^0.28.0` is unresolvable on npm
  (registry tops at 0.27.0 — SR-010 FND-001/002, ticket #176).
- Main is now 3 merges past v0.21.2 — precisely the drift the workflow measures — and the
  guard cannot run until #176 lands. The `manifests` and `pins` steps (lines 40/45) are also
  unreachable, so the plugin-manifest version check and the module-pin report are dark too.

Verification for #176's fix: `pnpm install --frozen-lockfile` green locally, then a manual
`release-drift` dispatch green (it will correctly _fail the drift check itself_ until a tag
ships — that failure is the guard working, distinct from the install failure that prevents it
from running at all).

### FND-004 — the floor is sound; the number it reads is not (cross-repo, no quoin ticket)

`QUIRE_CONTRACT.minimumCli` stays `0.21.0` after the re-vendor, and the comment block at
src/quire/contract.ts:45-60 is explicit that this is deliberate: a **contract** floor (first
release emitting these shapes), not a **capability** floor, with the drift risk documented.
That reasoning is verified and accepted — no gap in the pin itself.

The gap is upstream of it: TC-118's premise check reads `quire --version`, and the installed
binary on this machine reports **0.23.0** while actually being the quire-rs v0.41.0-era CLI
(quire-cli's Cargo.toml was never bumped past 0.23.0 — quire-cli#52; its release.yml smoke
test never compares binary version to tag). Today that mis-stamp is harmless here (0.23.0 ≥
0.21.0, premise passes, and the binary genuinely satisfies the contract). But any future
raise of `minimumCli` past 0.23.0 — e.g. to the first release emitting `undeclared_statuses`
— would make TC-118 wrongly reject or skip on a _current_ binary. Sequencing consequence:
**quire-cli#52 must land before quoin ever raises this floor.** Recorded here so the
dependency is visible from the quoin side; the defect and fix are quire-cli's.
