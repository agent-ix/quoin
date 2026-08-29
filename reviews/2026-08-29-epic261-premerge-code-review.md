---
id: SR-035
title: "Code review — stabilize/263-verification pre-merge (epic #261)"
type: SpecReview
analysis: code-review
scope: "src/, scripts/, evals/lib/, tests/, skills/, spec/, .github/workflows/, Makefile"
review_set: subset
---

# Code review — stabilize/263-verification pre-merge (epic #261)

## Summary

Reviewed the full `main...stabilize/263-verification` diff (213 files; the
hand-written surface is ~15k lines across `src/`, `scripts/`, `evals/lib/`,
`tests/`, `skills/`, specs and CI, the remainder retained measurement JSON) at
tracking revision `15dabe9dc7a1bf4cb7e7bfcebfd7e198e89a7068`, including PR #295
(`assurance-status-report` skill, #294). All claimed validation reproduced
under the governed stack — lint clean, 731/731 tests with the locked Quire
build (CLI `bcface27`, quire-rs `ca7362d4` via `--locked` Cargo resolution),
33/33 tool-drift mutations rejected, 23/23 verification-stack invariants — and
the ambient-binary refusal was observed firing live against an ungoverned
`quire`. No high-severity defect was found in shipped product code or in PR
#295. Two high findings land in the unshipped instrumentation itself, both of
the class the honesty program exists to close: a ratchet that silently skips a
metric whose rate goes `null` (so a vanished labeled population exits green),
and freeze tooling that can only mint all-`pass` "independent review" evidence
with canned per-row rationales.

## Verdict

**FAIL** — two `high` findings (FND-001, FND-002), both confined to
measurement instrumentation that does not ship in the npm package; the product
surface, the focal skill, and the release path are clean, so the practical
disposition (merge now with P1 follow-ups vs. fix-first) is a maintainer call.

## Findings

| ID      | Severity | Summary                                                                                                                     | Refs                                                                                                                                      |
| ------- | -------- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| FND-001 | high     | Null-rate metrics skip the ratchet and `malformed[]` gates nothing, so a vanished span population runs green                | evals/lib/quality.mjs:786, scripts/lib/tier1-comparison.mjs:183                                                                           |
| FND-002 | high     | freeze-guidance-review can only emit `pass` outcomes with canned rationales and a fixed reviewer attribution                | scripts/freeze-guidance-review.mjs:88                                                                                                     |
| FND-003 | medium   | Tier-1 `quire validate` leg never checks `result.ok`; a timeout/crash scores as zero findings, not a failure                | scripts/lib/tier1-execution.mjs:278                                                                                                       |
| FND-004 | medium   | `--disposition correct` bulk-stamps every unadjudicated advisory row; 108/108 rulings minted by one command                 | scripts/freeze-advisory-adjudication.mjs:36                                                                                               |
| FND-005 | medium   | Actionability-v2 producer-authored `not_applicable` empties the denominator; null rate escapes the gate                     | evals/lib/quality.mjs:457, scripts/lib/tier1-comparison.mjs:149                                                                           |
| FND-006 | medium   | battletest falls back to ambient PATH `quire`, the exact selector bench-tier1 refuses by design                             | scripts/battletest.mjs:441                                                                                                                |
| FND-007 | medium   | Plan lookup keys on metric across all statuses; a `proposed` successor shadows the `active` plan and blocks recording       | src/measurement/validate.ts:39                                                                                                            |
| FND-008 | medium   | Library-API write accepts `NaN`/`Infinity`, stores `value: null`, and the whole store then fails on read                    | src/measurement/validate.ts:191                                                                                                           |
| FND-009 | medium   | Multi-word SARIF driver names (`GitHub CodeQL`) can never match `--tool`, making the record unrecordable                    | src/commands/evidence/record.ts:103                                                                                                       |
| FND-010 | medium   | Zero-assertion gate heuristic only matches same-line checks; a working two-line gate is flagged as gating nothing           | src/validators/gates.ts:91                                                                                                                |
| FND-011 | medium   | Catch-all around `loadCorpus()` converts any loader regression into a silent skip of the tier-1 suite                       | tests/bench-corpora.test.ts:41                                                                                                            |
| FND-012 | medium   | `mkdtempSync` fixtures without teardown in four test files recreate the quoin#184 leak class                                | tests/gate-validator.test.ts:20, tests/mock-inspection.test.ts:13, tests/evidence-audit-command.test.ts:258, tests/battletest.test.ts:210 |
| FND-013 | low      | Dead `.git` existence check (`&&` where `                                                                                   |                                                                                                                                           | ` is meant); non-repo dirs fall through to the parent repo | scripts/verification-stack.mjs:185 |
| FND-014 | low      | Empty `QUIRE` resolves to cwd, which passes the absolute-path guard and dies later as opaque spawn EACCES                   | scripts/verify-span-breadth.mjs:136                                                                                                       |
| FND-015 | low      | `process.exit(await main())` after a multi-MB `console.log` can truncate piped `--json` stdout                              | scripts/bench-tier1.mjs:795, scripts/battletest.mjs:1169                                                                                  |
| FND-016 | low      | L2 locality match is lenient (`endsWith` both directions, line-less findings match line-pinned labels), inflating L2 recall | scripts/lib/tier1-recall.mjs:311                                                                                                          |
| FND-017 | low      | Stale comment references the deleted `evals/fixtures/bench/labels.json`                                                     | evals/lib/quality.mjs:6                                                                                                                   |
| FND-018 | low      | Empty YAML frontmatter parses to `null` and throws a `TypeError` naming no file                                             | src/measurement/plans.ts:37, src/measurement/profiles.ts:40                                                                               |
| FND-019 | low      | Newest collection chosen lexicographically while ages use `Date.parse`; mixed TZ offsets can pick a wrong staleness anchor  | src/measurement/portfolio.ts:87                                                                                                           |
| FND-020 | low      | Corpus-gap count sourced from one collection but labeled with the newest collection's path                                  | src/measurement/portfolio.ts:154                                                                                                          |
| FND-021 | low      | `assurance` drops the computed `unevaluated` bucket, rendering not-inspected obligations as fully supported                 | src/commands/assurance.ts:91                                                                                                              |
| FND-022 | low      | `--suite`/`--commit` joined into store paths unsanitized, unlike the sibling measurement store's `safeId`                   | src/commands/evidence/inspect-mocks.ts:47                                                                                                 |
| FND-023 | low      | `--since` prefix matching the latest collection self-compares and reports all-zero deltas instead of a diagnostic           | src/measurement/report.ts:245                                                                                                             |
| FND-024 | low      | `HISTORICAL_MEASUREMENT_SCHEMA_VERSIONS` exported but unused; the read check hardcodes versions                             | src/measurement/types.ts:2                                                                                                                |
| FND-025 | low      | Source walks exclude `node_modules` etc. but not `.worktrees`/`.claude`, so leftover worktrees scan as a second repo copy   | src/evidence/mock-inspection.ts:8, src/validators/gates.ts:4                                                                              |
| FND-026 | low      | Legacy `bench/measurements.jsonl` readability test silently passes if the file is deleted                                   | tests/bench-tier1.test.ts:1                                                                                                               |
| FND-027 | low      | Dead assertion (`"must not"` subsumed by line 55) and reflow-brittle line-wrap regexes in the skill contract test           | tests/assurance-status-report-skill.test.ts:56                                                                                            |

## Method

- Three parallel reviewers over `src/` (production TS), `scripts/` +
  `evals/lib/` (instrumentation), and `tests/` + `vite.config.ts`; each finding
  above was confirmed against the code in context, and both highs were
  re-verified independently before recording.
- Gates re-run locally, not assumed: `make lint`; `make test
QUIRE=<governed build>` (731/731); `make audit-tool-drift` (33/33 drift
  mutations rejected, 23/23 stack invariants). The first ungoverned run failed
  exactly one test — TC-118's live-contract refusal of the ambient
  `~/.cargo/bin/quire` — which is the designed behavior, observed live.
- CI/build surface hand-reviewed: every workflow action, runner, Node, pnpm,
  Rust, and container image is SHA- or exact-version-pinned; dependency ranges
  collapsed to exact versions; `package.json` `files` whitelist confirmed to
  exclude the 69 MB of retained evidence, so the published package gains only
  the ~10 KB skill. Merge is a clean fast-forward (main has no commits the
  branch lacks).
- PR #295 reviewed directly: the skill and report contract are coherent,
  read-only-by-default, and delegate measurement semantics to `quoin
report`/`quire provenance`; its contract test genuinely pins the
  load-bearing phrases (FND-027 is cosmetic).

## Disposition notes

- FND-001/FND-005 are latent gate holes: current retained numbers were measured
  against real populations and are not invalidated. The hole fires on the next
  producer change that collapses a labeled population.
- FND-002/FND-004 do not prove the retained review was not performed; they
  establish that the tooling cannot record a non-pass ruling and that per-row
  rationales in the retained artifacts are template text, not row-level
  records. Regeneration is at least visible: both artifacts are
  content-addressed in `quality/verification-stack-lock.json`, so a re-mint
  requires a reviewable rebind commit.
- Everything from FND-003 down is follow-up-ticket material by the maintainer's
  stated preference; nothing below `high` touches the release path.
