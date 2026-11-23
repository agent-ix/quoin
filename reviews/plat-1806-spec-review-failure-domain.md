---
id: SR-20261123163421
title: "Failure-domain review of PLAT-1806 evidence record exit status and pytest symbol mapping"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quoin; spec/functional/FR-030-evidence-store.md; spec/functional/FR-033-evidence-format-adapters.md (PR #1025, ticket PLAT-1806)"
review_set: subset
---

# Failure-domain review of PLAT-1806 evidence record exit status and pytest symbol mapping

## Summary

Ticket: PLAT-1806. This review looked for edge inputs that the new ACs leave unspecified. It found three gaps: the exit status of a zero-entry run, the output when both error conditions hold, and a classname prefix that does not spell the file path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-030-AC-34 sends zero-entry runs to FR-030-AC-30, but AC-30 governs qualification, not exit status; the exit of `{"entries":[]}` or an empty SBOM is unstated (AC-33's "at least one entry" implies zero; say so) | spec/functional/FR-030-evidence-store.md:260,264 |
| FND-002 | low | When AC-32 and AC-33 both hold, for example an sbom run with `--discharges NO-SUCH-1` only, whose entries carry no id and whose unmatched list is non-empty, the spec does not say whether both stderr lines are written or in what order | spec/functional/FR-030-evidence-store.md:262-263; rust/crates/quoin-core/src/ops/evidence/records.rs:278-322 |
| FND-003 | low | Step 2 does not say what happens when the front of `classname` does not spell the `file` path (for example `file="tests/test_bolus.py" classname="other.TestDose"`): keep every segment as a class, or treat it as not a pytest case | spec/functional/FR-033-evidence-format-adapters.md:90-95 |

## Verdict

These are edge gaps that can be closed with one sentence each in the same PR. None blocks the main path.

## Examined

FR-030-AC-32, FR-030-AC-33, FR-030-AC-34, FR-033 pytest procedure section, FR-033-AC-16 (examined). FR-030-AC-30, FR-041-AC-8 (context only).
