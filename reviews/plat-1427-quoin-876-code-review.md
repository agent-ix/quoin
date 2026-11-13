---
id: SR-20261113181007
title: "Code review of quoin PR 876 (spec-security-analysis typed controls, FR-171, FR-172)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@5afac302; README.md, skills/spec-security-analysis/SKILL.md, skills/spec-security-analysis/assets/*, spec/evals.md, tests/fixtures/security-analysis/**, rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs"
review_set: subset
---

# Code review of quoin PR 876

## Summary

Ticket: PLAT-1427. Scoped to `git diff origin/main...HEAD` at 5afac302 (merge base 15131e3a).
The Rust lane (`rust-review`, with the repo `.claude/skills/rust-style`) is folded in; the only
Rust file is the integration test `tc_171_security_analysis_skill.rs`. Companion module read in
place from agent-ix/spec-objects-security PR 53 at bc6eba15. Tools: quoin 0.30.1, quire 0.44.0
(engine 0.59.1).

Measured: all 345 bare ASVS requirement ids in `references/owasp-asvs/*.md` table rows resolve
in `owasp/asvs@5.0.0` of the companion module; `quoin write [REPO_DIR] --types` exists as the
skill instructs. Test evidence (locked-build.sh, jobs 4, debug 0): at 5afac302 with spec-objects-security PR 53 (bc6eba15) and fresh spec-objects-safety origin/main (bf44b49), `tc_171_security_analysis_skill` passes 10 of 10 and the six risk-advise/discrepancies/summary binaries pass 76 of 76; the same six pass 76 of 76 on quoin origin/main 15131e3a. The coder's 75 failures come from the local spec-objects-safety checkout (d11cbe8, a branch that predates PLAT-1565), not from this PR: against that checkout the same six binaries fail 75 of 76 (reproduced).

## Verdict

**MERGEABLE AFTER FIXES (one medium, four low).** The skill rewrite matches FR-171 and FR-172:
no Section 19, typed `control` and `scope_exclusion` output via `quoin write`, Set/Clause rows,
`not_assessed` default, installed-set check, not-mapped statement, no-text rule, no exclusion
without a responsible party, `quire validate` run. The Mandatory Standards table is unchanged,
so FR-171-AC-4 holds. The test reads the companion modules in place and copies nothing. The
medium defect is that the FR-171-AC-6 check cannot see 345 of the 350 ASVS ids it claims to check.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `is_asvs_id` only accepts a `V` prefix, but every ASVS requirement table under `references/owasp-asvs/` cites ids bare (`6.1.1`), so 345 of the 350 cited ASVS ids are never checked; a wrong row number there passes the FR-171-AC-6 test. All 345 resolve today (measured), so this is a test that cannot fail, not a wrong citation. Accept the bare form in the ASVS reference files (prefix `V` before lookup) | rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs:223-229 |
| FND-002 | low | The fixture SpecReview summary says the gateway answers "eight" clauses through three controls; the three controls map nine distinct clauses, and the test itself asserts `mapped == 9` | tests/fixtures/security-analysis/expected/reviews/26-11-13-security-analysis-example-session-gateway.md:14 |
| FND-003 | low | `installed_clause_sets` parses `manifest.yaml` by line prefix (`clause_sets:` then `- `), though the workspace has `quoin-yaml`; an equally valid indented list (`  - clauses/x.json`) reads as "declares no clause sets". Requirement and component ids are taken by fixed slices of file names (`[..6]`, `[..8]`), which mis-read a longer id. Parse the manifest with the YAML reader and take ids from frontmatter | rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs:130-139 |
| FND-004 | low | `module()` takes a `subdir` argument its only caller passes as `""`, with the module subdirectory folded into `sibling`; `SPEC_OBJECTS_SECURITY_DIR` names the module directory while `SPEC_ARTIFACTS_PROCESS_DIR` and the existing `SPEC_OBJECTS_SAFETY_DIR` name the repository root. Two meanings for the same kind of variable is a setup trap; make all three name the repository root and drop the dead parameter | rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs:54-88 |
| FND-005 | low | Every control template, example and expected fixture names its invariant for a behaviour (`EveryRequestIsAuthorized`, `FailSafeOnMissingGrant`, `NoRoleNoForward`, ...) while the OCL body only says `self.effectiveness->notEmpty()`. The skill copies these, so it is taught to write invariants whose name and body disagree. The companion skeleton has the same pattern. Name the invariant for what the body states, or state the behaviour in the body | skills/spec-security-analysis/assets/control-template.md:41-47 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, head c98fb578. `asvs_ids_cited` also reads the first-column dotted triples of the `references/owasp-asvs/V*.md` chapter tables and checks them as `V<id>`, so the 345 bare ids are now checked |
| FND-002 | fixed | Round 1, head c98fb578. The fixture summary now says nine clauses |
| FND-003 | fixed | Round 1, head c98fb578. The manifest is read with `quoin_yaml` (`manifest["clause_sets"]`); requirement and component ids come from frontmatter `id` via `artifact_id` |
| FND-004 | accepted-no-change | The dead `subdir` parameter is removed. The variable meanings are kept per module to match the existing tests that read the same modules (`tc_1515_soup_promote` reads `SPEC_OBJECTS_SECURITY_DIR` as the module directory; `tc_136_anomaly_command` reads `SPEC_ARTIFACTS_PROCESS_DIR` as the repository root, verified). Unifying them means changing those tests too, which is outside this PR; the module doc now states each meaning |
| FND-005 | fixed | Round 1, head c98fb578. The template, example and the three expected controls name the invariant `EffectivenessIsStated`, which is what `self.effectiveness->notEmpty()` states |
