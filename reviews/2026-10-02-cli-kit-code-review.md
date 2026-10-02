---
id: SR-180
title: "Shared CLI foundation diff review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@326d5f42efe8ace26c08a9578488ece8be9052fb; all 27 files in origin/main...HEAD, including workflow diff (empty), manifests, lockfile, deny policy, Rust idioms, FR-096, SR-179, quoin-core protocol/error/operation imports, quoin-cli adapters and delivery documentation"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-096"
    type: reviews
---

# SR-180: Shared CLI foundation diff review

## Summary

Review of the shared foundation adoption against FR-096 and the repository Rust conventions.

## Verdict

Pass after the documentation correction below. The local taxonomy and recursive
sorter are deleted. Every affected consumer directly names the authoritative
shared outcome. Domain error classification and the diagnostic array remain
in Quoin. The encoder's underlying serde error preserves the prior message.
The lockfile adds only ix-cli-kit and two dependency edges; no secret feature
or OS store is enabled. No workflow lane changed, panic/unsafe was added, or
resource boundary relaxed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The touched boundary checklist still claims preserve_order is disabled and sorting is implicit, although the replacement explicitly sorts. A later maintainer could incorrectly remove explicit sorting based on this instruction. | .claude/skills/rust-style/SKILL.md:61 at 326d5f42 |

## Dispositions

FND-001 fixed in the review-artifact commit: the checklist now describes shared
explicit recursive sorting and downstream Io error translation.

## Gates

`make rust-gate CARGO_TARGET=/Users/peter/dev/.cli-kit-adoption-target` exited 0:
format, workspace all-target/all-feature clippy with -D warnings, cargo deny,
workspace tests and doctests passed. The ordinary committed semantic packages
were installed with the frozen pnpm lockfile; no source or binary was copied.
