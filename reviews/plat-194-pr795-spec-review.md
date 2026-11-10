---
id: SR-20261103118161
title: spec review of the PLAT-194 spec and matrix edits
type: SpecReview
analysis: base
scope: agent-ix/quoin; spec/functional/FR-077-generated-schema-emission.md; spec/matrix.md
review_set: subset
date: '2026-11-10'
---

# spec review of the PLAT-194 spec and matrix edits

## Summary

Ticket: PLAT-194 (agent-ix/quoin PR #795, head 6501ffdc).

This review covers the FR-077 Inputs, Behavior, AC-5 and AC-8 edits, the FR-077, FR-083 and NFR-020 summary rows, and the TC-1414..1416, 1431, 1448, 1449, 1457 and 1465 rows. The integrity and EARS sub-checks were applied inline to the edited statements.

## Scope

- `FR-077-AC-5` (examined): "An `@jsonSchema` base that disagrees with the package identity fails the emit command naming both values, and no committed file changes."
- `FR-077-AC-8` (examined): "In a mixed rendering with an imported module, a `$ref` to an imported model is written against that imported module's base; a `$ref` matching no declared base fails the emit command naming the reference."
- `FR-077#inputs` (examined): "The package identity (`<org>/<repo>`)"
- `matrix#FR-077` (examined): the summary row.
- `matrix#FR-083` (examined), `matrix#NFR-020` (examined): the summary rows.
- `matrix#TC-1431` (examined): left at 🚧, with the validation leg driven.
- `matrix#TC-1428`, `matrix#TC-1429`, `matrix#TC-1450`, `matrix#TC-1460` (context_only): left at 🚧, untouched.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | This PR rewrote the FR-077 summary row and kept "TC-1408, TC-1409, TC-1413 pass under `make template-gate`". The root Makefile has no `template-gate` target, and no Rust test is traced to FR-077-AC-1..3. The row also leaves out TC-1457, which this PR flipped. The row should name only what runs: TC-1414..1416, 1449 and 1457 on `make test`, with AC-1..3 unbacked. | spec/matrix.md:111 |

## Verdict

The FR-077 edits match the post-#793 driver. `moduleBase` checks the declared base against `https://schemas.agent-ix.org/<identity>/`, where the identity comes from the `package.json` name. The base carries no version. `importedBases` keys each imported base by identity alone. "Manifest `version`" is therefore correctly replaced by package identity in Inputs, the Behavior bullet and AC-5. AC-8 no longer claims a versioned import base. No stale `version` wording is left in FR-077.

The TC-1415 and TC-1457 titles were restated to match. The 🚧 rows are honest:

- TC-1431 names the leg that is driven and the two legs that are not.
- TC-1428, TC-1429, TC-1450 and TC-1460 are unchanged and still need a Python environment.

FND-001 is a false statement in a row this PR authored. Fix it in this PR.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (head ac764d9e): the FR-077 row names TC-1414..1416, 1449 and 1457 on `make test` via tc_194_001..005, and marks AC-1..3 unbacked with no `make template-gate` target. |
| FND-002 | fixed | Round 2 (head 6b38ff24): `git diff origin/main...HEAD` no longer touches FR-080. AC-1, AC-2, AC-7, AC-8 and the two engine-floor SHALLs are back verbatim. TC-1428, 1429, 1450 and 1460 are restored, each reading "❌ Unbacked — owner dropped test; needs PyPI-installed Python deps". The FR-080 row lists them as unbacked. The TASK-048 and PLAN-008 citations resolve again. |
| FND-003 | fixed | Round 2 (head 6b38ff24): FR-080-AC-5 again reads "fails when any one fails", matching main and the Behavior SHALL. TC-1431 reads "🚧 Partly": the validation leg is backed by tc_194_008, and the lint, drift and suite legs are unbacked. tc_194_008 now traces `FR-080-AC-5` and `TC-1431`. |
| FND-004 | fixed | Round 2 (head 6b38ff24): TC-1409 and TC-1410 read "🚧 Partly". Each names `tc_264_021` for its backed half and marks the FR-077-AC-2/CON-2 or StR-008-VC-2 half unbacked. The FR-078 row is now 🚧 Partly and cites only tests that exist with those traces (tc_264_020, 021 and 050, and the FR-078-AC-5/AC-8 traces at tc_264_template_render.rs:376 and :397). |
| FND-005 | fixed | Round 2 (head 6b38ff24): `grep -rn template-gate spec/` returns nothing. The US-021 and NFR-019 rows and all thirteen former `✅ make template-gate` TC rows now name a real Rust test or read ❌ Unbacked. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | The fix round deleted FR-080-AC-1, AC-2, AC-7 and AC-8, the two engine-floor SHALL statements, and TC-1428/1429/1450/1460, with no change record. The Description and three Behavior SHALLs it kept still require that the suite fails when the engine is absent (naming `poetry install`), when `extract_semantic` is missing, and when the grammar package is not installed. The rendered `conftest.py:46,53,110` implements all three, so three live obligations now have no acceptance criterion. The floor requirement was removed silently, not withdrawn. TASK-048:33-43 and PLAN-008 plan.md:112 still cite the deleted TC ids. "Needs a Python environment" is a reason the rows are unbacked. It is not a reason to delete the requirement. | spec/functional/FR-080-generated-verification-suite.md:44-69 |
| FND-003 | medium | FR-080-AC-5 was narrowed from "fails when any one fails" to "a failing validation leg fails the gate before lint and the suite run", to fit tc_194_008. The Behavior SHALL it verifies still says the gate fails when any one of its four legs fails. The AC now checks one leg of four, and TC-1431 is flipped to ✅ against the narrower text. | spec/functional/FR-080-generated-verification-suite.md:68 |
| FND-004 | medium | The PR sets TC-1409 to "❌ Unbacked — no Rust test is tagged to this row". That is false: `tc_264_021` carries `#[trace("FR-078-AC-3", "TC-1409", "TC-1410")]` (tc_264_template_render.rs:348). The FR-078 summary row still says ✅ Covered while citing TC-1409, which contradicts the PR's ❌. The accurate status is 🚧: the FR-078-AC-3 half is backed by tc_264_021, and the FR-077-AC-2/CON-2 emitted-schema half is unbacked. | spec/matrix.md:1121 |
| FND-005 | medium | This PR edited the US-021 row and kept "passes its own suite under `make template-gate` (31 rows, zero skipped, in all three variants)". Its own FR-077 and TC-1408 rows say that no `make template-gate` target exists. This is the same false-evidence defect as FND-001, in a row this PR authored. Thirteen TC rows (TC-1410, 1411, 1417, 1468, 1420..1426, 1458, 1469) still read `✅ make template-gate`, which the PR's own statement contradicts. | spec/matrix.md:1476 |
