---
id: SR-115
title: "Gap analysis — producer-facing change assurance CLI surface"
type: SpecReview
analysis: gap-analysis
scope: "FR-068; spec/matrix.md TC-1317..TC-1327; src/commands/change-assurance/; tests/change-assurance-command.test.ts; PLAN-005-change-assurance-contracts"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/issues/322"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-068"
    type: "references"
---

# SR-115: Gap analysis — producer-facing change assurance CLI surface

## Summary

Verification gate over FR-068 after #322. Every FR-068 acceptance criterion has
a matrix row, every row resolves to a tagged test in
`tests/change-assurance-command.test.ts`, and no code added by this change
lacks an owning requirement. The repository's pre-existing traceability debt is
recorded separately so it is not read as introduced here.

## Verdict

**CONDITIONAL** — no FR-068 gap; two medium findings on repository-wide
traceability debt and on #322 having no plan bundle.

## Findings

| ID      | Severity | Summary                                                                  | Refs                          |
| ------- | -------- | ------------------------------------------------------------------------ | ----------------------------- |
| FND-001 | medium   | 107 unbacked matrix rows, 3 status lies, and 20 untracked symbols repo-wide | spec/matrix.md:739            |
| FND-002 | medium   | #322 landed with no plan bundle, so the Task-level record is the issue alone | plan/PLAN-005-change-assurance-contracts/plan.md:5 |

## Finding detail

### FND-001 — repository-wide traceability debt

`quire coverage --scope . --json` reports 725 of 1207 matrix rows backed, with
107 unbacked rows, 3 status lies (`TC-1096`, `TC-1097`, `TC-1124`, all marked ✅
while binding nothing), and 20 untracked evidence symbols.

Failure scenario: a reader takes a ✅ row as evidence that a criterion is
discharged when the engine binds no symbol for it.

None of it belongs to FR-068: no FR-068 row is unbacked, no status lie names an
FR-068 or TC-13xx row, and `tests/change-assurance-command.test.ts` contributes
no untracked symbol and no unmatched tag. Recorded so the debt is attributed
where it arose. It needs its own ticket.

### FND-002 — no plan bundle for #322

`PLAN-005-change-assurance-contracts` is `status: done` and covers the FR-063 to
FR-065 contracts delivered by #282. #322 exposes those contracts and adds a new
requirement, FR-068, but has no Task in any bundle.

Failure scenario: a later reader reconstructing what was built for FR-068 finds
the requirement, the matrix rows, and the tests, but no plan record of the
decisions — for instance why `recover` is in the surface when the deliverable
list does not name it.

Not fixed here: this analysis is read-only over plans, and whether a
single-requirement CLI exposure warrants its own bundle is a planning decision
rather than a gap-analysis edit. The reasoning it would have carried is recorded
in SR-114's boundary notes instead.

## Coverage

Requirement-to-test, all backed:

| Criterion | Test case | Backing symbol |
| --- | --- | --- |
| FR-068-AC-1 | TC-1317 | seals and retains an explicit record and refuses a supplied digest |
| FR-068-AC-2 | TC-1318 | derives only the retained-output binding when sealing an attestation |
| FR-068-AC-3 | TC-1319 | retains exact bytes, is idempotent, and refuses contradictions |
| FR-068-AC-4 | TC-1320 | builds the verification input from named inputs only |
| FR-068-AC-5 | TC-1321 | never converts unavailable, not-computed, or missing evidence into a pass |
| FR-068-AC-6 | TC-1322 | exits 0 for valid, 1 for invalid and incomplete, and 2 for usage errors |
| FR-068-AC-7 | TC-1323 | re-verifies a sealed receipt and refuses an altered one |
| FR-068-AC-8 | TC-1324 | lists and emits the packaged assets and refuses an unknown name |
| FR-068-AC-9 | TC-1325 | recovers only interrupted staging and leaves retained pairs alone |
| FR-068-AC-10 | TC-1326 | reproduces the golden record, attestation, and receipt byte-identically |
| FR-068-AC-11 | TC-1327 | executes nothing and claims no identity, authorization, or certification |

Constraint coverage: CON-1 and CON-3 by TC-1327 (static, over the command
sources); CON-2 by TC-1318, which asserts field-by-field that only the three
retained-output fields differ from the caller's body; CON-4 by TC-1326 and by
the unchanged `src/change-assurance/` module, which this change does not touch.

Engine reconciliation — `quire coverage --scope . --json`:

| Measure | FR-068 | Repository |
| --- | --- | --- |
| Unbacked rows | 0 | 107 (FND-001) |
| Status lies | 0 | 3 (FND-001) |
| Untracked symbols | 0 | 20 (FND-001) |
| Unmatched tracking tags | 0 | — |

Underspecified code — none. The four helpers added outside command classes
(`readInputBytes`, `readInputJson`, `refuseSuppliedFields`, `parseSelection`)
are reached only from the seven commands, each of which traces to an FR-068
criterion. No production module outside `src/commands/change-assurance/` was
modified; `vite.config.ts` and `.prettierignore` are build and formatter
configuration, and TC-149 already gates the former.

Semantic review — performed inline over FR-068's eleven criteria rather than
fanned out, because the surface is one requirement over an unchanged library.
The tests exercise the real commands rather than doubles: TC-1317 through
TC-1326 drive the actual command classes against a temporary store on disk, and
the whole chain was additionally driven end to end through the built
`bin/quoin.js`, which is the only path that proves oclif can discover the
commands at runtime. TC-1327 is source inspection, which is the right shape for
a claim about what the code does *not* do — there is no runtime path that
demonstrates the absence of a subprocess.
