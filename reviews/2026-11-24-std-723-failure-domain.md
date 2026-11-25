---
id: SR-2026112502011203
title: "failure-domain review of STD-723 signed case acceptance"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-225-signed-acceptance-of-case-content.md, spec/functional/FR-226-sacm-export.md, spec/non-functional/NFR-045-assurance-exports-run-offline.md"
review_set: subset
---

## Summary

Reviewed partial clones, invalid selections, verifier failures, subject mismatches and trust isolation against EA FR-041 and NFR-045. The offline contract was corrected at head `8274f4d01c6dd244bbb9abd7c65809d1b3b9fe8b`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved. EA's Git reads for tag objects and committed trust blobs can trigger promisor fetches. FR-225 now requires disabled Git transport in EA and its children, preserves the case on a missing local object and adds a partial-clone check. | FR-225, FR-225-AC-5, NFR-045 |

## Verdict

**PASS.** The revision requires the export to succeed without positive acceptance for a selected tag with malformed inputs, absent local objects, unavailable EA or an unqualified verdict. A caller that selects no tag avoids the verifier and acceptance diagnostics.

EA's `signature_verify_host.rs` at `b78efad10` obtains a committed allowed-signers source using Git subprocesses. A local file-transport fixture on terra with Git 2.34.1 confirmed that a direct missing-blob read invokes the promisor fetch and transport. With Git transports disabled, the same read refused the missing object before opening a transport. The updated requirement covers this failure without imposing a specific mechanism.

Tag selection and trust sources come from explicit inputs. EA performs tag and signer resolution, the case digest and record kind bind the subject, and Quoin leaves graph validation and claim support under their existing requirements.

