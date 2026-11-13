---
id: SR-20261113202801
title: "Gap analysis of quoin PR 879 against PLAT-1530, FR-171 and FR-172"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@0581b950; spec/functional/FR-171-security-analysis-emits-typed-controls.md, spec/functional/FR-172-clause-set-sources-and-licence-boundary.md, spec/evals.md (TC-EV-061, TC-EV-062), skills/spec-security-analysis/**, tests/fixtures/security-analysis/**, rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs, rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs"
review_set: subset
---

# Gap analysis of quoin PR 879

## Summary

Ticket: PLAT-1530. Planless. Plan completion: not assessed. Bindings read with `quire trace --id`
(not `quoin matrix`, which misreads `#[trace]` tags, PLAT-1672). Tools: quoin 0.30.1, quire 0.44.0
(engine 0.59.1).

Bindings: `tc_172_001` verifies FR-172-AC-7 and `tc_172_002` verifies FR-172-AC-8; both tags match
their AC text as written (the weakness of AC-7 itself is in the spec review, SR-20261113202802,
and the test's vacuity in the code review, SR-20261113202800 FND-001).
`skill_text_instructs_the_defaults_the_reply_and_the_set_check` still verifies FR-172-AC-5 with the
updated needle. FR-172-AC-3 and AC-4 carry no claims (only a file-scope mention), which is correct
for Demonstration criteria.

Ticket ACs against the PR: AC 1 (map to pack clause ids by reference when a pack is installed) and
AC 2 (report the source unavailable instead of prose or embedded text when absent) are behaviours
of an agent run. The PR removes the embedded text the skill could fall back to and instructs both
behaviours in `SKILL.md`, which `tc_171` and `tc_172_002` check as text. AC 3 (output and
SpecReview carry ids only; sentinel test with a synthetic pack) is met only for committed files,
not for skill output.

## Verdict

Mergeable for its code; the ticket is not closable on this PR alone (FND-001, medium). The
Demonstration labelling is honest: no test claims a skill run, and the test module says so. That
makes it a coverage gap, not a false trace.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | PLAT-1530 ACs 1 and 2, and the skill-output half of AC 3, rest only on TC-EV-061 and TC-EV-062, which exist only as rows in `spec/evals.md`: no runnable eval suite, no synthetic pack fixture in the repository (the pack is built only inside the Rust test), and no recorded evidence of a run. The ticket's verification plan ("skill eval on a fixture repo with and without a synthetic licensed pack") was not executed. Either run the two scenarios and attach the evidence before closing the ticket, or keep those ACs open under a named follow-up | spec/evals.md:127-128 |
