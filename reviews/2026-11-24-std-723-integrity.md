---
id: SR-2026112502011202
title: "integrity review of STD-723 signed case acceptance"
type: SpecReview
analysis: integrity
scope: "spec/functional/FR-225-signed-acceptance-of-case-content.md, spec/functional/FR-226-sacm-export.md, spec/non-functional/NFR-045-assurance-exports-run-offline.md"
review_set: subset
---

## Summary

Traced the selected approval from the CLI through EA FR-041 to the two exports. Reviewed caller scope, subject construction, trust isolation and report ownership at head `8274f4d01c6dd244bbb9abd7c65809d1b3b9fe8b`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No open integrity findings. | FR-225 |

## Verdict

**PASS.** FR-221 supplies the validated model and FR-225 defines the optional approval record before FR-226 maps it to the separate Artifact. Both model derivations share the RFC 8785 encoding of the complete unsigned GSN model. FR-098 already names the shared encoder, whose value-based interface refuses out-of-domain integer values. FR-225 preserves the underlying export if encoding fails.

The three CLI inputs are scoped to case verification and preserve the independent clause, formal-rule and evidence inputs. The request protocol and field mapping match EA FR-041. The claimed signing time and EA's SSH missing-trusted-time reason remain distinguishable. EA determines signing-key validity, so an additional Quoin expiry calculation would change the agreed policy.

The acceptance criteria cover the exact signed subject, successful and failed dispositions, metadata projection and independence from assurance claims. FR-226-AC-9 checks the separate Artifact placement. No additional pledge about product acceptability or evidence sufficiency is introduced.

