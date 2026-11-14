# Security analysis profile fixtures

`a/output/` and `b/output/` are the control, scope-exclusion and SpecReview artifacts emitted by live `codex exec` runs following `skills/spec-security-analysis/SKILL.md` on 2026-11-13. The two synthetic `.ix/source-profile.yaml` inputs select `web-app-baseline` and `owasp-only`, respectively. The outputs contain only synthetic set keys and clause identifiers; no source corpus or vendored clause text is included.

`tc_171_profile_mappings.rs` evaluates equivalent synthetic profile definitions at test time, parses every mapping from these emitted artifacts, and checks each against the evaluation report. The tests assert that non-binding, unresolved and out-of-profile mappings are absent, and that unresolved findings name missing context dimensions.
