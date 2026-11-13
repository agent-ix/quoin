---
id: SR-20261113092140
title: "EARS conformance review of the release evidence snapshot (FR-166, FR-167, FR-168, NFR-036)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #861 (git diff origin/main...ea-101-snapshot-spec at 4325f1fb): spec/functional/FR-166, spec/functional/FR-167, spec/functional/FR-168, spec/non-functional/NFR-036"
---

# EARS conformance review of the release evidence snapshot (FR-166, FR-167, FR-168, NFR-036)

## Summary

Ticket: EA-101. I checked every Behavior statement in FR-166, FR-167 and FR-168, and the NFR-036 statement,
against the EARS patterns. Most statements are well-formed ubiquitous ("The command SHALL"), event-driven
("When a user runs") or unwanted-behaviour ("If ... then ... SHALL") statements, each with a single response.
Three statements do not conform.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "The command SHALL read the clock for no value; the only instant in the snapshot is `--as-of`" is an oblique negative joined to a second claim. Split it into "The command SHALL NOT read the system clock." and "The command SHALL take every instant in the snapshot from `--as-of`." | spec/functional/FR-166-release-snapshot-command.md (Behavior) |
| FND-002 | low | "The command SHALL NOT add, drop or reorder-by-judgement an entry other than by the ordering above" combines three prohibitions with an exception, and "reorder-by-judgement" is undefined. State the requirement as "The command SHALL write exactly the supplied entries, ordered as Outputs states." | spec/functional/FR-168-release-snapshot-change-records.md (Behavior) |
| FND-003 | low | The NFR-036 statement is compound. It requires determinism (byte identity) and a separate no-verdict property in one SHALL, under a single `reliability` attribute. Split it into two statements, or two NFRs, so each metric traces to one property. | spec/non-functional/NFR-036-release-snapshot-is-deterministic-and-never-decides.md (Statement) |

## Verdict

Minor. These are wording fixes with no change in behaviour.
