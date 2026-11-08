---
id: SR-16319
title: "Gap analysis of quoin PR 772 against FR-038 (spec-fuzz harness and carriers)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@8291ec2; rust/crates/quoin-core/tests/tc_166_spec_fuzz_skill.rs; skills/spec-fuzz/**; spec/functional/FR-038-generate-fuzz-harnesses.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/FR-038
    type: reviews
---

# Gap analysis of quoin PR 772

## Summary

Ticket: PLAT-166, PLAT-167. Planless gap analysis of the change set. Plan completion: not assessed.
Tools: quoin 0.29.1, quire 0.36.2 (engine 0.50.2).

`quoin matrix --json` on the PR head: FR-038-AC-4 now has a static binder
(`tc_167_step_0_separates_installed_tooling_from_the_repo_workspace`), FR-038-AC-5 has two
(`tc_166_cargo_fuzz_template_*`), and AC-1..3 and AC-6..10 stay untagged. Every FR-038 row's
method is `agent-behaviour-eval`, except AC-10, which is `Test`. The TC-EV-054..057 scenarios that
the AC table cites do not exist in the repository (`evals/scenarios/index.mjs` is absent). That
gap predates this PR.

The skill change itself (template wrapper, step-0 harness rule) is implemented in every
skill file that states the rule. The only stub or partial is the leftover wording that SR-16318
FND-004 records.

## Verdict

**PASS WITH FINDINGS.** The AC-4 binder inflates coverage. It checks the wording of the skill,
not the behaviour the AC describes. The AC-5 binders cover one of the AC's three parts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | tc_167 is tagged FR-038-AC-4, so `quoin matrix` reports AC-4 as tagged. The test only greps step-0 prose. It never runs the skill on a repo that lacks a fuzz workspace, and never shows the result: one finding per obligation, no file, nothing installed | rust/crates/quoin-core/tests/tc_166_spec_fuzz_skill.rs:66-76 |
| FND-002 | low | The FR-038-AC-5 binders check where the `Trace:` carrier sits in the template only. Neither checks the obligation id in the target name or the provenance line, and both read the template, not an emitted target | rust/crates/quoin-core/tests/tc_166_spec_fuzz_skill.rs:25-64 |

## Examined scope

```yaml
clean: false
scope:
- id: FR-038-AC-4
  path: spec/functional/FR-038-generate-fuzz-harnesses.md
  role: examined
  excerpt: "An absent fuzz harness (no fuzz workspace in the repository, whether or not cargo-fuzz is installed) yields one finding per obligation and no file, and nothing is installed."
- id: FR-038-AC-5
  path: spec/functional/FR-038-generate-fuzz-harnesses.md
  role: examined
  excerpt: "Every emitted target carries both trace carriers and a provenance line."
bindings:
- {test_id: tc_167_step_0_separates_installed_tooling_from_the_repo_workspace, ac_id: FR-038-AC-4, trace: wrong}
- {test_id: tc_166_cargo_fuzz_template_puts_no_doc_comment_on_the_macro_invocation, ac_id: FR-038-AC-5, trace: correct}
- {test_id: tc_166_cargo_fuzz_template_carriers_sit_on_a_named_wrapper, ac_id: FR-038-AC-5, trace: correct}
```

## Dispositions

Round 1, verified against fix commit 4a76f316 (head d6503ab).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | tc_167 and its FR-038-AC-4 tag removed; AC-4 is no longer counted as tagged by a prose check |
| FND-002 | fixed | AC-5 binder now checks the id-in-name carrier and the provenance line; it reads the template the skill copies, and emitted targets remain the agent-eval method's job |
