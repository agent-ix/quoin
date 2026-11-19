---
id: SR-20261117175259
title: "Independent code-review of gap-analysis instruction repair"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1782; skills/gap-analysis/; skills/spec-correctness/; skills/spec-fuzz/; tests/scripts/check_gap_analysis.py; Makefile; plugin manifests"
review_set: subset
---

## Summary

Ticket: PLAT-1782. Independent review of frozen source `0b4af7283aa44e3215f8e32c4c2895740ff525e4` against `da3805f0a01777f83a6bda45a0d836552bf99d73` in the sole author tree on terra. This is a bounded changed-instruction and acceptance-gap check, not a whole-repository gap audit or release acceptance.

## Verdict

**CONDITIONAL** — one medium finding requires a fix before the requested instruction repair and regression coverage are complete.

## Assurance Context

AP-201 (spec/assurance/AP-201-finding-quality.md) was read as context: its benchmark scoring, measurement persistence and deterministic report-view scope is unchanged. AP-202 (spec/assurance/AP-202-jev-lens-evaluation.md) does not apply; no Jev lens is integrated or evaluated. No applicable assurance exception was found. Broad native runtime validation and release/install evidence are outside this bounded review; no selected assurance context is being claimed unavailable to justify a missing check.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new exclusive bash-fence rule contradicts the required untyped command fences for quoin write and quire validate. A literal executor can skip mandatory artifact authoring or validation. Remove the fence-language restriction or mark every executable example consistently. | skills/gap-analysis/SKILL.md:16-17 |

## Verification

- Verified clean tree and exact frozen source head before review.
- Independently inspected all 13 changed files, including version-only manifests, marker templates, matrix-status handling, diagnostic/fallback guidance, artifact authoring and fixture runner.
- `make skill-test`: six Python binding controls passed; quoin and quire matrix agreed; fixture pytest execution and SpecReview validation passed. These are installed-binary smoke checks, not proof that source instructions were executed.
- `claude plugin validate .`: passed.
- `git diff --check da3805f0a01777f83a6bda45a0d836552bf99d73..HEAD`: passed.
- Ruff check of the new Python runner passed.
- Installed quoin 0.36.8 and quire 0.52.0 (engine 0.62.2); process module v0.36.0. Current engine enum has qualified-record and retired plus the four previously documented statuses.
- Rust and React lanes do not apply to this changed scope. No Rust implementation changes or spec/plan requirement edits were made; broad native rebuild and spec-analysis lanes were not run.

## Examined Units

- `.claude-plugin/plugin.json` — role: examined; excerpt: "{\n  \"name\": \"quoin\",\n  \"version\": \"0.31.1\",\n  \"description\": \"Spec authoring, review, matrix, and planning skills for Agent IX.\",\n  \"author\": {\n    \"name\": \"Agent IX\"\n  }\n}"
- `.codex-plugin/plugin.json` — role: examined; excerpt: "{\n  \"name\": \"quoin\",\n  \"version\": \"0.31.1\",\n  \"description\": \"Spec authoring, review, matrix, and planning skills for Agent IX.\",\n  \"author\": {\n    \"name\": \"Agent IX\",\n    \"url\": \"https://github.com/agent-ix\"\n  },"
- `Makefile` — role: examined; excerpt: "# SPDX-License-Identifier: AGPL-3.0-or-later\n# Copyright (C) 2026 Agent-IX\n\n# Native Quoin build, test, and delivery entrypoints. The published executable\n# is built from the Rust workspace; pnpm supplies semantic schema inputs.\n# The published executable needs no source checkout or Node CLI shell.\n\nRUST_DIR := $(CURDIR)/rust"
- `skills/gap-analysis/SKILL.md` — role: examined; excerpt: "Invoke this agent skill as `quoin:gap-analysis` (or `/gap-analysis` in a host\nthat exposes slash commands). It is not a shell subcommand: do not run\n`quoin gap-analysis`. Read and execute the procedures below; only their fenced\n`bash` commands are shell invocations. A failed invented subcommand does not\nmake this skill unavailable or excuse skipping reverse-gap and stub checks."
- `skills/gap-analysis/references/step-1-target-selection.md` — role: examined; excerpt: "# Target Selection\n\n**Goal**: Resolve exactly what this run audits — one **repository**, its spec, its source\nand test trees — and the `ix://` identity used in the output artifact.\nRecord whether the caller explicitly supplied a plan.\n\nThe target of gap-analysis is a repository, not a plan. A plan is an optional extra input.\n"
- `skills/gap-analysis/references/step-3-matrix-verification.md` — role: examined; excerpt: "# Test-Matrix Verification\n\n**Goal**: Prove every acceptance criterion is backed by an actual test in the suite,\nidentified by a **trace tag** carrying the criterion's id. This is the heart of\ngap-analysis: a criterion is covered when a test says so in code, not when a document does.\n\n**The Test Matrix is computed, not read.** `quire matrix` derives it on every run from the\nspec's criteria and the trace tags in the source tree; `quoin matrix` adds the run evidence"
- `skills/gap-analysis/references/step-6-specreview-artifact.md` — role: examined; excerpt: "# SpecReview Artifact\n\n**Goal**: Emit the findings from the preceding steps as a single **quire-validated\n`SpecReview`** document with a Verdict, at `reviews/YY-MM-DD-<slug>.md`.\n\nThe artifact must make the mode unambiguous. A reader who sees only this document has to be\nable to tell whether plan completion was assessed or skipped, and must not be able to read a\nplanless PASS as evidence that planned work was completed."
- `skills/spec-correctness/assets/templates/fast-check.ts.md` — role: examined; excerpt: "# TypeScript / fast-check templates\n\nFile: `tests/props/fr-NNN.prop.test.ts`.\nRunner: whichever the repo already uses — read `vitest.config.*`, `jest.config.*`, or\n`package.json#scripts.test`. The imports below assume vitest, the only runner the `trace` helper\n(`@agent-ix/ix-trace-ts/vitest`) supports; a jest repo has no explicit TS trace marker.\n\n## Anatomy"
- `skills/spec-correctness/assets/templates/hypothesis.py.md` — role: examined; excerpt: "# Python / hypothesis templates\n\nFile: `tests/props/test_fr_NNN.py`.\nEvery emitted test runs — there is no inert form.\n\n## Anatomy\n\n```python"
- `skills/spec-correctness/assets/templates/proptest.rs.md` — role: examined; excerpt: "# Rust / proptest templates\n\nFile: `tests/props_fr_NNN.rs`.\nEvery emitted test runs — there is no inert form.\n\nPaths are **flat**: cargo auto-discovers integration tests only at `tests/*.rs`. A file\nunder `tests/props/` compiles as a helper module and never runs. See step 4.\n"
- `skills/spec-correctness/references/step-4-generate-tests.md` — role: examined; excerpt: "# Step 4: Generate Tests\n\n**Goal**: write the test, tagged so `gap-analysis` can reconcile it.\n\nTemplates per harness:\n[proptest](../assets/templates/proptest.rs.md) ·\n[fast-check](../assets/templates/fast-check.ts.md) ·\n[hypothesis](../assets/templates/hypothesis.py.md)"
- `skills/spec-fuzz/references/step-3-emit-targets.md` — role: examined; excerpt: "# Step 3: Emit the targets\n\n**Goal**: a target that compiles, runs, and reconciles.\n\nTemplates: [cargo-fuzz](../assets/templates/cargo-fuzz.rs.md) ·\n[atheris](../assets/templates/atheris.py.md) ·\n[fast-check](../assets/templates/fast-check.ts.md)\n"
- `tests/scripts/check_gap_analysis.py` — role: examined; excerpt: "    cases = {\n        \"healthy\": (\n            '@pytest.mark.trace(\"FR-001-AC-1\")',\n            \"assert double(3) == 6 and double(-2) == -4\",\n            \"tagged\",\n        ),\n        \"missing\": (\"\", \"assert double(3) == 6 and double(-2) == -4\", \"untagged\"),\n        \"name-only\": (\n            \"# FR-001-AC-1\",\n            \"assert double(3) == 6 and double(-2) == -4\",\n            \"untagged\",\n        ),\n        \"ignored\": (\n            '@pytest.mark.skip(reason=\"fixture\")\\n@pytest.mark.trace(\"FR-001-AC-1\")',\n            \"assert double(3) == 6 and double(-2) == -4\",\n            \"tagged-by-ignored-t"

## Coverage

Plan completion: not assessed
Semantic review: skipped; no whole-repository semantic review claimed.

## Dispositions

Round 1 independently reviewed `f9840129599c3e5dd091768d2f1eeb5988177d05` on the same frozen author tree.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Removed the exclusive bash-fence rule; command examples, including untyped artifact steps, now remain executable. |

## Disposition Verification

- `make skill-behavior-test` independently passed all six synthetic interpretation cases against the current source.
- Independent pre-repair control using `--skill-root /tmp/gap-instruction-before-repair` exited 1 at the unknown-status assertion: qualified-record and retired are unsupported under those instructions.
- The new target explicitly separates live instruction interpretation from the existing installed-binary smoke. The synthetic controls do not claim to execute a full repository audit.
- Ruff and `git diff --check` passed; author tree remained unchanged at the reviewed head.
- No new defect was found in the fix round. Original findings above are preserved verbatim.

## Current Disposition Verdict

**PASS** — FND-001 is fixed; no open findings remain for this method. Final exact-head gate, source merge and release remain the parent's actions.
