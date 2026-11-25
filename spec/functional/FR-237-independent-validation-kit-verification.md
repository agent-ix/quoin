---
id: FR-237
title: "Independently verify each released tool validation kit"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-235"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-236"
    type: "depends_on"
---

# FR-237: Independently verify each released tool validation kit

## Description

For each released tool validation kit, the release process SHALL record an
independent verification by a named reviewer who did not author the self-test
cases being verified, so the evidence shows whether the packaged kit detects
known faults as well as recognizing the released tools.

## Inputs

- The released validation kit and the exact tool binaries it validates
- The release identity and the source revisions that identify the kit and cases
- The supported clean verification environment and required external tools
- Controlled fault variants for the kit's declared comparison outputs

## Outputs

- An independent verification record identifying the kit release, each tool
  source revision and binary version, operating system and architecture,
  reviewer, case authors, and overall outcome (`passed`, `incomplete`, or
  `failed`)
- The outcome and retained run-record reference for each executed case
- A finding for each failed, errored, or not-executed case and each seeded fault
  the kit does not detect

## Behavior

- The verification SHALL run the released kit and tool binaries on a clean
  machine without using a developer checkout as the binary under test.
- The verification SHALL identify the release and source revision for each
  tested binary and the versions of external tools used by the run.
- The independent verification record SHALL carry the source revisions
  separately from the FR-236 run record.
- The verification SHALL list the exact self-test cases selected and retain the
  per-case outcomes using [FR-236](./FR-236-selftest-run-record.md).
- The verifier SHALL record their identity separately from the authors of the
  cases under review.
- If the verifier or case authors cannot be identified, the verifier SHALL mark
  the verification incomplete.
- The verification SHALL apply a controlled mismatch to every output-comparison
  channel the kit declares, including exit status, standard output, and expected
  files where those channels are used.
- When the verifier applies a seeded mismatch, the verifier SHALL record whether
  the kit detects it and returns a failing result for the affected case.
- When a tool is missing, the environment is unsupported, a case is skipped, or
  a seeded mismatch is not detected, the verifier SHALL record the verification
  as incomplete or failed.
- When the verifier and all tested case authors are identified, the verifier is
  independent of those authors, every selected released case completes with its
  expected result, and every seeded mismatch in a declared comparison channel
  is detected, the verification SHALL report `passed`.
- The verification record SHALL contain test outcomes and findings only.
- Where the verifier prepares a verification record, the verifier SHALL NOT
  assign a process-risk rating, compliance result, or customer acceptance
  decision.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-237-CON-1 | The reviewer SHALL NOT be an author of any self-test case included in the verification. | Process | Inspection |
| FR-237-CON-2 | The verifier SHALL identify each tool binary by its released version. | Configuration | Inspection |
| FR-237-CON-3 | A local development build SHALL NOT substitute for a released tool binary. | Configuration | Inspection |
| FR-237-CON-4 | A skipped, unavailable, or failed case SHALL NOT count as successful independent verification. | Safety | Test |
| FR-237-CON-5 | The verification SHALL NOT imply that a person has accepted the tool for a regulated use. | Compliance | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-237-AC-1 | Given a released kit run on a clean machine, the record identifies the kit and tool release versions, source revisions, operating system, architecture, external-tool versions, reviewer, and tested case authors. | Test |
| FR-237-AC-2 | Given an independent verification run, every selected case has a retained outcome; a missing, skipped, errored, or not-executed case is visible and does not count as passed. | Test |
| FR-237-AC-3 | Given controlled mismatches for each output-comparison channel used by the kit, each affected case returns a failing result that identifies the case and mismatch; the unmodified released case retains its expected result. | Test |
| FR-237-AC-4 | Given a reviewer who authored a tested case, or missing reviewer or case-author identity, the record reports incomplete independence and does not report verification as passed. | Test |
| FR-237-AC-5 | Given any verification result, the record states outcomes and findings without assigning a process-risk rating, compliance result, or customer acceptance decision. | Inspection |
| FR-237-AC-6 | Given an independent reviewer with identified tested case authors, a clean supported environment, all selected released cases completing with their expected results, and every seeded output mismatch detected, the record reports `passed`; if any case is skipped, unavailable, or errored, or any seeded mismatch is not detected, it does not report `passed`. | Test |

## Dependencies

- **Upstream**: [FR-235](./FR-235-quoin-selftest.md) defines the packaged self-test cases; [FR-236](./FR-236-selftest-run-record.md) defines retained run records.
- **Downstream**: [FR-217](./FR-217-validation-report.md) may present the resulting run evidence.
- [PLAT-1849](https://linear.app/agent-ix/issue/PLAT-1849/independent-verification-of-the-validation-kit) owns the independent verifier and its retained record. FR-237-AC-1 through FR-237-AC-4 and FR-237-AC-6 are downstream implementation criteria; this specification change defines them but does not implement the verifier. FR-237-AC-5 remains an inspection criterion for the record's output boundary.
