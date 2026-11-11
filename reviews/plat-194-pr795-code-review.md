---
id: SR-20261103118160
title: code review of the PLAT-194 template refusal tests
type: SpecReview
analysis: base
scope: agent-ix/quoin; rust/crates/quoin-semantic/tests/tc_194_template_refusals.rs; rust/crates/quoin-semantic/tests/common/template.rs; rust/crates/quoin-semantic/tests/common/mod.rs; rust/crates/quoin-semantic/tests/tc_264_template_render.rs
review_set: subset
date: '2026-11-10'
---

# code review of the PLAT-194 template refusal tests

## Summary

Ticket: PLAT-194 (agent-ix/quoin PR #795, head 6501ffdc). This review covers code-review and the rust-review lane, scoped to `git diff origin/main...HEAD`.

The run was done in a scratch worktree at 6501ffdc. `cargo test -p quoin-semantic --test tc_194_template_refusals --test tc_264_template_render` passed 8 and 35 tests. `cargo clippy -p quoin-semantic --all-targets --all-features -- -D warnings` was clean, and so was `cargo fmt --all --check`. The tools used were quoin 0.29.4, quire 0.39.0 (engine 0.55.0), cookiecutter 2.6.0, Node 22.15.0 and @agent-ix/semantic-module-tools 0.1.0 from the public registry.

## Scope

- `tc_194_001` (examined): check mode is green on clean output; one byte is flipped in Element.json; the run fails naming that file and only that file; no committed byte changes.
- `tc_194_002` (examined): the `@jsonSchema` base is mutated; the emit fails naming both the declared and the expected base; no committed byte changes.
- `tc_194_003` (examined): `main.tsp` imports semantic-core and declares no model whose name is in the installed grammar's generated schema set; the regex is checked to fire on an injected `model FieldDecl`.
- `tc_194_004` (examined): an unterminated model makes emit and check fail with `tsp compile failed`, the source name and the compiler diagnostic; no committed byte changes.
- `tc_194_005` (examined): an imported `$ref` is written under the imported base; an unknown `$ref` fails naming it, with `matches no declared base`; no committed byte changes.
- `tc_194_006` (examined): `@typespec/compiler` is removed; emit and check fail naming the toolchain and `make install`, and the success line is absent.
- `tc_194_007` (examined): `make validate` and `make gate` run with an empty PATH.
- `tc_194_008` (examined): an invalid `spec/spec.md` fails `make gate`.
- `common/template.rs` (examined): helper move from tc_264.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | tc_194_007's `message.contains("quire")` is met by make's own echo of the recipe line (`quire validate --scope . "spec/**/*.md"`), not by the failure. Reproduced: the output is the echo, then `/bin/sh: 1: quire: not found`. Assert the error line itself, for example `quire: not found`. | rust/crates/quoin-semantic/tests/tc_194_template_refusals.rs:405-409 |
| FND-002 | low | tc_194_008's "a later leg ran" check looks for `ruff`/`pytest`. When the lint leg runs, make echoes `poetry run poe lint`, so on a machine without a poe environment a lint leg that ran would never be detected. Assert the absence of `poe lint` and `poe test`. | rust/crates/quoin-semantic/tests/tc_194_template_refusals.rs:451-454 |
| FND-003 | low | The new `tests/common/template.rs` has no `SPDX-License-Identifier` / copyright header. It is the only `.rs` file under `rust/` without one. | rust/crates/quoin-semantic/tests/common/template.rs:1 |

## Verdict

The suite is real, with three nits. The seven flipped rows each come from a run that drives the path and asserts the value the refusal must name, not only a non-zero exit:

- The file name for TC-1414.
- Both bases for TC-1415.
- The redeclared set for TC-1416.
- The compiler diagnostic for TC-1449.
- The bad ref for TC-1457.
- The toolchain and the install command for TC-1448.
- The validator for TC-1465.

The driver's messages in `packages/semantic-module-tools/src/emit.mjs` match each assertion. Every mutation happens inside a `TempDir`, and the `Render` drop removes it on panic. `npm install` runs in the temp tree with the user npmrc disabled and a public registry. There is no pin, ledger or digest. The helper move to `tests/common/template.rs` is byte-faithful apart from `pub` and one sentence of doc. Clippy idioms are clean: the extension test uses `Path::extension`, not a string `ends_with`.

Mergeable as far as this method goes.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (head ac764d9e): tc_194_007 asserts `quire: not found` and `make install`; the template's `validate` recipe now emits that line from an `@`-silenced guard, so the recipe echo cannot satisfy it. |
| FND-002 | fixed | Round 1 (head ac764d9e): tc_194_008 asserts the absence of `poe lint` and `poe test`, which make echoes for `$(POETRY) run poe lint` / `poe test` whenever those legs start. |
| FND-003 | fixed | Round 1 (head ac764d9e): `tests/common/template.rs` carries the SPDX and copyright header. |
