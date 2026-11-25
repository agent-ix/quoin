---
id: FR-220
title: "Record external tool versions in the validation report"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-217"
    type: "depends_on"
---

# FR-220: Index external tools and record their versions in the validation report

## Description

The validation report SHALL resolve each external tool used by a retained run
to one entry in a reviewed configuration index and record the version
observed during that run, so a customer can interpret tool results in the
configuration in which they were produced.

## Behavior

- The quoin repository SHALL maintain the reviewed index as the authoritative
  source of configured external-tool IDs at the stable path
  `spec/validation/external-tools.v1.json`; the index SHALL declare
  `format_version: "1"` and an `entries` array.
- Each entry SHALL have a unique, non-empty canonical `id`, an `executable` identity
  (the executable name or path expected on PATH), and a non-empty
  `version_invocation` argument array passed to that executable.
- The index `entries` array SHALL contain exactly nine entries, one each with
  canonical IDs `git`, `ssh`, `gitsign`, `syft`, `osv-scanner`, `pandoc`,
  `typst`, `z3`, and `cvc5`; it SHALL contain no other entries. Pandoc and
  Typst SHALL have separate entries.
- Each retained run's external-tool record SHALL contain `tool_id` (the
  canonical entry ID), `executable` (the resolved executable identity),
  `version_invocation` (the exact argument array),
  `observed_version_output` (UTF-8 text), `observed_version_stream` (`stdout`
  or `stderr`), `index_path`, and `index_format_version`.
- The report generator SHALL resolve every external-tool record to exactly one
  index entry by canonical ID.
- When an index contains a duplicate ID, the report generator SHALL mark every
  entry for that ID as `not provided`.
- An invalid index entry SHALL not invalidate otherwise valid entries.
- When the index is absent or invalid, or a run's executable identity, version
  invocation, index path, or format version differs from the applicable entry,
  the report generator SHALL mark that evidence `not provided`.
- A `not provided` external-tool row SHALL identify the affected tool.
- A run that requires an external tool that is unavailable SHALL record the
  tool as `not-executed`; the report SHALL NOT count it as a pass.
- The report SHALL present index contents and observed output as
  human-reviewed evidence. The report SHALL NOT calculate scores or infer
  acceptability from tool versions.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-220-AC-1 | Given the versioned index at the stable path, it declares format version `1` and exactly nine entries with the required canonical ID set, each with `executable` and `version_invocation`. | Test |
| FR-220-AC-2 | Given a retained run record for any indexed external tool, the report resolves exactly one matching entry and shows `tool_id`, `executable`, `version_invocation`, `observed_version_output`, `observed_version_stream`, `index_path`, and `index_format_version`. | Test |
| FR-220-AC-3 | Given an unavailable required external tool, the result is `not-executed` and the report does not present it as passing. | Test |
| FR-220-AC-4 | Given a missing or duplicate index entry, or a run record whose executable identity or invocation differs from the index, the report states that the affected tool evidence was `not provided`, preserves unaffected valid index entries, and does not present the affected evidence as passing. | Test |

## Dependencies

- **Upstream**: PLAT-1641 defines the external-tools configuration index; PLAT-1817 records local real-tool runs.
- **Downstream**: [FR-217](./FR-217-validation-report.md) presents the indexed tool data.
