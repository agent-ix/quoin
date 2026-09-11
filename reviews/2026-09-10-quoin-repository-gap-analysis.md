---
id: SR-150
title: "Gap analysis — quoin repository audit (planless)"
type: SpecReview
analysis: gap-analysis
scope: "spec/, src/, tests/, templates/, spec/matrix.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/TM-001", type: references }
---

## Summary

Planless repository audit of quoin at `365-planless-gap-analysis`, run with the branch's own
skill definition (the installed plugin cache is quoin 0.13.0, which still carries the
plan-first workflow). `quire coverage` reconciles 1013 of 1727 declared rows as backed,
leaving 207 unbacked rows, 9 status lies and 40 untracked symbols. Sampling the 9 status
lies splits them 6 rule / 2 real / 1 binding-form: six are caused by a quire tokenizer
defect that abandons any TypeScript file containing a regex literal with brace quantifiers,
so every tracking tag in that file binds nothing. **Every finding below is pre-existing
repository state.** None is introduced by PR #366, which adds no source module and whose one
new test file does not appear anywhere in the coverage report.

## Verdict

**FAIL** — 207 unbacked rows, 9 status lies, and a `high` finding in the reconciliation
engine itself. This verdict describes the quoin repository, not PR #366.

## Findings

| ID      | Severity | Summary                                                                                  | Refs                                                     |
| ------- | -------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| FND-001 | high     | quire's brace scanner counts regex-literal braces, abandoning three files and their tags | tests/spec-review-vocab-drift.test.ts:37                 |
| FND-002 | high     | TC-1096 and TC-1097 claim ✅ with no tag anywhere in `tests/` or `src/`                  | spec/matrix.md:864                                       |
| FND-003 | high     | 207 declared reference rows have no backing `verifies` relation                          | spec/matrix.md, spec/evals.md                            |
| FND-004 | medium   | TC-1124's id sits second in a two-id `it()` title and does not bind                      | tests/evidence-audit-command.test.ts:257                 |
| FND-005 | medium   | 40 tagged tests carry a trace id that resolves to no declared row                        | templates/semantic-module/.../test_skeletons_semantic.py |
| FND-006 | medium   | 91 declarations selected nothing; 30 are tags on non-binding symbols                     | spec/matrix.md                                           |

### FND-001 — the engine cannot read a file with a regex quantifier (high)

`quire coverage` emitted three parse diagnostics on stderr:

```
src/evidence/mock-inspection.ts: unbalanced braces: a `}` closes no block
src/semantic/sweep.ts: unbalanced braces: 1 block(s) left open
tests/spec-review-vocab-drift.test.ts: unbalanced braces: 1 block(s) left open
```

All three files compile under `tsc --noEmit` and pass eslint, so the braces are balanced.
The cause is the same in each: a **regex literal** the scanner reads as source.

- `tests/spec-review-vocab-drift.test.ts:37` — `/^import \{ specInvariants \} from [^;]+;/m`
- `src/evidence/mock-inspection.ts:121` — `[\s\S]{0,240}?`
- `src/semantic/sweep.ts:87` — `-{3,}`

Escaped braces and `{n,m}` quantifiers are counted as block delimiters, the file's depth
never returns to zero, and the binder abandons it. `tests/spec-review-vocab-drift.test.ts`
holds 9 tag-bearing lines, so **TC-279 through TC-284 report as status lies while their
tests exist, are tagged, and pass.**

This is the worst shape a reconciliation defect can take: it under-reports coverage silently
and manufactures false accusations against correct specs, in a direction that punishes
authors for writing a regex. It belongs upstream in quire-rs, not here. The three quoin files
are correct as written and must not be edited to appease the scanner.

### FND-002 — two rows are genuinely unbacked (high)

`TC-1096` and `TC-1097` (`spec/matrix.md:864`, `FR-043-AC-27`) are marked `✅` and neither id
appears in `tests/` or `src/`. Unlike FND-001 these are real: the matrix asserts a passing
test that does not exist.

### FND-003 — the unbacked-row backlog (high)

207 rows across `spec/matrix.md` (76), `spec/evals.md` (52) and eleven requirement documents.
`spec/evals.md` dominating is expected — eval rows mint through the eval harness, and 49 rows
are separately reported under `no_symbol_rows` as exempt by their declared verification
method. The remaining balance is a real backlog, not an artifact.

**This finding is a census, not a precision estimate.** The 9 status lies were sampled at
100%; the 207 unbacked rows were not, so the rule/corpus split above is established only for
the status lies.

## Coverage

- Reconciliation: `quire coverage` (quire 0.31.0, cli 4f6ed024, engine 0.46.0@ca7362d4),
  module `spec-artifacts-process`. Not the grep fallback.
- **Plan completion: not assessed**
- Rows backed by a tagged test: 1013 / 1727
- Evidence symbols: 1109 matched of 1396 examined
- Status lies: 9 — sampled 9 of 9: 6 rule (FND-001), 2 real (FND-002), 1 binding-form (FND-004)
- Untracked symbols: 40; rows exempt by declared method (`no_symbol_rows`): 49
- Diagnostics that selected nothing: 91 — 30 `tag-on-non-binding-symbol`,
  24 `untracked-id-has-minted-children`, 17 `uncatalogued-verification-method`,
  16 `section-matches-nothing`, 4 other
- Acceptance criteria: 803 bound, 388 property-shaped, 69 specific-shaped
- Engine suspicions: 4, all `oracle-resembles-implementation` inside `corpus/cases/skeptic/`,
  where a copied oracle is the fixture's purpose. Not findings.
- Semantic review: skipped — not opted into.
- **Reverse gap: branch surface only.** PR #366 adds no source module, so its reverse gap is
  empty by construction. A repository-wide behaviour inventory was **not** performed in this
  run, and no reverse-gap finding above rests on one. The verdict stands on the matrix
  evidence alone, which is sufficient for FAIL on its own.
- Suite execution: `make lint` clean; 1181/1181 tests pass (105 files) via the
  `test-with-quire` inner target. `make test` and `make validate` fail on pre-existing state
  (quire evidence-overlay relock; a duplicate heading in
  `spec/assurance/AP-201-finding-quality.md`), both reproducing on a clean tree.
