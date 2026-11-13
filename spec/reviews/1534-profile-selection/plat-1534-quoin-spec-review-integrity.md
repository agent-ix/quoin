---
id: SR-20261113140010
title: "Integrity review of source profile selection and the web-app-baseline profile"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quoin; PR #870 at 9e8997c3: spec/functional/FR-180, FR-181, FR-182, spec/non-functional/NFR-037; context: agent-ix/spec-objects-security PR #48 FR-020, FR-021; tickets PLAT-1521, PLAT-1522"
---

# Integrity review of source profile selection and the web-app-baseline profile

## Summary

Ticket: PLAT-1534. Checked the quoin FRs against the engine they call and against the default `web-app-baseline`
profile (spec-objects-security PR #48 FR-021), which draws on the NIST SP 800-53 Rev 5 set (PLAT-1521) and the
OWASP ASVS 5.0 set (PLAT-1522).

PLAT-1521 and PLAT-1522 exist in the same project ("Security Analysis to Typed Controls", milestone M1, Backlog).
The author's statement that no NIST or OWASP set ticket exists is wrong. What matches: both sets are free sources,
and PR #871 FR-181 keys them as `nist/sp800-53@r5` and `owasp/asvs@5.0.0` with `text: none`, so they are publishable.
What does not match is applicability. Both tickets make a level an applicability dimension (NIST baselines
low/moderate/high, ASVS levels), so that "a profile can bind only the baseline it needs". The profile specs do not
handle that.

The engine check holds. `ClauseSet::evaluate` copies the whole context into the report and ignores a dimension the
set does not declare. A missing dimension yields `missing_dimension` (unresolved) and an undeclared value yields
`unknown_value` (unresolved). `clause-binding-v1` has no text or title field, so FR-182-AC-5 and the byte-equal
`--out-dir` of FR-181-AC-7 can both hold.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-181 sends one context to every set. If two sets in a profile use the same dimension name with different value sets, a value valid for one set is `unknown_value` (unresolved) in the other. NIST baseline and ASVS level in `web-app-baseline` are the likely case. The spec has no rule to prevent it: neither distinct dimension names per profile (a refusal in FR-180, or in FR-020 validation) nor a per-set context. | spec/functional/FR-181-source-profile-evaluation.md:23-25 |
| FND-002 | medium | `web-app-baseline` is inconsistent with PLAT-1521 and PLAT-1522. Those tickets have a profile bind only the baseline or level it needs. FR-021 lets a profile's context state only facts its name implies, and bars product facts. Neither FR-021 nor FR-180 decides whether the NIST baseline and ASVS level come from the profile (one profile per level) or from the repo's `context`. As specified, every baseline- or level-gated clause evaluates `unresolved`. Decide it, and add an AC that evaluates `web-app-baseline` with the level dimensions set. | spec-objects-security spec/functional/FR-021-default-source-profiles.md:17-35 (PR #48); spec/functional/FR-180-source-profile-selection.md:26-30 |
| FND-003 | low | FR-021 cites PLAT-1593, PLAT-1612 and PLAT-1627 for the other packs, but says the NIST and OWASP sets come from "their own pack work". PLAT-1521 and PLAT-1522 exist. Either cite them, or drop the ticket list as the separate PR #48 review (SR-041 FND-010) proposes. | spec-objects-security spec/functional/FR-021-default-source-profiles.md:23-24 (PR #48) |

## Verdict

Consistent with the engine. FND-002 leaves the general web-app default inconsistent with the NIST and ASVS set
tickets, and it shares a root cause with FND-001.

Examined (role examined): FR-181 Description and Behavior; FR-180 context merge; FR-182-AC-5. Context
only: spec-objects-security PR #48 FR-020, FR-021; quoin PR #871 FR-181; PLAT-1521; PLAT-1522;
quire-rs src/clauses.rs evaluate and schemas/output/clause-binding-v1.schema.json.
