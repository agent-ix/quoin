---
id: SR-20261113140426
title: "Gap analysis of typed control and clause-set output (FR-171 to FR-174, NFR-038)"
type: SpecReview
analysis: gap-analysis
review_set: subset
scope: "agent-ix/quoin; PR #871 (git diff origin/main...plat-1426-typed-controls-spec at 20e50a4a): spec/functional/FR-171 to FR-174, spec/non-functional/NFR-038, against skills/spec-security-analysis and rust/crates/quoin-assurance/src/discharge on the same head"
---

# Gap analysis of typed control and clause-set output (FR-171 to FR-174, NFR-038)

## Summary

Ticket: PLAT-1426. This PR changes only spec files, so no new test or code is expected. Plan completion: not
assessed. The new criteria have no tagged tests yet. That is the expected state for a spec ticket, so it is not
recorded as a finding. The analysis checked each existing surface the new requirements build on against the code
at the PR head:

- `quoin discharge --repo --rmm` renders `Compliant`, `Tailored`, `Not compliant - undischarged` and `Not compliant - attestation not current` (quoin-assurance discharge/rmm.rs:75-78).
- The FR-046 parser accepts disposition decision `delegated` and requires non-empty `evidenceRefs` and `approvalRef`.
- The clause-set key renders as `authority/id@version` (quoin-modules pack.rs:29).
- quire-rs `check_publishable` exists (src/clauses.rs:720), and `quire clauses evaluate` exists in quire 0.44.0.
- The skill today still writes Section 19 (SKILL.md:61-70) and cites "NIST SP 800-53 Rev 5.2.0" (SKILL.md:20). The skill change FR-171 asks for is not yet made, as expected.

The one spec-versus-code contradiction found, the FR-173 attestation fields that the FR-046 parser rejects, is
recorded once, as FND-001 of SR-20261113140423, and is not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

No gap that is unique to this method. `quoin evidence facts-from-mappings` does not exist yet, which is correct
for a spec-only PR.
