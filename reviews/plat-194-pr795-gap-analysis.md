---
id: SR-20261103118162
title: gap analysis of the PLAT-194 template refusal tests
type: SpecReview
analysis: base
scope: agent-ix/quoin; rust/crates/quoin-semantic/tests/tc_194_template_refusals.rs; spec/matrix.md; spec/functional/FR-083-template-render-self-tests.md; spec/non-functional/NFR-020-declared-toolchain-floors.md
review_set: subset
date: '2026-11-10'
---

# gap analysis of the PLAT-194 template refusal tests

## Summary

Ticket: PLAT-194 (agent-ix/quoin PR #795, head 6501ffdc). Plan completion: not assessed.

`quoin matrix` (0.29.4) reports "no coverage_matrix criteria: the module declares no `obligations:` source". As a result, the criterion-to-test check was done by hand. Each `#[trace]` tag in the new suite was compared with its AC text and with the driver code it exercises.

## Scope

- `FR-077-AC-4` (examined) "Check mode exits zero on the committed output and non-zero, naming the file, after one emitted byte is changed." This is bound to tc_194_001.
- `FR-077-CON-1` (examined): bound to tc_194_001.
- `FR-077-AC-5` (examined): bound to tc_194_002.
- `FR-077-AC-6` (examined): bound to tc_194_003.
- `FR-077-AC-7` (examined): bound to tc_194_004.
- `FR-077-AC-8` (examined): bound to tc_194_005.
- `NFR-020-AC-3` (examined) "With the schema toolchain absent, the emit and drift checks fail naming the toolchain and the install command, and report no skipped check." This is bound to tc_194_006.
- `FR-083-AC-6` (examined) "With the schema toolchain absent, the gate fails naming the install command and reports no skipped emission check." This is bound to tc_194_006.
- `NFR-020-AC-4` (examined) "With the validator absent, the rendered gate's validation leg fails naming the validator rather than reporting zero documents." This is bound to tc_194_007.
- `NFR-020` (context_only): the statement and the Verification section.
- `FR-080-AC-5` (examined): bound to tc_194_008, partial, with the row left at 🚧.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-083-AC-6 is about Quoin's gate: "If the schema toolchain is not installed, then Quoin's gate SHALL fail naming the install command". tc_194_006 removes `@typespec/compiler` from a rendered tree and asserts that the rendered emit command fails. That is NFR-020-AC-3. Quoin's own harness path is `installed()`. Its absent-npm panic names "install Node 20 or later", and its failed-install panic carries only npm's message. Neither path is driven, so TC-1448's ✅ overstates the backing for FR-083-AC-6. | rust/crates/quoin-semantic/tests/tc_194_template_refusals.rs:38-53 |
| FND-002 | medium | NFR-020's statement says to fail "naming the command and how to install it". Its Verification section asserts "the failure names the command and the install step". The rendered `make validate` leg yields only the shell's `quire: not found`, with no install step. TC-1465 is flipped to ✅ against AC-4's narrower wording. The template's validate recipe does not meet the NFR's install-step obligation for `quire`, and no test asserts that it does. | templates/semantic-module/{{cookiecutter.repo_name}}/Makefile:70-71 |

## Verdict

The bindings are sound for the other six flipped rows: TC-1414, TC-1415, TC-1416, TC-1449, TC-1457 and NFR-020-AC-3 via TC-1448. Each test drives the exact refusal in `packages/semantic-module-tools/src/emit.mjs` and asserts its named value.

The tests contain no stubs and no coverage inflation. tc_194_008 is tagged to FR-080-AC-5, and the matrix honestly keeps TC-1431 at 🚧 because the lint and test legs are not driven.

FND-001 and FND-002 are spec-to-behaviour gaps. They can be closed by a small test or recipe change, or by narrowing the rows' claims. Neither is a wrong result in the shipped code.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | Round 1 (head ac764d9e): the tc_194_006 trace and the TC-1448 row now name NFR-020-AC-3 only, and the FR-083 matrix row says FR-083-AC-6 is unbacked. But `spec/functional/FR-083-template-render-self-tests.md:65` still reads `Test (TC-1448)`, and TC-1448 no longer lists FR-083-AC-6. No test drives Quoin's own gate with the toolchain absent. Repointing the cell at tc_194_001..008 would reintroduce this finding, because none of them drives Quoin's gate. The fix is a test of the `installed()` absent-toolchain path, or a verification cell that does not claim a test. |
| FND-002 | fixed | Round 1 (head ac764d9e): the rendered `validate` recipe fails with `quire: not found; run 'make install' (poetry install provides the quire validator)`, and tc_194_007 asserts both `quire: not found` and `make install`. |
| FND-001 | fixed | Round 2 (head 6b38ff24): the FR-083-AC-6 Verification cell now reads "Inspection (unbacked: no test drives quoin's own gate with the toolchain absent; TC-1448 covers only the rendered emit command, NFR-020-AC-3)". The cell no longer claims a test. It agrees with the FR-083 matrix row and with TC-1448's NFR-020-AC-3-only trace. |
