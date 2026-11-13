---
id: SR-20261113202800
title: "Code review of quoin PR 879 (licensed packs for restricted security sources, FR-172)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@0581b950; rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs, rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs, skills/spec-security-analysis/SKILL.md, skills/spec-security-analysis/references/standards-index.md, skills/spec-security-analysis/references/component-type-mapping.md, skills/spec-security-analysis/references/soc2/* (deleted), skills/spec-security-analysis/references/iso27001/* (deleted)"
review_set: subset
---

# Code review of quoin PR 879

## Summary

Ticket: PLAT-1530. Diff `origin/main...HEAD` (base 2975c419, head 0581b950). Code-review with the
rust-review lane folded in (one new Rust test binary, one changed needle in `tc_171`). Tools:
quoin 0.30.1, quire 0.44.0 (engine 0.59.1).

Run: `cargo test -p quoin-cli --test tc_172_licensed_pack_boundary --test tc_171_security_analysis_skill`
at head: 12 passed, 0 failed.

Examined and clean: the 15 deleted files under `references/soc2/` and `references/iso27001/`;
a repository-wide grep finds no dangling link to them outside historic review records
(`reviews/`, `spec/reviews/`), which stay as written. No plugin or marketplace manifest lists
them. No SOC 2 or ISO/IEC 27001 control identifier remains under `skills/spec-security-analysis`
or `tests/fixtures/security-analysis`; the remaining mentions are standard names and the
"not mapped, set not installed" statement. The NIST, ASVS and GDPR rows of
`component-type-mapping.md` and `standards-index.md` are unchanged; only restricted rows and
two restricted ids in "Secondary Focus" bullets were removed; no NIST, ASVS, GDPR or asset file is touched. The changed `tc_171` needle
matches the new `SKILL.md` wording.

## Verdict

Mergeable after FND-001 (medium). The skill-side change is correct and complete: the restricted
tables are gone and nothing links to them. The new sentinel test, however, cannot fail for the
defect it is named after. FND-002 to FND-004 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `tc_172_001` is vacuous for the licensing boundary. The sentinel is a constant invented in `pack_support`; the scan of the skill and fixture files is not connected to the pack install (the skill is never run, nothing derived from the pack is written anywhere). The pre-PR tree, with all 15 restricted reference tables present, contains no sentinel either, so this scan passes on exactly the broken state it guards. The install-and-list half re-asserts FR-137 behaviour already covered elsewhere. Replace it with a scan that would fail on the base tree: for example, no restricted control identifier pattern in the skill or fixtures, or delete it and rely on `tc_172_002` | rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs:64-96 |
| FND-002 | low | `tc_172_002` detects a restricted link only by the substrings `soc2/` and `iso27001/`. A reference re-added under another name (for example `references/tsc.md`) or a control table pasted into `component-type-mapping.md` passes. An identifier-pattern scan (as in FND-001) would cover both tests' intent | rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs:100-123 |
| FND-003 | low | `files_under` uses `read_dir(...).unwrap().flatten()`, which silently drops entries that fail to read. Both tests are negative scans, so a skipped file makes them pass. Unwrap each entry instead | rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs:31-40 |
| FND-004 | low | `SKILL.md` repeats the pack-only rule in two consecutive bullets (lines 44 and 45 both say restricted sources are served only from a locally installed pack, by clause id), and the Rules bullet scopes the no-text rule to "(SOC 2, ISO/IEC 27001)" while line 44 covers "any other licence-restricted source". Keep one statement and use the wider scope in Rules | skills/spec-security-analysis/SKILL.md:44-45, 87 |

## Disposition pass 1

Reviewed head `74150d5d97028bbbeae6a5dfe08dfc4fc4d28290`, including fix commits `eec8f59e` and `74150d5d`. Replacement reviewer; original findings and verdict above are preserved as historical records. Tools: quoin 0.30.1; quire 0.44.0 (engine 0.59.1). Original Linear comment: `0d06bfd5-180e-4bd1-adc9-93d5dfc9fdba`.

## Assurance Context

Examined `spec/assurance/AP-201-finding-quality.md` and `spec/assurance/AP-202-jev-lens-evaluation.md`. Neither applies: this change does not alter measurement/scoring/report engine behavior or evaluate a Jev lens. Unavailable selected context: none. Active exceptions: none.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The added matcher and test literals name restricted frameworks in public Rust source. This violates the dispatch standing rule that new public artifacts use generic terms and do not name paywalled standards. Replace the named matcher/test examples with a generic boundary check, for example an allow-list of the existing free-reference paths. Preserve the historical review findings unchanged. | rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs:114,173-188 |
| FND-006 | medium | The fix replaces the original pack_support::scratch() with tempfile::tempdir(). The existing scratch helper deliberately chooses a base outside every ancestor .git; the new test bypasses it. The focused run fails at the install assertion with QM024_PACK_INSIDE_WORK_TREE when the temporary base has a .git marker (as /tmp does in this sandbox). Reuse pack_support::scratch() so the valid-pack test does not exercise the inside-worktree refusal by accident. | rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs:249 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The scan now rejects restricted identifier shapes in shipped skill and fixture files; the prior sentinel-only oracle is gone. |
| FND-002 | fixed | Normalization handles punctuation/case, path and Markdown-link forms; the identifier scan also detects a table pasted under another name. The public naming-rule defect in these new literals is separately FND-005. |
| FND-003 | fixed | Directory reads, individual entries, metadata, file reads and UTF-8 decoding fail explicitly; errors are no longer flattened away. |
| FND-004 | fixed | The consecutive duplicate pack-only bullet is removed; Rules now covers every licence-restricted clause and includes reviews. |
| FND-005 | still-open | The added matcher and test literals name restricted frameworks in public Rust source. This violates the dispatch standing rule that new public artifacts use generic terms and do not name paywalled standards. Replace the named matcher/test examples with a generic boundary check, for example an allow-list of the existing free-reference paths. Preserve the historical review findings unchanged. |
| FND-006 | still-open | The fix replaces the original pack_support::scratch() with tempfile::tempdir(). The existing scratch helper deliberately chooses a base outside every ancestor .git; the new test bypasses it. The focused run fails at the install assertion with QM024_PACK_INSIDE_WORK_TREE when the temporary base has a .git marker (as /tmp does in this sandbox). Reuse pack_support::scratch() so the valid-pack test does not exercise the inside-worktree refusal by accident. |
| FND-005 | fixed | Round 2: named restricted-framework matcher and example literals are removed. The replacement enumerates only existing free-source reference paths. Its path-resolution coverage defect is separately FND-007. |
| FND-006 | fixed | Round 2: tc_172_004 imports and uses pack_support::scratch(), restoring its existing outside-Git temporary-base selection. |
| FND-007 | still-open | Round 2: The replacement link check uses only the suffix after a literal references/ instead of resolving the target relative to the containing document. A link such as [x](not-in-inventory/index.md) in references/standards-index.md produces no checked path, and [x](../unapproved/references/gdpr/index.md) is accepted as gdpr/index.md despite naming another location. Resolve local reference targets against their source document and enforce the approved reference root/path, with generic negative cases for relative targets and misleading prefixes. |

## Disposition scope

Original units below were re-examined against the fixed branch; excerpts retain the original review text for comparison. The dispositions comment includes the full original finding records and fixed excerpts.

```yaml
scope:
- id: tc_172_licensed_pack_boundary.rs
  path: rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs
  role: examined
  excerpt: 'New test binary: tc_172_001 (FR-172-AC-7) installs a synthetic example.invalid/std@1 pack and scans
    skill and fixture files for SENTINEL; tc_172_002 (FR-172-AC-8) asserts no references/soc2 or references/iso27001
    and no link containing soc2/ or iso27001/.'
- id: tc_171-needle
  path: rust/crates/quoin-cli/tests/tc_171_security_analysis_skill.rs
  role: examined
  excerpt: '"Licensed standards come from an installed pack only",'
- id: SKILL.md
  path: skills/spec-security-analysis/SKILL.md
  role: examined
  excerpt: Licensed standards come from an installed pack only. The skill ships no reference table, summary or web
    source for SOC 2, ISO/IEC 27001 or any other licence-restricted source.
- id: standards-index.md
  path: skills/spec-security-analysis/references/standards-index.md
  role: examined
  excerpt: SOC 2, ISO/IEC 27001 and every other licence-restricted source have no reference table here. They are
    mapped only from a locally installed pack, by clause id; with no pack installed the source is reported "not
    mapped, set not installed".
- id: component-type-mapping.md
  path: skills/spec-security-analysis/references/component-type-mapping.md
  role: examined
  excerpt: Restricted rows removed from each Primary Focus table and from the out-of-scope table; NIST, OWASP ASVS
    and GDPR rows unchanged.
- id: references/soc2+iso27001
  path: skills/spec-security-analysis/references/
  role: examined
  excerpt: 15 files deleted (soc2 index and CC1-CC9, iso27001 index and A5-A8); repo-wide grep finds no remaining
    link outside reviews/ and spec/reviews/.
- id: names_restricted_reference
  path: rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs
  role: examined
  excerpt: '    ["soc2", "socii", "iso27001", "isoiec27001"]'
- id: tc_172_004_fixture_pack_is_listed_as_a_pack_with_no_clause_text
  path: rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs
  role: examined
  excerpt: '    let root = tempfile::tempdir().expect("scratch root");'
```

## Current verdict

Original code findings fixed; new FND-005 and FND-006 remain open. Not mergeable.

## Disposition verification

Independent focused command, from rust/: locked-build.sh /home/peter/dev/worktrees/plat-1530-quoin/target env CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 cargo test --locked -j 4 -p quoin-cli --test tc_171_security_analysis_skill --test tc_172_licensed_pack_boundary. Result: tc_171 10/10; tc_172 4/5; exit 101. tc_172_004 failed with QM024_PACK_INSIDE_WORK_TREE because the sandbox exposes /tmp/.git. This is new FND-006, not ignored as a passing run.

Rerun through the same locked wrapper/target/settings with TMPDIR=/var/tmp outside the sandbox: cargo test --locked -j 4 -p quoin-cli --test tc_172_licensed_pack_boundary. Result: 5/5 passed, exit 0. No source changes between runs.

Base oracle: extracted origin/main (60fcdd381ca3a1eb5f131052ed5fce7934c73941) skill/fixture paths into the scratchpad base directory. Through the locked wrapper/target/settings, QUOIN_LICENSE_SCAN_ROOT pointed there and only tc_172_001 ran. Expected exit 101: the old references/iso27001/A8-TechnologicalControls.md carried restricted identifiers. Current-head scan passed.

quire trace --scope . --id FR-172-AC-3 --format json and the AC-4 equivalent both exited 0 with verifies=[] and implements=[]; only file-scope mentions remain. No live skill demonstration was performed or claimed. Trace emitted unused-import and conditional-symbol collision diagnostics outside this diff.

Explicit FR-171 and FR-172 quire validation exited 0. Each of the three scratchpad SR artifacts validated at its explicit absolute path; --summary reported 1/1 docs grammar-clean and zero grammar findings. Installed tool versions: quoin 0.30.1; quire 0.44.0, engine 0.59.1. Validation emitted semantic.unused-import for spec-artifacts-app importing spec-artifacts-iso, with no validation failure.

No aggregate gate, live model run, source edit, push, or GitHub findings comment was performed. Formatting/Clippy are coder-reported checks and were not repeated in this disposition. The branch remains 74150d5d97028bbbeae6a5dfe08dfc4fc4d28290.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The replacement link check uses only the suffix after a literal references/ instead of resolving the target relative to the containing document. A link such as [x](not-in-inventory/index.md) in references/standards-index.md produces no checked path, and [x](../unapproved/references/gdpr/index.md) is accepted as gdpr/index.md despite naming another location. Resolve local reference targets against their source document and enforce the approved reference root/path, with generic negative cases for relative targets and misleading prefixes. | rust/crates/quoin-cli/tests/tc_172_licensed_pack_boundary.rs:141-169 |

## Disposition pass 2

Reviewed head `42c9b80e4f036930722b893a9b2b724b87472318`. FND-005 and FND-006 are fixed; FND-007 remains open. Not mergeable.

Prior fixed findings were rechecked without duplicating their disposition rows. Previous review text and prior rows remain unchanged.

Rechecked frozen head 42c9b80e4f036930722b893a9b2b724b87472318 and fix diff against 74150d5d. No skill or spec source changed in this fix round; original code FND-001 through FND-004 and all four spec findings were rechecked and retain their previous fixed dispositions, with the replacement link-check regression separately recorded as code FND-007.

The licensed-pack test source contains no restricted-framework name literals. Its exact free-reference path allow-list checks the shipped references tree; tc_172_004 now calls the existing pack_support::scratch() helper.

Reviewed /tmp/codex-controls-1530-skillrun/evidence/RESULT.md, both recorded replies and content-scan.json. The record is from 74150d5d; the skill is unchanged at this head. It explicitly records failed positive pack installation and zero authored artifacts. The no-pack reply is observed; positive skill-output acceptance remains open.

Signature status: raw commit contains an SSH gpgsig header. ssh-keygen -Y check-novalidate -n git verified the extracted canonical commit payload/signature successfully (exit 0). git show --show-signature cannot check signer trust because gpg.ssh.allowedSignersFile is not configured; its displayed No signature does not mean the raw commit lacks a signature. Signer authorization was not verified. No unsigned-commit finding is justified by this configuration error.

No aggregate gate or live model run was performed in this disposition. Only the three review artifacts are updated in the repository, as explicitly requested by the dispatch. No commit, push or merge.


Independent focused Cargo tests and the extracted Rust helper probe waited for an occupied locked-build lane and were canceled before execution at the lead’s explicit request to finalize. No round-2 passing test result is claimed here; focused tests, fmt, Clippy and the expected base-oracle failure remain coder-reported. FND-007 is established by direct inspection: reference_path returns None unless rsplit_once("references/") succeeds and discards every prefix on success.
