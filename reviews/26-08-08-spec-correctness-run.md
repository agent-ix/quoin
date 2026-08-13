---
id: SR-004
title: "spec-correctness — quoin — 130 criteria"
type: SpecReview
analysis: spec-correctness
scope: "spec/**/*.md, tests/props/"
review_set: all
---

# SR-004: spec-correctness — quoin — 130 criteria

## Summary

The `spec-correctness` run of 2026-08-08 over quoin's own spec classified 130
binding acceptance criteria, emitted 17 property tests unattended, reclassified
11 more through the review-gated second pass, and resolved 81 to existing
hand-written tests that were missing only a tracking tag. This artifact is the
record of that run, migrated from the ad-hoc `tests/props/QUEUE.md` the run
originally wrote (agent-ix/quoin#63); the findings below are the criteria the
run could not settle deterministically, and why.

## Findings

| ID      | Severity | Summary                                                                                                                                    | Refs                                                |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------- |
| FND-001 | medium   | Reclassified by the LLM second pass, not deterministically — read the generated test before trusting it                                    | FR-007-AC-2, FR-013-AC-2, FR-013-AC-3               |
| FND-002 | medium   | Reclassified by the LLM second pass; grounded on `src/plugins.ts:34` `parseSourceArg`                                                      | FR-018-AC-1, FR-018-AC-2, FR-018-AC-3, FR-018-AC-4  |
| FND-003 | medium   | Reclassified by the LLM second pass; grounded on `src/org.ts:190` `originOrg` and `src/config-schema.ts:15`                                | FR-025-AC-2, FR-025-AC-3, FR-025-AC-11, FR-027-AC-6 |
| FND-004 | low      | `universal` over-counts: 31 of 52 quantify over a single-element domain and are witnesses, not properties — all resolved to existing tests | FR-025-AC-9, FR-025-AC-10                           |
| FND-005 | low      | Verified by a method that produces no test to tag; not a gap and not a reason to reword                                                    | FR-003-AC-1, FR-003-AC-3, NFR-007-AC-1              |
| FND-006 | low      | Harness defaults deviated from, with reason: `numRuns: 30` (real oclif dispatch per case) and `numRuns: 40` (a directory created per case) | FR-005-AC-1, FR-015-AC-2                            |

## Census

```
extraction    extractable 60 · candidate 0 · not-extractable 70
property      example 62 · universal 52 · error-case 9 · unclassified 5 · ordering 2
archetype     FR 123 · StR 6 · NFR 1
spans present domain 21 · precondition 8 · oracle 21

emitted              17   extractable, grounded — tests/props/
emitted + finding    11   second pass, reclassified — tests/props/second-pass.prop.test.ts
already covered      81   existing tests, now carrying row_id tracking tags
verified elsewhere   21   eval, inspection, or delegated — no code tag by design
```

`17 + 11 + 81 + 21 = 130` — every record carrying a `row_id`.

Counts only. No threshold, no verdict, no rewording suggestion: the classification
carries no severity and no promotion path by design (quire-rs FR-052-CON-1). The
severities above describe what a reader has to do next, not the quality of a
criterion.

## Notes

- **No existing test carried a `row_id` tag when this started.** `spec/matrix.md`
  mapped requirements to tests as `` `file.test.ts` :: "test name" `` prose, so
  nothing was reconcilable by tool even where a hand-written test existed. 115
  `Trace:` comments now span 15 test files.
- **Two grounding bugs the emit caught**, both defects in the generated tests
  rather than in `src/` — which is what a failing generated test is supposed to
  mean. Four catalog properties built the wrong domain (fixtures wrote
  `artifacts:`/`objects:` where `loadCatalog` reads `artifact_types:`/`object_types:`),
  and `FR-005-AC-1`'s second assertion used a `\b`-anchored regex that fast-check
  shrank to `"a-"` — `\b` does not match after `-`.
- **11 matrix-cited test names looked absent on a literal search and are not.**
  All 11 are `test.each` / `it.each` parameterized cases whose source holds the
  template and whose matrix entry holds the expanded name. The matrix does not
  over-claim.
- **A count in the original record was withdrawn, not migrated.** It read "109 of
  130 criteria are reconcilable by tracking tag" and was measured with grep, which
  matches a tag wherever it sits — including the ~15 that sat above a `describe(`
  block and bound to nothing (agent-ix/quoin#61). Re-derive from
  `quire coverage --scope . --json`.
