---
id: SR-012
title: "Code review — WP4 fix batch (82909bf..1ffef84: #164–#171, #135, #138, #177–#179, #183)"
type: SpecReview
analysis: code-review
scope: "src/quire/, src/advisor/, src/auditor/, src/assurance/, src/commands/, src/evidence/, evals/lib/assert.mjs, tests/, skills/spec-matrix/, spec/, Makefile"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-029"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-031"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "reviews"
---

## Summary

Pre-release review of the 18 unreviewed commits this session landed on `main`
(`82909bf..1ffef84`): the WP4 batches A–C (#164–#171, #135, #136, #138), the
review follow-ups (#177, #178, #179), and #183's validate gate. The batch was
read in full diff, spot-verified by seven mutation/red-verification checks, and
its contract carry-ahead was diffed against the upstream source it claims. One
medium finding (the headline #164 fix was unpinned by any test — the mutation
reverting it survived) and one low finding were fixed inline; two lows were
filed as #184 and #185; one low is recorded as unreachable. Baseline
`make lint` and `make test` (which now includes `make validate`) are green.

## Verdict

**CONDITIONAL** — no high findings; the one medium (FND-001) is fixed in this
batch (`d7b5a34`), the remaining lows are filed or recorded.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                                                                                                                                                                                                   | Refs                                                     | Escape Cause                    |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------- |
| FND-001 | medium   | #164's headline fix was unpinned: reverting `maxBuffer: QUIRE_MAX_BUFFER` in `runQuire` failed none of TC-254..TC-257 — the kill-path tests die ENOBUFS identically under either cap. FR-029-AC-11's oracle ("a payload over the 1 MiB default still runs") had no test. **Fixed**: TC-254 gains a 2 MiB success path (`d7b5a34`); the mutant now fails it.                                               | src/quire/exec.ts:45, tests/quire-exec.test.ts:46 → #164 | correct-requirement-no-evidence |
| FND-002 | low      | `runAdvise`'s `payload` parameter was dead: every TC-274..TC-276 caller faked PATH itself and passed the same payload in to be ignored, leaving the fixture the command runs against free to diverge from the payload the assertions reason about. **Fixed**: the helper now fakes quire from its own argument (`beeff12`).                                                                               | tests/advise-command.test.ts:267 → #168                  | correct-requirement-no-evidence |
| FND-003 | low      | 24 of 27 test files calling `mkdtempSync` never clean up; a full run leaks dozens of tmp dirs. Pre-existing repo pattern (not a WP4 regression), but the batch added ~15 new call sites following it. Filed for a one-seam fix rather than 27 edits.                                                                                                                                                      | tests/ (survey in issue) → #184                          | missing-requirement             |
| FND-004 | low      | `runQuireAllowFailure` got the cap but not #164's cause distinction: an ENOBUFS/signal kill returns truncated stdout with `ok: false`, indistinguishable from a non-zero exit; downstream (`advise` keeps non-zero `properties` payloads per #103) would blame the payload shape. Remote under the 64 MiB cap; filed. Also cosmetic: the spawn-failure branch can print `could not be run (undefined)`.   | src/quire/exec.ts:96-119 → #185                          | missing-requirement             |
| FND-005 | low      | `uncataloguedAuthoredMethods` on a mixed payload (some `uncatalogued-verification-method` diagnostics with `value`, some without) sets `degraded: true` **and** keeps the joined values, so the "reported as a mismatch, as before" warning would overstate the degradation. Unreachable from any single engine vintage (CR-091 engines always emit `value`; pre-CR-091 never do); recorded, not changed. | src/advisor/advise.ts:1045-1056                          | missing-requirement             |

## Mutation and red-verification checks

Seven checks, each reverting the guard/branch a new test claims to pin and
re-running the suite that should fail:

| Check | Mutation                                                                       | Result                                                                                                        |
| ----- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| M1    | #167: `characteristicsOf` back to bare `re.test` (compound guard bypassed)     | **Killed** — 3 TC-267 tests fail (`unsafe-audit` mints memory-safety)                                         |
| M2    | #169: `ratcheted` keyed on `flags.ratchet` again                               | **Killed** — TC-258 and TC-260 fail                                                                           |
| M3    | #165: `unknown-method` re-gated behind `bindings.length > 0`                   | **Killed** — TC-264 and TC-265 fail                                                                           |
| M4    | #178: `undeclared_statuses` dropped from TC-116's "every optional key" fixture | **Killed** — fails naming the missing key (the schema-derived "every")                                        |
| M5    | #177: `⛔` dropped from the SKILL.md `### Status` declaration                  | **Killed** — 3 of 5 TC-271 assertions fail                                                                    |
| M6    | #179: `form` deleted from the `ImplementsRecord` interface                     | **Killed** — TC-272's spawned-tsc gate fails (runtime half cannot see it, as the test's own comment predicts) |
| M7    | #164: `maxBuffer` removed from `runQuire`                                      | **SURVIVED** pre-fix (→ FND-001); killed by the new TC-254 success path                                       |

Plus #183's gate red-verified directly: rewording TC-239's Status back to
`➖ Retired` makes `pnpm run validate` exit 1 (grammar warnings stay advisory).

## Verified positively

- **#164 error branching**: a genuine non-zero exit still surfaces the child's
  own stderr (TC-256, and the `status != null` branch keeps the append); the
  three kill shapes report cause only, no stderr, no exit status
  (TC-254/255/257). The 64 MiB cap is set at all three `execFileSync` sites.
- **#168 degraded mode**: a value-less payload degrades to the two-state report
  with the explicit "engine predates vocabulary classification" warning and the
  five battle strings as mismatches (TC-275, both halves); no diagnostics at
  all is correctly NOT degraded. `uncatalogued` is exclusive with `mismatch`
  and survives `inconclusive` (TC-273); an empty authored cell is never
  uncatalogued.
- **#168 carry-ahead provenance**: the vendored `coverage-v1.schema.json` was
  diffed against quire-rs — it is exactly the `v0.41.0` tag plus the two CR-091
  shapes (`CoverageDiagnostic.value`, `vocabulary_coverage` /
  `$defs/VocabularyValueRecord`), and those two regions are byte-identical to
  `main` commit `87a1869` as `QUIRE_CONTRACT` claims. The other post-tag fields
  at `87a1869` (`shared_trace_ids`, `excluded_source_files`, `line`) are
  deliberately not carried. The recorded SHA-256 matches the file
  (`f40cca33…`), so TC-110's hash discipline holds.
- **#167 corpus regressions**: the full `STATEMENT_CHARACTERISTICS` producibility
  gates (TC-247/TC-248 in `tests/catalog-fact-join.test.ts`) and the whole
  advisor suite pass; named compounds (`fault[- ]toleran`, `fail-safe`,
  `memory[- ]safe(ty)?`, `thread([- ]safe(ty)?)?`) still mint, and the earlier
  worry that hyphen-glued stems regress was checked against the actual regexes —
  the reliability stem `tolerat` never matched `fault-tolerant` in the first
  place, and the compound is named explicitly.
- **#136 trace edges** (3 of 7 spot-checked against the documents):
  NFR-001→StR-005 is the missing mirror of StR-005's authored
  `satisfied_by: NFR-001`; NFR-003→StR-001's rationale ("an agent that must
  self-correct") echoes StR-001's audience sentence; NFR-005→StR-003's
  singularity claim is StR-003's, extended to workflows — the divergences from
  the ticket's suggested parents are argued and sound. `quoin assurance
--repo .` lists exactly `NFR-006` under "Reachable from no claim", as CR-037
  claims, and #136 correctly stays open on that one document.
- **Spec bookkeeping**: CR-028..CR-041 cover every ticket in the batch, one
  entry each, mutually consistent with the matrix rows (TC-254..TC-276 all
  resolve to the right FR ACs; TC-239 retired with the vocabulary's own `⛔`
  marker; TC-116's row now states its actual oracle). `make validate` is wired
  into `make test` and runs the repo-pinned quire-cli.

## Review discipline notes

- Test-standard lanes were applied in this repo's own idiom (vitest,
  `// TC-NNN` tracking tags, fake-binary-on-PATH seams); no `TODO`/`FIXME`/
  placeholder returns in the changed source; no coverage thresholds lowered;
  no suppressed warnings introduced.
- Mock boundaries: the new command tests fake at the process boundary (a `quire`
  script first on PATH — the established fake-ix-flow pattern, required by
  TC-203's string-literal architecture check) and spy only on oclif's
  `log`/`warn` output seam; no internal logic is mocked.
- The one dependency change (`^0.28.0` → `^0.27.0`, #176 interim) is a
  narrowing to a version that exists; `pnpm-lock.yaml` was regenerated to
  match, which is the pair SR-010's FND-001/002 found missing last time.
