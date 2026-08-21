---
id: SR-010
title: "Retroactive code review — PRs #163, #173, #175 (⚠️ retirement, v0.41.0 contract, slash sweep)"
type: SpecReview
analysis: code-review
scope: "skills/spec-matrix/, skills/gap-analysis/references/step-3-matrix-verification.md, package.json, pnpm-lock.yaml, src/quire/, tests/quire-contract.test.ts, tests/cli.test.ts"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-029"
    type: "reviews"
---

## Summary

Retroactive review of the three PRs merged to main on 2026-08-21 without review: **#163**
(retire the `⚠️` status marker from the skills vocabulary, merge `455b310`), **#173** (consume
the quire-rs v0.41.0 output contract, merge `78e6287`), **#175** (slash→comma trace-chain
normalization, merge `bccf21a`). All three were merged by their author with zero reviews and no
CI runs — the latter by design (CI is manual-only in this ecosystem), which makes review the
only gate, and it did not run. This artifact reviews the batch **as merged**; every defect is
already ticketed (#174, #176, #177, #178, #179) and none is fixed here.

Each seed finding from the exploration pass was re-verified against the merged diffs, the
registry, the working tree, and — for FND-008 — a fixture experiment against the installed
`quire` binary. One seed finding was **refined** by that experiment (FND-008: the #175 edit is
a coverage no-op in this repo, not a novel untested binding).

All three PRs are **unreleased**: the latest tag `v0.21.2` peels to PR #162, and
`v0.21.2..main` is exactly these three merges. No published quoin carries any defect below.

## Verdict

**FAIL** — two `high` findings (#173 left main unbuildable). The verdict records what the
review found; remediation is tracked on the tickets, not performed here.

## Findings

| ID      | Severity | Summary                                                                                                | Refs                                    |
| ------- | -------- | ------------------------------------------------------------------------------------------------------ | --------------------------------------- |
| FND-001 | high     | #173: devDependency `@agent-ix/quire-cli` widened to `^0.28.0` — a version that does not exist on npm  | package.json:100 → #176                 |
| FND-002 | high     | #173: `pnpm-lock.yaml` not regenerated — still resolves quire-cli 0.26.0, frozen-lockfile install fails | pnpm-lock.yaml:116 → #176               |
| FND-003 | medium   | #173: new `TC-116` test nested under TC-111's `describe`, and the TC-116 id reused for a different oracle | tests/quire-contract.test.ts:93 → #178  |
| FND-004 | medium   | #173: matrix row TC-116 and FR-029-AC-7 not updated for the reused id — same tag, two oracles          | spec/matrix.md:263 → #178               |
| FND-005 | medium   | #173: `ImplementsRecord` drift caught by hand, not by a gate — interface-vs-schema still unchecked     | src/quire/types.ts:66 → #179            |
| FND-006 | medium   | #173: intended default-modules pin bump excluded because it makes TC-118 fail — reconcile defect       | default-modules.yaml:13 → #174          |
| FND-007 | medium   | #163: nothing couples the skills' status vocabulary to the module manifest — next drift is undetectable | skills/spec-matrix/SKILL.md:130 → #177  |
| FND-008 | low      | #175: trace-binding semantics changed with no test of the effect; in this repo the edit is a coverage no-op | tests/cli.test.ts:667                   |
| FND-009 | low      | #163: `🚧 Partial` in the example rows re-admits two forms for one meaning as annotation text          | skills/spec-matrix/assets/test-matrix-example.md:53 → #177 |

## Detail

### FND-001 — the range could never have resolved anything (#176)

#173 widened `@agent-ix/quire-cli` from `^0.26.0` to `^0.28.0` to reach the CLI carrying the
v0.41.0 engine. Verified against the registry on 2026-08-21: `npm view @agent-ix/quire-cli
versions` tops out at **0.27.0**. quire-cli v0.28.0 is tagged in its repo but was never
published (no GitHub release, no release.yml run, no npm publish — see quire-cli#52 for the
related version-stamping failure). On a 0.x range `^0.28.0` means `>=0.28.0 <0.29.0`, so the
new pin resolves **nothing**: a fresh `pnpm install` on main fails outright. The PR's own
rationale — that `^0.26.0` could not reach 0.27/0.28 "by construction" — is correct, and the
fix re-created the same unreachability one notch higher, against a version that does not exist.

Remediation path (per WP2/WP5 of the QA plan): interim pin to `^0.27.0` (the newest published
version), re-bump to `^0.29.0` once the consolidated quire-cli release ships.

### FND-002 — the lockfile still says 0.26.0 (#176)

`pnpm-lock.yaml` was not touched by #173. It records `@agent-ix/quire-cli@0.26.0` (importer
spec and all four platform binaries). Two independent breaks stack:

1. `pnpm install --frozen-lockfile` fails with a lockfile/manifest mismatch (lockfile 0.26.0
   vs manifest `^0.28.0`).
2. Regenerating the lockfile cannot fix it either, because of FND-001 — the range is
   unresolvable on the registry.

`.github/workflows/release-drift.yml:32` runs `pnpm install --frozen-lockfile` as its first
step, so the drift guard dies before it can measure anything (details in SR-011, which records
the workflow-breakage state).

### FND-003 — TC-116 in the wrong describe, and the id minted twice (#178)

The new test (tests/quire-contract.test.ts:93) is
`it("TC-116 accepts the v0.41.0 optional keys, and rejects a malformed one")` — placed
**inside** `describe("TC-111 a conformant payload validates")`. The file already has a
top-level `describe("TC-116 optional keys are optional and absence is not emptiness")` at
line 230, which is the symbol the matrix row binds. Two consequences:

- The TC-111 block now contains an assertion belonging to a different test case; the block's
  title no longer describes its contents.
- `TC-116` is bound to two different oracles under one id — the pre-existing block's oracle is
  "malformed **statement hash** rejected"; the new test's oracle is "malformed
  **`undeclared_statuses` entry** (missing required `status`) rejected". This is the same
  duplicate-TC-id defect class the batch shipped in quire-rs (TC-943 ×2, TC-944 ×2).

### FND-004 — matrix and AC not updated for the reuse (#178)

`spec/matrix.md:263` still reads: "TC-116 | A payload omitting every optional key validates,
one carrying every optional key validates, and a malformed statement hash is rejected | Unit |
P0 | FR-029-AC-7 | ✅". `spec/functional/FR-029-consume-the-quire-json-contract.md:107`
(FR-029-AC-7) carries the same text. Neither mentions the v0.41.0 keys or the new rejection
oracle, so the matrix row is ✅ over a strictly narrower claim than what the tag now backs.
The reconciler cannot see this — both tests mint `TC-116`, the row is backed either way — so
it will not surface as an unbacked row or a status lie. Gap-analysis detail in SR-011.

### FND-005 — the drift was found by reading, and reading is not a gate (#179)

The vendored coverage schema has carried `implements` since quire-rs v0.39 (CR-080);
`types.ts` lacked the key until #173 added `ImplementsRecord` — two releases of drift in the
one direction nothing checks, exactly as the PR body says. The contract test validates
payloads against the **schema** (TC-110 checks the schema's hash; TC-111/112/116 validate
fixtures against it) and never validates the **interface** against the schema. #173 fixed the
instance and added a candid comment at src/quire/types.ts:155, but no conformance test — the
next optional key added to the published schema drifts the same way. #179 tracks the gate.

### FND-006 — the excluded pin bump, and what it protects (#174)

#173 deliberately excluded the `default-modules.yaml` bump to iso 0.18.0 / process 0.23.0:
bisected to that file alone, TC-118's anti-vacuity assertion fails ("the fixture must mint an
`implements` edge, or this guard is vacuous") because `reconcile(..., { mode: "lazy" })`
materializes the changed refs into a module set that loads **no `trace_targets`** — while the
identical command by hand against the same installed modules works. Splitting this out rather
than forcing it was the right call, and TC-118's guard did exactly its job. The defect is in
reconcile and is #174; the blast radius (the reconcile in `tests/global-setup.ts` rewrites the
**shared** `~/.ix/filament/modules`) is analyzed in SR-011. Consequence as merged:
`default-modules.yaml` still pins iso `v0.17.0` / process `v0.21.1`, so quoin's defaults do
not yet carry the `⚠️` retirement that #163 taught the skills — the skills and the shipped
module set currently disagree, which is FND-007's coupling gap made concrete.

### FND-007 — the generator is uncoupled from the contract it generates for (#177)

#163's premise is correct: the skills are the generator, and retiring `⚠️` in the module while
`spec-matrix` keeps teaching it would re-contaminate the corpus. But the coupling that premise
implies does not exist. Verified: the only references to `spec-matrix` in src/ or tests/ are
flow-name mappings (`src/flows.ts:14`, `tests/flows.test.ts:16`); nothing reads the Status
vocabulary out of `skills/spec-matrix/SKILL.md` (or its template/example assets) and compares
it against the module manifest's `traceability.status` declaration. The sweep that produced
#163 was manual, and the next vocabulary change in spec-artifacts-process will drift the
skills again with nothing to notice. #177 tracks the drift gate.

### FND-008 — verified real in form, no-op in effect (refined from seed)

#175 changed one comment line (tests/cli.test.ts:667): `FR-025-AC-6 / FR-023-AC-4` →
`FR-025-AC-6, FR-023-AC-4`. Fixture experiment against the installed quire (fresh scope, one
FR with two ACs, one test):

| comment form                                | backed | unbacked      |
| ------------------------------------------- | ------ | ------------- |
| `// FR-001-AC-1 / FR-001-AC-2: …`           | 1/2    | `FR-001-AC-2` |
| `// FR-001-AC-1, FR-001-AC-2: …`            | 2/2    | —             |
| `// Trace:` lines only                      | 2/2    | —             |
| comma prose + `// Trace:` lines             | 2/2    | —             |

So the legacy prose form **does** mint bindings and the slash **does** drop the second id —
the PR's mechanism claim is verified. But in quoin's actual file, explicit `// Trace:
FR-023-AC-4` / `// Trace: FR-025-AC-6` lines (tests/cli.test.ts:672-673) sit on the **same
test** and already bound both ids before the edit. The normalization added a duplicate binding,
not a new one: coverage totals are unchanged, and matrix rows TC-078/TC-088 were backed before
and after. This refines the seed finding: the *class* (a trace-binding semantic change landed
with only the sweep's "no new untracked symbols" gate, no test of the binding itself) is the
same one quire-cli#54 instantiated — but there the minted FR-015-AC-5 binding was novel and
untested, while here it is redundant and harmless. Severity low; no quoin ticket warranted
beyond the ecosystem-level sweep-harness ticket (quire-rs NR-6 scope).

### FND-009 — "Partial" survives as annotation text (#177, secondary)

#163 argued "a second form for one meaning enforces nothing" — and then wrote
`🚧 Partial` into both retained example rows (test-matrix-example.md:53-54) and
`🚧 In Progress / Partial / Pending` into the Markers list. The *marker* vocabulary is clean
(the module classes the marker, not the text), but the example is the thing agents copy, and
it now teaches "Partial" as a sub-state spelled in prose — the same two-forms-one-meaning
shape one layer down, invisible to the module by construction. Folded into #177 (it is the
second clause of that ticket).

## Process observations

- All three PRs: merged by author, zero reviewers, zero CI runs. CI-on-PR absence is by
  design (manual-only policy); the compensating control — review — did not happen. This
  artifact is the retroactive compensation.
- #173's PR body is unusually candid: it names the TC-118 exclusion, the ImplementsRecord
  drift direction, and the tautology trap. Every high finding here was *discoverable from the
  PR body plus a registry check* — a pre-merge review would likely have caught FND-001/002.
- `npm test` was green (462/463 passing) for #163/#173 because vitest never resolves the
  devDependency range — the breakage is install-time only, which is why it survived.
