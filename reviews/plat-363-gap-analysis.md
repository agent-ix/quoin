---
id: SR-20261111175656
title: "Gap analysis of quoin PR 803 (FR-040 strategy nodes and bound-leaf evidence)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@83c7baf0; spec/functional/FR-040-assurance-case-view.md, spec/matrix.md, rust/crates/quoin-assurance/tests/case_rendering.rs, rust/crates/quoin-assurance/src/case.rs"
review_set: subset
---

# Gap analysis of quoin PR 803

## Summary

Ticket: PLAT-363. Planless; plan completion was not assessed. The review read the computed
matrix (`quire matrix --scope .`) at 83c7baf0 for FR-040, plus the FR-040 prose and the
criteria the PR added.

FR-040-AC-16 and FR-040-AC-17 are both `untagged`. Their tests carry criterion-only markers,
and quire reports `criterion-tagged-outside-row` because no `spec/matrix.md` row exists.

Pre-existing and outside this diff, FR-040-AC-15 (TC-1711) has the same defect: it has no
TC-1711 row. It is worth folding into the same fix.

## Verdict

**NOT MERGEABLE. Two medium findings and one low.** Neither new criterion is backed in the
computed matrix. The FR-040 prose defines `fresh` the same wrong way the code does.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-040-AC-16 and AC-17 have no TC id in the Verification column and no `spec/matrix.md` row, and the tests are tagged `#[trace("FR-040-AC-16")]` without a TC id. `quire matrix` reports both as untagged, with `criterion-tagged-outside-row`. Allocate the next unused TC ids (TC-2141 and TC-2142 on origin/main at review time; re-check first), add the rows and cite them in FR-040. Tag the tests `#[trace("FR-040-AC-16", "TC-2141")]` / `#[trace("FR-040-AC-17", "TC-2142")]`, and extend the FR-040 summary row at matrix.md:100 | spec/functional/FR-040-assurance-case-view.md:143-144, rust/crates/quoin-assurance/tests/case_rendering.rs:226, rust/crates/quoin-assurance/tests/case_rendering.rs:262, spec/matrix.md:100 |
| FND-002 | medium | The FR-040 prose defines `fresh` as "bound, nothing raised", which is weaker than the auditor's `healthy`. An obligation whose ladder stops early, or that has an unevaluated check, raises nothing and is not healthy. The prose also says "a leaf raising any other finding states no verdict", where FR-115 says `undischarged`. Restate `fresh` as "the auditor reported it healthy" and name the remaining state, matching the code fix | spec/functional/FR-040-assurance-case-view.md:44-50 |
| FND-003 | low | FR-040-AC-17 is compound: listing suites, the newest timestamp, the verdict, two output formats and byte-identity. Its single test cannot fail on one clause without the others. Consider splitting out byte-identity, which FR-040-AC-8 already states | spec/functional/FR-040-assurance-case-view.md:144 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-040-AC-16, path: spec/functional/FR-040-assurance-case-view.md, role: examined, excerpt: "A goal decomposed into sub-claims has a strategy node, in markdown and mermaid, that names the edge verbs and the sub-claims argued over each."}
- {id: FR-040-AC-17, path: spec/functional/FR-040-assurance-case-view.md, role: examined, excerpt: "A bound leaf lists its suites, the newest run or scan timestamp and the auditor's fresh, stale or suspect verdict, in markdown and `--json`, byte-identically over unchanged inputs."}
- {id: FR-040, path: spec/functional/FR-040-assurance-case-view.md, role: examined, excerpt: "A leaf raising any other finding states no verdict and keeps the finding in its reason."}
```
