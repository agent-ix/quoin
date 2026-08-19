---
id: SR-006
title: "Gap analysis — ADR-0011 Phase 2 Waves C and D (FR-033, FR-034, FR-035)"
type: SpecReview
analysis: gap-analysis
scope: "spec/matrix.md; FR-033, FR-034, FR-035; tests/evidence-adapters.test.ts, tests/finding-record.test.ts, tests/combinatorial-coverage.test.ts; src/evidence/, src/auditor/, src/advisor/"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-034"
    type: "reviews"
---

## Summary

Verified Waves C and D against the Test Matrix and the source tree. **Every Wave C/D row is backed
by a real test carrying a tracking tag — zero unbacked, zero status lies in range.** Two gaps were
found inside the wave and fixed here; one large pre-existing gap is reported and deliberately not
fixed.

Both in-wave gaps were introduced by the previous turn's fixes and are exactly what this gate exists
to catch: work that passes its own tests while tracing to requirements that do not exist.

## Verdict

**FAIL** — 41 matrix rows repo-wide are marked ✅ with no backing tagged test. None is in Waves C or
D, and narrowing the scope to make this pass would be the same move the finding is about.

## Findings

| ID      | Severity | Summary                                                                                                                              | Refs                                                  |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------- |
| FND-001 | high     | 41 matrix rows read ✅ with no tagged test; the tests exist but use a `describe("TC-nnn …")` form the declared grammar does not bind | tests/auditor.test.ts:69                              |
| FND-002 | high     | Five tests traced to `FR-034-AC-16..20`, which were never declared in the FR                                                         | spec/functional/FR-034-finding-shaped-evidence.md:118 |
| FND-003 | medium   | The same five tests had no matrix rows, so five verified behaviours were invisible to the rollup                                     | spec/matrix.md:340                                    |
| FND-004 | low      | A commit message claimed `TC-180..184` for those tests — ids already held by FR-035                                                  | (commit ff7a164)                                      |

## Detail

### FND-001 — two tag conventions, one of which binds

quoin's tests use two forms:

| Form                   | Example                                     | Binds   |
| ---------------------- | ------------------------------------------- | ------- |
| `// Trace: TC-nnn`     | `evidence-adapters.test.ts` (23 uses)       | **yes** |
| `describe("TC-nnn …")` | `auditor.test.ts`, `evidence-store.test.ts` | **no**  |

Measured: `TC-137`, `TC-119` and `TC-129` all report as status lies while `TC-151`, `TC-165` and
`TC-180` bind cleanly. Eight test files carry **zero** `// Trace:` tags between them —
`auditor.test.ts` alone holds 27 real tests.

**The code is tested. The claim is unverifiable**, which is a different and quieter problem: quoin's
own matrix asserts ✅ over rows the engine cannot confirm, and quoin is the tool that reports this
class in other repositories.

Not fixed here. It is 41 rows across eight files, predates this program, and belongs in a ticket of
its own rather than inside a wave's gate — but it should not stay unrecorded, which is how it
survived this long.

### FND-002 / FND-003 — tests tracing to nothing

`SR-005`'s fixes added five tests carrying `// Trace: FR-034-AC-16` … `AC-20`. **Those criteria did
not exist.** FR-034 stopped at AC-15, so five tests referenced requirements no document stated, and
no matrix row named them.

This is the mirror of the defect `SR-005` found: there, a capability existed that nothing could
reach; here, tests existed that nothing declared. Both pass every gate — `make test` was green with
384 tests throughout — because neither is a property any gate inspects.

Fixed: AC-16..20 declared, `TC-192..196` added, and each test now carries both ids. All five verify
as backed.

### FND-004 — a claim in a commit message

The `SR-005` commit said the new tests were `TC-180..184`, which FR-035 already holds. The ids never
reached the test file or the matrix, so no artifact is wrong and history is not being rewritten —
recorded because the message is what a reader would consult first.

## Coverage

`quire coverage --scope .` over this repository:

| Measure                                    | Value     |
| ------------------------------------------ | --------- |
| Backed rows / total reference rows         | 150 / 450 |
| Unbacked rows                              | 184       |
| **Status lies**                            | **41**    |
| Untracked symbols                          | 6         |
| **Status lies in Waves C/D (TC-165..196)** | **0**     |
| **Unbacked rows in Waves C/D**             | **0**     |

No Wave C/D source is unspecified: every module traces to an FR — `src/evidence/adapters/` and the
registry to FR-033, `FindingRecord` with the store and auditor wiring to FR-034,
`src/auditor/combinatorial.ts` with the advisor's structural signal to FR-035.

Step 1 (plan completion) has nothing to assert against: this program is tracked in the GitHub epic
`agent-ix/quire-rs#81`, not a `plan/` bundle. The optional semantic review was **skipped**, as
requested.
