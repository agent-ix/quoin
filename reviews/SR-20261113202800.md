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
