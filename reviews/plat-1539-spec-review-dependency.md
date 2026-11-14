---
id: SR-20261114002408
title: "Dependency review of PLAT-1539 quoin#893"
type: SpecReview
analysis: dependency
scope: "agent-ix/quoin; spec/functional/FR-171-security-analysis-emits-typed-controls.md"
review_set: subset
---

## Summary

The amendment introduces dependencies on FR-180/FR-181 and spec-objects-security FR-021. The local links resolve, but the FR-021 relative link is broken and the frontmatter does not express the new local prerequisites as dependency edges.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The dependency list links FR-021 as `./FR-021-default-source-profiles.md`, which does not exist in this repository; FR-021 belongs to spec-objects-security. Replace the relative link with a resolvable cross-repository reference. | spec/functional/FR-171-security-analysis-emits-typed-controls.md:81 |
| FND-002 | low | FR-180 and FR-181 are prerequisites for applying the selected profile, but their frontmatter relationships are `references` while the dependency section calls them upstream. Declare the relationship type that matches their implementation dependency. | spec/functional/FR-171-security-analysis-emits-typed-controls.md:10-13 |

## Scope

- FR-171 frontmatter relationships and dependency section examined; FR-180, FR-181 and spec-objects-security FR-021 read as context.
