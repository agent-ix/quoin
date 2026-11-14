---
id: SR-20261113202802
title: "Spec review of quoin PR 879 changes to FR-171 and FR-172"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@0581b950; spec/functional/FR-171-security-analysis-emits-typed-controls.md, spec/functional/FR-172-clause-set-sources-and-licence-boundary.md"
review_set: subset
---

# Spec review of quoin PR 879 changes to FR-171 and FR-172

## Summary

Ticket: PLAT-1530. Base spec review with EARS and integrity checks over the changed lines only:
two new FR-172 Behavior statements (lines 39-40), FR-172-AC-7 and AC-8, and the FR-171 statement
(line 42) and FR-171-AC-4 edits. Both files pass `quire validate`. Tools: quoin 0.30.1, quire
0.44.0 (engine 0.59.1).

Examined and clean: FR-172-AC-8 is atomic, observable and failable (the deleted directories and
links would fail it on the base tree). FR-172 line 40 is a well-formed event-driven statement.
FR-171 edit (question raised by the lead): "among those the skill keeps" does not weaken a live
requirement. The original clause protected unverified SOC 2 and ISO numbers from being "corrected"
without a free source; with the tables deleted no such number remains, so nothing it protected is
lost. The clause is now empty (FND-003).

## Verdict

Acceptable with one medium (FND-001) and three low findings. No requirement contradicts another;
FR-171 is not weakened.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-172-AC-7 cannot fail for the defect it targets. The sentinel is a value the test invents, so no shipped file can carry it unless someone pastes it there; the pre-PR tree with every restricted reference table present satisfies AC-7. Installing the pack in a temp `IX_HOME` is not causally connected to the files the criterion inspects. Restate it as an observable absence of restricted content (for example, no SOC 2 or ISO/IEC 27001 control identifier under `skills/spec-security-analysis` or `tests/fixtures/security-analysis`) or delete it in favour of AC-8 | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:54 |
| FND-002 | medium | The new statements at lines 39-40 do not say whether the skill may read the locally installed pack's licensed clause text to judge which clause applies, nor how a pack's set key is recognised as "the" set of SOC 2 or ISO/IEC 27001. With ids only and no text or title, the skill has nothing to decide applicability from, so ticket AC 1 ("maps controls to its clause ids") is underdetermined; two implementers would read it differently. State that pack text may be read locally for applicability but never written, and how a restricted source is matched to an installed set | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:39-40 |
| FND-003 | low | FR-171 line 42 and FR-171-AC-4 now constrain "SOC 2 and ISO/IEC 27001 numbers the skill keeps", an empty set after this PR, and line 41 / AC-4 ("no changed standard name ... in the references") were left as is although this PR removes SOC 2 and ISO rows from the references. Either these FR-171 statements are scoped to the PLAT-1426 change (then the edit was unnecessary) or they are standing (then the standard-name clause is now violated). Delete the SOC 2/ISO clause from the statement and AC-4 rather than qualify it, and scope the statements explicitly to the PLAT-1426 change | spec/functional/FR-171-security-analysis-emits-typed-controls.md:41-42 |
| FND-004 | low | Line 39 is compound: two obligations ("SHALL NOT ship ... and SHALL NOT read one") in one statement, and "read one" has no AC (AC-8 checks shipping and linking only). Split it; the read half is covered by the existing no-text rule or by AC-8's link check | spec/functional/FR-172-clause-set-sources-and-licence-boundary.md:39 |

## Disposition pass 1

Reviewed head `74150d5d97028bbbeae6a5dfe08dfc4fc4d28290`, including fix commits `eec8f59e` and `74150d5d`. Replacement reviewer; original findings and verdict above are preserved as historical records. Tools: quoin 0.30.1; quire 0.44.0 (engine 0.59.1). Original Linear comment: `63ca90dc-64f4-4cce-baf0-494f469cdcaa`.

## Assurance Context

Examined `spec/assurance/AP-201-finding-quality.md` and `spec/assurance/AP-202-jev-lens-evaluation.md`. Neither applies: this change does not alter measurement/scoring/report engine behavior or evaluate a Jev lens. Unavailable selected context: none. Active exceptions: none.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-7 now states observable restricted-identifier absence; the repaired scan can fail on the old shipped table. |
| FND-002 | fixed | Local reading for applicability and no artifact/review/reply copying are explicit. Pack identity requires the exact source-profile add key; the next behavior statement forbids name-similarity matching. |
| FND-003 | fixed | The obsolete restricted-number statement and AC clause are removed, and the remaining standard-name constraint explicitly belongs to the PLAT-1426 change. |
| FND-004 | fixed | Shipping and reading instructions are separate obligations; AC-8 checks shipped references and AC-9 checks links/path instructions, with tc_172_002/003 coverage. |

## Disposition scope

Original units below were re-examined against the fixed branch; excerpts retain the original review text for comparison. The dispositions comment includes the full original finding records and fixed excerpts.

```yaml
scope:
- id: FR-172-AC-7
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: No file shipped under `skills/spec-security-analysis`, and no fixture or expected output under `tests/fixtures/security-analysis`,
    contains the clause-text sentinel of a synthetic licensed pack `example.invalid/std@1` installed in a temp `IX_HOME`
    where `quoin module list` shows it as `pack=true`.
- id: FR-172-AC-8
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: The skill ships no `references/soc2` or `references/iso27001` directory, and `SKILL.md` links to none
    and instructs ids of a restricted source only from an installed pack's clause set.
- id: FR-172-B39
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: The skill SHALL NOT ship a reference table, summary or index of the controls of a licence-restricted
    source, and SHALL NOT read one.
- id: FR-172-B40
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: When a licensed pack is installed, the skill SHALL take the clause ids of its source only from the pack's
    clause set and write them by reference.
- id: FR-171-B42
  path: spec/functional/FR-171-security-analysis-emits-typed-controls.md
  role: examined
  excerpt: The change to the skill SHALL leave every SOC 2 and ISO/IEC 27001 number it keeps as it is, because no
    free source checks them; the licence-restricted reference tables are not kept ([FR-172](./FR-172-clause-set-sources-and-licence-boundary.md)).
- id: FR-171-AC-4
  path: spec/functional/FR-171-security-analysis-emits-typed-controls.md
  role: examined
  excerpt: A diff of the skill against its base shows no changed standard name, other than ASVS chapter ids and
    titles replaced by those of ASVS 5.0.0, and no changed SOC 2 or ISO/IEC 27001 number among those the skill keeps.
- id: FR-171-B41
  path: spec/functional/FR-171-security-analysis-emits-typed-controls.md
  role: context_only
  excerpt: The change to the skill SHALL leave every other standard name in `SKILL.md` and the references as it
    is.
- id: FR-172-AC-9
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: '| FR-172-AC-9 | No file of the skill links to, or names the path of, a licence-restricted source reference
    table, in any spelling of the link, and `SKILL.md` instructs ids of a restricted source only from an installed
    pack''s clause set. | Test |'
- id: FR-172-AC-10
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: '| FR-172-AC-10 | The synthetic pack fixture `example.invalid/std@1` under `tests/fixtures/security-analysis/packs`,
    installed in a temp `IX_HOME` with clause text added only inside that temp pack, is listed by `quoin module
    list` as a pack with no clause text in the output, and is not listed before it is installed. | Test |'
- id: FR-172-AC-11
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: '| FR-172-AC-11 | `SKILL.md` instructs that pack clause text may be read locally but never written, and
    that a pack is the source''s set only when its set key is the one the profile names. | Inspection |'
```

## Current verdict

All four original findings fixed. No new spec defect found in this disposition pass; the separate gap-analysis demonstration gap still prevents merge.

## Disposition verification

Independent focused command, from rust/: locked-build.sh /home/peter/dev/worktrees/plat-1530-quoin/target env CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test --locked -j 4 -p quoin-cli --test tc_171_security_analysis_skill --test tc_172_licensed_pack_boundary. Result: tc_171 10/10; tc_172 4/5; exit 101. tc_172_004 failed with QM024_PACK_INSIDE_WORK_TREE because the sandbox exposes /tmp/.git. This is new FND-006, not ignored as a passing run.

Rerun through the same locked wrapper/target/settings with TMPDIR=/var/tmp outside the sandbox: cargo test --locked -j 4 -p quoin-cli --test tc_172_licensed_pack_boundary. Result: 5/5 passed, exit 0. No source changes between runs.

Base oracle: extracted origin/main (60fcdd381ca3a1eb5f131052ed5fce7934c73941) skill/fixture paths into the scratchpad base directory. Through the locked wrapper/target/settings, QUOIN_LICENSE_SCAN_ROOT pointed there and only tc_172_001 ran. Expected exit 101: the old references/iso27001/A8-TechnologicalControls.md carried restricted identifiers. Current-head scan passed.

quire trace --scope . --id FR-172-AC-3 --format json and the AC-4 equivalent both exited 0 with verifies=[] and implements=[]; only file-scope mentions remain. No live skill demonstration was performed or claimed. Trace emitted unused-import and conditional-symbol collision diagnostics outside this diff.

Explicit FR-171 and FR-172 quire validation exited 0. Each of the three scratchpad SR artifacts validated at its explicit absolute path; --summary reported 1/1 docs grammar-clean and zero grammar findings. Installed tool versions: quoin 0.30.1; quire 0.44.0, engine 0.59.1. Validation emitted semantic.unused-import for spec-artifacts-app importing spec-artifacts-iso, with no validation failure.

No aggregate gate, live model run, source edit, push, or GitHub findings comment was performed. Formatting/Clippy are coder-reported checks and were not repeated in this disposition. The branch remains 74150d5d97028bbbeae6a5dfe08dfc4fc4d28290.

## Disposition pass 2

Reviewed head `42c9b80e4f036930722b893a9b2b724b87472318`. All four original spec findings remain fixed. No new spec finding. The separate code and gap findings still prevent merge.

Prior fixed findings were rechecked without duplicating their disposition rows. Previous review text and prior rows remain unchanged.

Rechecked frozen head 42c9b80e4f036930722b893a9b2b724b87472318 and fix diff against 74150d5d. No skill or spec source changed in this fix round; original code FND-001 through FND-004 and all four spec findings were rechecked and retain their previous fixed dispositions, with the replacement link-check regression separately recorded as code FND-007.

The licensed-pack test source contains no restricted-framework name literals. Its exact free-reference path allow-list checks the shipped references tree; tc_172_004 now calls the existing pack_support::scratch() helper.

Reviewed /tmp/codex-controls-1530-skillrun/evidence/RESULT.md, both recorded replies and content-scan.json. The record is from 74150d5d; the skill is unchanged at this head. It explicitly records failed positive pack installation and zero authored artifacts. The no-pack reply is observed; positive skill-output acceptance remains open.

Signature status: raw commit contains an SSH gpgsig header. ssh-keygen -Y check-novalidate -n git verified the extracted canonical commit payload/signature successfully (exit 0). git show --show-signature cannot check signer trust because gpg.ssh.allowedSignersFile is not configured; its displayed No signature does not mean the raw commit lacks a signature. Signer authorization was not verified. No unsigned-commit finding is justified by this configuration error.

No aggregate gate or live model run was performed in this disposition. Only the three review artifacts are updated in the repository, as explicitly requested by the dispatch. No commit, push or merge.

## Disposition pass 3

Reviewed head `14e6a21b421f72bf7208d7661595312eb61b50fc`. All four original spec findings remain fixed. No new spec finding. The separate gap finding still prevents merge.

Prior fixed findings were rechecked without duplicating their disposition rows. Previous review text and prior rows remain unchanged.

Reviewed frozen head 14e6a21b421f72bf7208d7661595312eb61b50fc and code-fix commit 76d944f97d267fd14ffb802942c7dbe64c259cbd. git diff confirms the latest commit adds only reviews/plat-1530-positive-skill-demo.md. The preceding fix changes the boundary test and review records; skill/spec source remains unchanged.

FND-007 is independently verified by reading resolve_local_target, local_link_targets, is_safe_local_link_path and safe_reference_paths_are_explicit_and_link_targets_resolve_within_them. Relative links are now resolved from each source document; the original unlisted-relative and misleading-prefix examples are explicitly rejected, as is ../../../outside/index.md.

Confirmed FND-005/006 remain fixed: free-source/asset allow-lists contain no named restricted-framework literals, and the pack-install test still imports and calls pack_support::scratch(). Other prior code/spec dispositions were rechecked; no new defect found in this pass.

Read the committed positive-demo record and actual /var/tmp/codex-controls-1530-live-demo inputs/outputs: pack-src/std.yaml, pack-install.txt, module-list.txt, profile-evaluation.txt, repo/spec/security/CTRL-001-synthetic-authorization.md, positive-mapping-reply.txt and quire-validate.txt. module-list reports pack=true, evaluation reports EX-1/EX-2 binding in example.invalid/std@1, and the only written control maps EX-1. The reply contains only that mapped set/clause. No reviews directory/output exists in the scratch repo.

The demo source pack has no clause-level text or sentinel. Its positive result establishes one synthetic exact-set mapping; it does not establish no-copy behavior when licensed clause text is actually available, does not prove a SpecReview output boundary, and is not generalized to other packs or agent runs. The recorded binary version is 0.31.0-46-g42c9b80e-dirty; this is reported as recorded, not relabeled as a clean build of the evidence commit.

Existing exact code-fix run is cited from the coder report supplied in this dispatch: tc_171 plus tc_172 15/15, focused Clippy/fmt pass, and expected base-oracle failure at 76d944f9. These are coder-reported results, not a new independent execution. No build/test was rerun, consistent with the dispatch allowing citation of the existing run. No aggregate gate was run.

Installed review tools: quoin 0.30.1, quire 0.44.0 (engine 0.59.1). FR-172 validation exits 0 with 1/1 docs grammar-clean, no grammar findings; semantic.unused-import for spec-artifacts-app is non-fatal. All three updated SR artifacts are validated at their explicit repository paths before handoff.

## Disposition pass 4

Reviewed head `68c8a1ac0e48bbf03d37c0a75366c0a26f0b5b1d`. All four original spec findings remain fixed. No new findings. No open reviewer finding remains; merge is clear from disposition, subject to the lead’s final gate and committing the updated review artifacts.

Prior findings and disposition rows remain unchanged.

Reviewed frozen head 68c8a1ac0e48bbf03d37c0a75366c0a26f0b5b1d. git diff confirms no rust/, skills/ or spec/ changes since code-fix head 76d944f97d267fd14ffb802942c7dbe64c259cbd. Later commits record demonstrations and review dispositions only.

Reconfirmed all seven code findings and all four spec findings remain fixed. The FND-007 test still covers not-in-inventory/index.md, ../../../outside/index.md and ../unapproved/references/gdpr/index.md; the resolver normalizes from the source document and checks the approved root/path. Named restricted-framework literals remain absent from the boundary test, and pack_support::scratch() remains in use.

Read reviews/plat-1530-sentinel-skill-demo.md and retained evidence under /var/tmp/codex-controls-1530-sentinel-demo/. Parsed pack-src/std.yaml and the installed target’s std.yaml and confirmed equality: both clauses have unique synthetic title/text sentinels. module-list.txt reports example-invalid-std as pack=true; source-profile.yaml adds the exact example.invalid/std@1 key; profile-evaluation.txt evaluates that set with EX-1 and EX-2 binding.

Independent literal checks found both clause sentinel strings absent in all seven requested outputs: repo/spec/security/CTRL-001-synthetic-authorization.md, repo/reviews/SR-001-gap-analysis.md, repo/demo-reply.md, profile-evaluation.txt, module-list.txt, write-pack.json and quire-validate-sentinel.txt. The control maps example.invalid/std@1 / EX-1 to FR-002; the review and reply identify the EX-2 gap by reference. No sentinel value is copied into this review record.

Independently ran IX_HOME=/var/tmp/codex-controls-1530-sentinel-demo/ix-home quire validate --scope /var/tmp/codex-controls-1530-sentinel-demo/repo "spec/security/*.md" "reviews/SR-001-gap-analysis.md" --summary. Exit 0; 2/2 docs grammar-clean, zero grammar findings. Advisory diagnostics: semantic.unused-import, semantic.source-identity-defaulted and semantic.clause-language-unchecked. This confirms the retained artifacts validate; it is not a fresh live skill run.

The earlier no-pack reply remains evidence for the unavailable-source scenario. The new sentinel-bearing positive example supplies the missing output-boundary demonstration. Conclusions are limited to these recorded synthetic scenarios, not arbitrary sources, paraphrase-detection completeness or statistical model reliability.

Existing exact code-fix run is cited from the coder report supplied by the lead: tc_171 plus tc_172 15/15, focused Clippy/fmt pass, expected base-oracle failure at the 76d944f9 source. No build/test or aggregate gate was repeated in this evidence-only disposition. Installed review tools: quoin 0.30.1; quire 0.44.0, engine 0.59.1.

Reviewer disposition is clear for merge after the lead’s final gate and commit of the updated SR artifacts. This pass changes only those three review files; it does not commit, push or merge.
