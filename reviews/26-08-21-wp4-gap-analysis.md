---
id: SR-013
title: "Gap analysis — WP4 ticket closures (#164–#171, #135, #136, #138, #177–#179, #183) and the #172 spine"
type: SpecReview
analysis: gap-analysis
scope: "spec/functional/FR-029..FR-032, spec/functional/FR-038..FR-040, spec/non-functional/NFR-001..NFR-009, spec/matrix.md, spec/log.md, src/, tests/, evals/, Makefile"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-029"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-031"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-039"
    type: "reviews"
---

## Summary

Ticket-by-ticket acceptance audit of the fifteen closures the WP4 session
landed on `main` (`82909bf..1ffef84` plus the two SR-012 fix commits), and a
completeness check of EPIC #172's quoin-side spine. Every closed ticket's
acceptance holds with test or inspection evidence; the epic's eight quoin
children (#164–#171) are all resolved and the epic correctly stays open for
the battle-test re-run. One acceptance held only by inspection until SR-012's
mutation check strengthened it (FND-001 there, fixed in-batch). No gap blocks
the dependent step.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                  | Refs                              |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------- |
| FND-001 | low      | FR-029-AC-11 was closed with its oracle untested (the maxBuffer raise itself); repaired in-batch by SR-012 FND-001 (`d7b5a34`). Recorded here so the closure audit stays honest about what held on its own.              | spec/functional/FR-029:… → SR-012 |
| FND-002 | low      | The #176 interim pin (`^0.27.0`) leaves the release-drift workflow's real target (a published quoin tag) still pending — expected, tracked on #174/#176 for the LATER dep-bump step; nothing in this batch regresses it. | package.json:101 → #176           |

## Ticket acceptance, with evidence

| Ticket                                              | Acceptance holds?                           | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                             |
| --------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| #164 maxBuffer + cause-named errors                 | **Yes**                                     | 64 MiB cap at all three `execFileSync` sites (`src/quire/exec.ts`); kill shapes reported by cause with no stderr appended, non-zero exit still surfaces stderr — TC-254..TC-257 (`tests/quire-exec.test.ts`), plus the SR-012 success-path test pinning the raise itself. FR-029-AC-10..AC-12; CR-028.                                                                                                                               |
| #165 unknown-method on unbound obligations          | **Yes**                                     | `unknownMethodFinding` extracted and asked before the binding guard; unbound + uncatalogued yields BOTH `undischarged` and `unknown-method`; `healthy` requires nothing found at all — TC-264 (unit) and TC-265 (command, no store, real fake-quire run). Mutation M3 killed. FR-032-AC-14; CR-032.                                                                                                                                  |
| #166 parameters read                                | **Yes**                                     | `parameters` threads `factsFor → ObligationFacts → characteristicsOf`; `target`/`threshold` mint `quantified-threshold` structurally; the ticket's own NFR-022-M-12 record now draws `performance-benchmarking` — TC-266. FR-031-AC-19; CR-033.                                                                                                                                                                                      |
| #167 compound tokens                                | **Yes**                                     | `matchesOutsideCompound` discards fragment matches glued by a hyphen; `unsafe-audit` mints nothing while named compounds still match; `layering`/`module-boundary` widened with corpus-verbatim phrasings so `architecture-conformance` is reachable — TC-267/TC-268, corpus gates TC-247/TC-248 still green. Mutation M1 killed. FR-031-AC-20/21; CR-034.                                                                           |
| #168 three-state advise                             | **Yes**                                     | `uncatalogued` split from `mismatch` via the engine's CR-091 `value` join; degraded mode explicit, never a misclassification (TC-275); filters union and the footer tallies the full population (TC-276); the five battle strings classify uncatalogued end-to-end (TC-274); carry-ahead schema provenance verified byte-identical against quire-rs v0.41.0 + `87a1869` with the recorded hash matching. FR-031-AC-22..24; CR-041.   |
| #169 ratchet honesty                                | **Yes**                                     | Label and JSON `ratchet` key on `baseline !== null`; a baseline-less `--ratchet` names the missing path and the writing command — TC-258..TC-260, first end-to-end audit-command tests. Mutation M2 killed. FR-032-AC-13; CR-029.                                                                                                                                                                                                    |
| #170 assurance reason + case-insensitive claim-type | **Yes**                                     | `reason` present exactly when `claims` is empty, naming the searched types as spelled; matching lowercased both sides; flag help states REPLACES — TC-261/TC-262. FR-040-AC-13/14; CR-030.                                                                                                                                                                                                                                           |
| #171 gc takes no --module                           | **Yes** (question resolved "correct as-is") | Help states why; TC-263 pins the flag set exactly (`repo`/`dry-run`/`json`) so a future `--module` must arrive with semantics; gc command surface gains its missing tests. FR-030-AC-8; CR-031.                                                                                                                                                                                                                                      |
| #135 brace glob is a load error                     | **Yes**                                     | `globToRegExp` throws on `{`/`}` naming the glob and the remedy; `absentFiles` can no longer pass vacuously; the harness's assert lib gains its first vitest coverage — TC-270 (`tests/eval-assert.test.ts`). FR-038-AC-10; CR-036.                                                                                                                                                                                                  |
| #136 orphaned NFR traces                            | **Yes, deliberately partial**               | Seven of eight NFRs gain `traces_to`; three edges spot-checked against the StR/NFR documents and found argued from them (including both divergences from the ticket's suggestions); `quoin assurance --repo .` lists exactly NFR-006 unreachable, and the ticket correctly **stays open** on that one document rather than minting a fabricated parent. CR-037.                                                                      |
| #138 metric discriminator                           | **Yes**                                     | `RunEntry.metric` written by the adapter (`MUTATION_SCORE_METRIC`, one shared constant); `scoresFor` drops its catalog parameter at both call sites; no tool-string fallback (an unlabelled `cargo-mutants` score is `unmeasured`); TC-239's no-catalog-silence rule retired in place — TC-269, TC-219 migrated. FR-039-AC-10..12 rewritten; CR-035.                                                                                 |
| #177 vocab drift gate                               | **Yes**                                     | TC-271 parses four seams (SKILL.md declaration, Markers list, both templates) against the **installed** manifest — the same seam TC-118 uses, so a pin bump re-tests in the same run; `🚧 Partial` dropped from the example and the note-word check keeps it out. Red-verified (M5: dropping `⛔` fails 3/5 assertions). NFR-005-AC-1; NFR-005 moves Review → Partial naming which half is enforced; CR-038.                         |
| #178 TC-116 identity                                | **Yes**                                     | The v0.41.0 assertions moved under TC-116's own describe; file header reads TC-110..TC-118; "every optional key" derived from the schema (`properties − required`) so a refresh fails the fixture until it carries the new key; matrix row and FR-029-AC-7 rewritten to the actual oracle. Falsifiability-checked (M4: narrowing the fixture fails naming the key). CR-039.                                                          |
| #179 interface-vs-schema gate                       | **Yes**                                     | TC-272: one `Required<Interface>` sample per `$defs` entry, key equality both directions, full-payload validation, and a spawned `tsc --noEmit` making the compile half a red test (vitest strips types; #180's dts diagnostics deliberately not inherited via file-list mode). Red-verified (M6: interface-side `form` deletion fails only the tsc gate — exactly the direction the runtime half cannot see). FR-029-AC-13; CR-040. |
| #183 marker + validate gate                         | **Yes**                                     | TC-239's Status is `⛔ Retired` (the vocabulary's own retirement marker, binding and note untouched per CR-035); `make validate` runs the repo-pinned quire-cli over `spec/`, `plan/`, `reviews/` and is a dependency of `make test`, so an unadmitted cell fails the local gate and CI alike. Red-verified: rewording the cell back to `➖` exits 1.                                                                                |

## EPIC #172 — quoin-side spine

The epic's quoin children are #164 (P0) and #165–#171 (P1/P2). All eight are
closed above with evidence, in the epic's own priority order (#164 first, which
unblocked re-evaluating the rest). The adjacent closures (#135, #138) and the
review follow-ups (#177–#179, #183) clear the batch's own escapes. What
remains on the epic is deliberately **not** quoin code:

- the battle-test re-run against filament-ide-rs (the epic's reason to exist —
  it must NOT be closed here);
- the quire-cli consumption-surface work (quire-cli#51/#53, other repo);
- the module-pin bump and its reconcile defect (#174) plus the release-drift
  restoration (#176), which are the **later dep-bump step this analysis
  gates**, not part of WP4.

The quoin-side spine is complete: every command the battle test found broken
(`assurance`, `advise`, `evidence audit`/`record`/`affirm`/`baseline`) has its
defect fixed and its fix pinned by a test that was shown to fail without it.

## Gates

- `make lint` — green (eslint + prettier over the batch and the artifacts).
- `make test` — green, and now transitively runs `make validate`, so every
  spec/plan/reviews document in this batch passed `quire validate`
  structurally.
- Mutation/red-verification: seven checks, six killed on first run, the
  seventh (M7) converted from survivor to killed by an in-batch fix (SR-012).

## Verdict

quoin release surface: GO
