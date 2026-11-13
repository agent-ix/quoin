---
id: SR-20261113181008
title: "Gap analysis of quoin PR 876 against FR-171, FR-172 and NFR-038"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@5afac302; spec/functional/FR-171-security-analysis-emits-typed-controls.md, spec/functional/FR-172-clause-set-sources-and-licence-boundary.md, spec/non-functional/NFR-038-mapping-facts-state-and-never-decide.md, skills/spec-security-analysis/**, tests/fixtures/security-analysis/**, rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs, spec/evals.md"
review_set: subset
---

# Gap analysis of quoin PR 876

## Summary

Ticket: PLAT-1427. Planless. Plan completion: not assessed. Each acceptance criterion of FR-171,
FR-172 and NFR-038 was read against the `#[trace]` tags in `tc_171_security_analysis_skill.rs`,
the skill text and the committed fixture. Tools: quoin 0.30.1, quire 0.44.0 (engine 0.59.1).
Companion: agent-ix/spec-objects-security PR 53 at bc6eba15.

Clean: FR-171-AC-1, AC-2, AC-3, AC-4, AC-5, AC-7; FR-171-AC-6 (data resolves; see SR-20261113181007
FND-001 for the test's blind spot); FR-172-AC-5; FR-172-AC-6 for `SKILL.md`. FR-172-AC-1 and AC-2
belong to PLAT-1521/PLAT-1522 and the companion module and were read as context only. NFR-038
constrains `facts-from-mappings` (FR-173, not in this PR); its one hook into the skill, never
assigning an effectiveness, is met by the `not_assessed` default and the "never write `effective`
yourself" rule.

## Verdict

**NOT MERGEABLE AS IS (two high, one medium, one low).** Two Demonstration criteria of FR-172 are bound in
the Test Matrix by tests that exercise neither a skill run nor a licensed pack, so the matrix
reports them covered when nothing demonstrates them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-172-AC-3 (synthetic pack `example.invalid/std@1` in a temp `IX_HOME`, a skill run maps only its clause ids and states not-mapped for an absent set) is tagged on `a_mapping_to_a_clause_no_installed_set_holds_does_not_resolve`, which only calls the test's own `unresolved` helper on three literal rows, and on `expected_fixture_output_validates_and_every_mapping_resolves`, whose fixture has no pack. No product code, skill run or pack is involved; the test can only fail if the helper changes. Drop the AC-3 tags and add an eval scenario for the pack case, or a demonstration that installs the pack and runs the skill | rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs:414-423 |
| FND-002 | high | FR-172-AC-4 (no artifact or reply of the AC-3 run contains the pack's clause-text sentinel) is tagged on `no_expected_artifact_carries_clause_text`, which asserts that Clause cells of the hand-written fixture contain no space. There is no pack, no sentinel and no reply; `mapping_rows` already forces two cells. Drop the tag and cover AC-4 in the same pack demonstration as FND-001 | rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs:439-451 |
| FND-003 | medium | The skill is told never to supply the title or summary of a SOC 2 or ISO/IEC 27001 clause, yet step 2 still sends it to `references/iso27001/*.md` and `references/soc2/*.md`, which carry per-control titles and paraphrased descriptions for those licence-restricted sources. A run is primed to repeat them. The files predate this PR, but the new rule makes the conflict live. Needs an owner decision under the licensing boundary: reduce those references to ids, or state why their titles are permitted | skills/spec-security-analysis/SKILL.md:34-35 |
| FND-004 | low | FR-171-AC-5 (`SKILL.md` instructs the gap and no-responsible-party reply) is tagged on `the_review_records_each_gap_as_a_finding`, which reads the hand-written fixture SpecReview, not `SKILL.md`; the `SKILL.md` half is already covered by `skill_text_instructs_the_defaults_the_reply_and_the_set_check`. Drop the tag | rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs:425-437 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, head c98fb578. The FR-172-AC-3 tags are removed from both tests, and TC-EV-061 in `spec/evals.md` is defined as demonstration evidence. The criterion's Verification column is Demonstration, so an eval scenario is the right form and the Test Matrix no longer claims backing it lacks. Residual, not a defect of this PR: no eval harness runs `spec/evals.md` in this repo, so AC-3 stays undemonstrated until TC-EV-061 is run, as with every other TC-EV scenario |
| FND-002 | fixed | Round 1, head c98fb578. The FR-172-AC-4 tag is removed, the test is renamed `no_expected_mapping_row_carries_clause_text`, and TC-EV-062 defines the sentinel demonstration. The residual is the same as FND-001: defined, not yet run |
| FND-003 | fixed | Round 1, head c98fb578. SKILL.md step 2 now says the `soc2/` and `iso27001/` references serve id lookup only and that no name, title, description or paraphrase of those controls goes into any artifact or reply. That agrees with the Rules section, so the skill text is consistent. The reference files themselves (control titles and short descriptions, older than this PR) remain an open licensing question for Peter, not a defect of this PR |
| FND-004 | fixed | Round 1, head c98fb578. `the_review_records_each_gap_as_a_finding` no longer carries the FR-171-AC-5 tag; the SKILL.md half stays tagged on `skill_text_instructs_the_defaults_the_reply_and_the_set_check` |
