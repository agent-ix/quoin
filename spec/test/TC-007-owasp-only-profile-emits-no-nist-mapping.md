---
id: TC-007
title: "OWASP-only profile emits no NIST mapping"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-171"
    type: verifies
---

# TC-007: OWASP-only profile emits no NIST mapping

## Description

Verify the profile-driven security-analysis behavior required by FR-171 using the named focused integration test and its synthetic profile fixtures.

## Test Procedure

1. Run `tc_171_profile_mappings::fixture_b_owasp_only_profile_has_no_nist_mapping` against Fixture B.

## Expected Results

The evaluation contains only `owasp/asvs@5.0.0`. Each actual emitted mapping uses that key and a binding clause; no NIST mapping is present.
