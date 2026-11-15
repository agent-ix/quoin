---
id: SR-20261115183514
title: "EARS conformance — PLAT-1441 FR-121 frontmatter-owned record keys"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-121-record-key-sections-mapping.md; PLAT-1441"
review_set: subset
---

## Summary

Ticket: PLAT-1441. EARS review of the four statements added or changed in
FR-121 by quoin PR #927 (`0799418e`). `quoin`'s automated EARS lint could not be
run (the sandbox requires approval for the binary), so this is a manual reading.

## Verdict

**CONDITIONAL.** Lines 86 and 87 are well-formed unwanted-behaviour (`If ... then
Quire SHALL ...`) statements with one response each. Line 80 is a well-formed
negative ubiquitous statement. Line 88 cannot be verified as written, and line 85
has a non-system subject.

## Coverage

Examined: FR-121 Behavior line 80 (frontmatter prohibition with exception),
line 85 (ownership), line 86 (section-only error), line 87 (both-form warning),
line 88 (unchanged behaviour). All are listed in the comment scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Line 88, "Keys owned by any other mapping, and every lifecycle key read from a body section, SHALL keep the behavior stated in this requirement", points back at its own requirement and states no response. It is also ambiguous when a module declares a `frontmatter_field` locator for a lifecycle key such as `status`: is that key "a lifecycle key read from a body section" or frontmatter-owned? | spec/functional/FR-121-record-key-sections-mapping.md:88 |
| FND-002 | low | The subject of line 85 is a locator ("A `frontmatter_field` locator ... SHALL own that key"), not the system, and its second sentence ("An absent frontmatter field leaves the key absent; it does not hand the key to a section") has no SHALL. Rephrased as "Quire SHALL treat a key ... as frontmatter-owned ... and SHALL NOT read it from a section", it could be verified. | spec/functional/FR-121-record-key-sections-mapping.md:85 |
