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

## Disposition pass 1

Reviewed head `74150d5d97028bbbeae6a5dfe08dfc4fc4d28290`, including fix commits `eec8f59e` and `74150d5d`. Replacement reviewer; original findings and verdict above are preserved as historical records. Tools: quoin 0.30.1; quire 0.44.0 (engine 0.59.1). Original Linear comment: `2d37a2ce-5680-4de5-9870-9a9f2ebe59cb`.

## Assurance Context

Examined `spec/assurance/AP-201-finding-quality.md` and `spec/assurance/AP-202-jev-lens-evaluation.md`. Neither applies: this change does not alter measurement/scoring/report engine behavior or evaluate a Jev lens. Unavailable selected context: none. Active exceptions: none.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Partially addressed: a committed synthetic pack fixture and tc_172_004 now cover absent/present installation, pack classification and no sentinel in CLI install/list output. They do not run spec-security-analysis or inspect its artifacts, SpecReview or reply. FR-172-AC-3/4 remain Demonstration; quire trace reports no verifying claims, and no recorded TC-EV-061/062 skill run or named follow-up was supplied. Ticket ACs 1/2 and the skill-output portion of AC3 remain incomplete. Complete and record those demonstrations, or obtain an explicit scoped deferral to a named follow-up; this reviewer does not accept the gap on the team’s behalf. |

## Disposition scope

Original units below were re-examined against the fixed branch; excerpts retain the original review text for comparison. The dispositions comment includes the full original finding records and fixed excerpts.

```yaml
scope:
- id: FR-172-AC-3
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: Against a synthetic licensed pack `example.invalid/std@1` installed in a temp `IX_HOME`, a skill run
    yields mappings only for clause ids present in the pack, and a source with no installed set yields no mapping
    and a not-mapped statement. | Demonstration
- id: FR-172-AC-4
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: No artifact or reply of the FR-172-AC-3 skill run contains the pack's clause-text sentinel. | Demonstration
- id: FR-172-AC-5
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: '`SKILL.md` instructs the installed-set check, the not-mapped statement and the no-text rule. | Inspection'
- id: FR-172-AC-7
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: No file shipped under `skills/spec-security-analysis`, and no fixture or expected output under `tests/fixtures/security-analysis`,
    contains the clause-text sentinel of a synthetic licensed pack `example.invalid/std@1` installed in a temp `IX_HOME`
    where `quoin module list` shows it as `pack=true`. | Test
- id: FR-172-AC-8
  path: spec/functional/FR-172-clause-set-sources-and-licence-boundary.md
  role: examined
  excerpt: The skill ships no `references/soc2` or `references/iso27001` directory, and `SKILL.md` links to none
    and instructs ids of a restricted source only from an installed pack's clause set. | Test
- id: FR-171-AC-4
  path: spec/functional/FR-171-security-analysis-emits-typed-controls.md
  role: examined
  excerpt: A diff of the skill against its base shows no changed standard name, other than ASVS chapter ids and
    titles replaced by those of ASVS 5.0.0, and no changed SOC 2 or ISO/IEC 27001 number among those the skill keeps.
    | Inspection
- id: TC-EV-061
  path: spec/evals.md
  role: examined
  excerpt: Run spec-security-analysis over the fixture repo with a synthetic licensed pack `example.invalid/std@1`
    (clause ids only) installed in a temp `IX_HOME`. Mappings exist only for clause ids present in the pack; a source
    with no installed set (SOC 2) yields no mapping and the reply says "not mapped, set not installed".
- id: TC-EV-062
  path: spec/evals.md
  role: examined
  excerpt: Run TC-EV-061 with a clause-text sentinel placed in the synthetic pack's source material. The sentinel
    appears in no written artifact and not in the reply.
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

FND-001 remains open. Ticket acceptance is incomplete and the PR is not mergeable under the reviewer disposition rule.

## Disposition verification

Independent focused command, from rust/: locked-build.sh /home/peter/dev/worktrees/plat-1530-quoin/target env CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test --locked -j 4 -p quoin-cli --test tc_171_security_analysis_skill --test tc_172_licensed_pack_boundary. Result: tc_171 10/10; tc_172 4/5; exit 101. tc_172_004 failed with QM024_PACK_INSIDE_WORK_TREE because the sandbox exposes /tmp/.git. This is new FND-006, not ignored as a passing run.

Rerun through the same locked wrapper/target/settings with TMPDIR=/var/tmp outside the sandbox: cargo test --locked -j 4 -p quoin-cli --test tc_172_licensed_pack_boundary. Result: 5/5 passed, exit 0. No source changes between runs.

Base oracle: extracted origin/main (60fcdd381ca3a1eb5f131052ed5fce7934c73941) skill/fixture paths into the scratchpad base directory. Through the locked wrapper/target/settings, QUOIN_LICENSE_SCAN_ROOT pointed there and only tc_172_001 ran. Expected exit 101: the old references/iso27001/A8-TechnologicalControls.md carried restricted identifiers. Current-head scan passed.

quire trace --scope . --id FR-172-AC-3 --format json and the AC-4 equivalent both exited 0 with verifies=[] and implements=[]; only file-scope mentions remain. No live skill demonstration was performed or claimed. Trace emitted unused-import and conditional-symbol collision diagnostics outside this diff.

Explicit FR-171 and FR-172 quire validation exited 0. Each of the three scratchpad SR artifacts validated at its explicit absolute path; --summary reported 1/1 docs grammar-clean and zero grammar findings. Installed tool versions: quoin 0.30.1; quire 0.44.0, engine 0.59.1. Validation emitted semantic.unused-import for spec-artifacts-app importing spec-artifacts-iso, with no validation failure.

No aggregate gate, live model run, source edit, push, or GitHub findings comment was performed. Formatting/Clippy are coder-reported checks and were not repeated in this disposition. The branch remains 74150d5d97028bbbeae6a5dfe08dfc4fc4d28290.
