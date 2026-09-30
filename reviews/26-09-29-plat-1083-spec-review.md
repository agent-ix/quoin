---
id: SR-178
title: "Spec review — PLAT-1083 CR notes (FR-082, FR-083, FR-076, FR-028, NFR-005)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@1aceb1552747024441f1ff7ddc1cd024d5a2a21f; spec/functional/FR-028-generate-property-tests-from-criteria.md, spec/functional/FR-076-semantic-module-template-variants.md, spec/functional/FR-082-generated-governance-tree.md, spec/functional/FR-083-template-render-self-tests.md, spec/non-functional/NFR-005-catalog-driven-workflows.md, spec/matrix.md (FR-082, FR-083, NFR-005 rows; TC-271, TC-1440..1442, TC-1451, TC-1470), spec/log.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-082"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-076"
    type: "reviews"
---

# SR-178: Spec review — PLAT-1083 CR notes

## Summary

Ticket: PLAT-1083. PR agent-ix/quoin#653 at `1aceb15`. The CR notes follow the repo's blockquote convention
(`> **CR-NNN (date, ticket):**`), and each is numbered per document. FR-028 correctly takes CR-002 after its
existing CR-001. The restated ACs are falsifiable and were checked against the template tree:

- FR-082-AC-2: no rendered `matrix.md` or `tests.md`, and no index link.
- FR-082-AC-3: no `TC-` in any rendered Verification cell.
- FR-082-AC-4: every rendered test is tagged with an existing criterion id.
- NFR-005-AC-1: no skill teaches a Status vocabulary. Checked by grep.

`quire validate` (0.34.0 release build) over the five changed requirement files and `spec/log.md` exits 0.
`spec/matrix.md` reports the same 21 structural assert failures as `origin/main` (lines 200, 693, 982-995,
1432, 1475), none of them on a row this PR touched. The row changes are right. TC-1440..1442 and TC-1451 are
restated at `🚧` with a reason, and TC-271 and TC-1470 are `⛔`. NFR-005 moves from the invalid `⚠️ Partial` to
`🚧 Partly`, and FR-082 moves from ✅ to 🚧 Partly. FR-083-AC-7 and FR-082-CON-1 have no dangling
references left.

## Verdict

**CONDITIONAL**. One medium finding: a stale behaviour line contradicts the restated FR-076-AC-10 in the same
document.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-076 still states "The template SHALL report the same declared-not-emitted target set in all three places." The PR reduced the preceding behaviour line, AC-10 and CR-001 to two records (the README and `semantic.targets`), and CR-001 says explicitly that "AC-10 now compares two records rather than three". The requirement now contradicts itself, and the "three" has no referent. | spec/functional/FR-076-semantic-module-template-variants.md:61 |
| FND-002 | low | The restated FR-076-AC-10 keeps `Test (TC-1451)` in its Verification cell. The same PR teaches method-only cells (specify, spec-object-review, spec-review) and writes FR-082's restated ACs as a bare `Test`. The edited row should follow the convention the PR introduces. Untouched rows are out of scope. | spec/functional/FR-076-semantic-module-template-variants.md:96 |

## Dispositions

Round 1, reviewed at `86f7c36cf8b592c59c106074f42f82b283f1eaf5`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 86f7c36: FR-076 line 61 now reads "in the rendered README and in `semantic.targets`", which agrees with AC-10 and CR-001. |
| FND-002 | fixed | 86f7c36: the FR-076-AC-10 Verification cell is now a bare `Test`. |

Round 2, reviewed at `a50fd540ad9285ef00bff1513076f98827ab8c8e`: no finding in this file was open, so no row was added. There is no regression in this file's scope. `quire validate` 0.34.0 on FR-082 exits 0. The only `spec/evals.md` error, at line 79, is identical on `origin/main`.
