---
id: FR-149
title: "Grype advisory report adapter"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-034"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-041"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-142"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-044"
    type: "references"
---

# FR-149: Grype advisory report adapter

## Description

Grype is the one scanner that reads the Syft CycloneDX SBOM against public advisory databases, chosen and run
through `producer-execute` by `ix://agent-ix/engineering-assurance/FR-044`. Its JSON report reaches quoin as a
finding-shaped evidence adapter of the FR-033 registry, beside `sarif`, `cargo-audit` and `cargo-deny`, and is
retained as a FR-034 `FindingRecord`. The adapter is pure over text: it runs no scanner, reads no SBOM and
fetches no database (FR-033). The join onto `soup_record` entries is [FR-142](./FR-142-advisory-join-onto-soup-records.md).

### What the report must prove

As with cargo-deny (FR-034), the existence of the `FindingRecord` is the proof the scan executed, so a report
is recorded only when its envelope proves a scan ran: a JSON object with a `matches` array, a `descriptor`
naming Grype and its version, and a database status carrying the database build time. An empty `matches`
array inside that envelope is a clean scan. A document without the envelope, or an empty or truncated file, is
refused and never read as clean.

A report is also refused when its `ignoredMatches` array is non-empty, because a match the run set aside
(through an ignore rule, a VEX statement or a fix-state filter) is a diagnostic the report suppressed, and its
absence from `matches` would read as clean. The run definition of FR-044 uses an empty configuration, so an
ignored match marks a run that departed from it.

### Finding shape

| Finding field | Source in the report |
| --- | --- |
| rule id | `vulnerability.id` of the match, as written (`GHSA-`, `CVE-`, or a distribution advisory id). |
| severity | `vulnerability.severity`, Grype's own word, unchanged and unmapped (FR-034-CON-2). |
| message | `vulnerability.description` where present, else the rule id. |
| path | `artifact.purl` of the matched package; where a match has no purl, `<artifact.name>@<artifact.version>`. |

Grype reports no count of the rules it evaluated, so `rulesEvaluated` stays unset and the vacuity check stays
silent rather than guess. The ruleset recorded is the database build time, so the evidence states which
advisory data the scan used.

## Behavior

- The adapter SHALL be selected by `--adapter grype` and by `--tool grype`.
- The adapter SHALL read each member of `matches` as one finding of the shape above.
- The adapter SHALL record the database build time from the report as the scan's ruleset.
- The adapter SHALL leave `rulesEvaluated` unset.
- If a match names no `vulnerability.id`, then the adapter SHALL skip it and count it as skipped.
- If the document is not a JSON object, has no `matches` array, has no `descriptor` naming Grype or has no database build time, then the adapter SHALL refuse it.
- If `ignoredMatches` is present and not empty, then the adapter SHALL refuse the document.
- If the document is refused, then `quoin evidence record` SHALL write no `FindingRecord` and exit non-zero with an error naming the reason.
- The adapter SHALL NOT run Grype, read an SBOM, read a vulnerability database or open the network.
- The adapter SHALL NOT compute a score, class or ranking.
- The adapter SHALL NOT map a severity word.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-149-AC-1 | `quoin evidence record --adapter grype --results <file>` over a captured Grype report with matches writes a `FindingRecord` under `scans/`, one finding per match, with rule id, Grype's own severity word and the artifact purl as path, and writes nothing under `runs/`. | Test |
| FR-149-AC-2 | A captured Grype report with a valid envelope and an empty `matches` array is recorded as a scan with `findings: []`, distinguishable from no record. | Test |
| FR-149-AC-3 | A truncated file, a non-JSON file, a JSON object with no `matches` array, and a JSON object with `matches` but no `descriptor` or no database build time are each refused with a non-zero exit and no `FindingRecord`. | Test |
| FR-149-AC-4 | A report whose `ignoredMatches` array is non-empty is refused with an error naming `ignoredMatches`, and writes no `FindingRecord`. | Test |
| FR-149-AC-5 | A match without a `vulnerability.id` is skipped and counted, and the other matches are recorded. | Test |
| FR-149-AC-6 | `--tool grype` selects the adapter, and `--tool cargo-audit` and `--tool cargo-deny` still select theirs. | Test |
| FR-149-AC-7 | A match with no purl is recorded with the path `<name>@<version>`. | Test |
| FR-149-AC-8 | The recorded ruleset states the database build time, and `rulesEvaluated` is unset so the vacuity check is silent. | Test |
| FR-149-AC-9 | The adapter source opens no file other than its input, runs no subprocess and opens no network connection. | Inspection |
| FR-149-AC-10 | The adapter fixtures are captured from a Grype run over the FR-039 npm, Python, Go module and container fixture SBOMs and checked in unedited. | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md), [FR-034](./FR-034-finding-shaped-evidence.md); `ix://agent-ix/engineering-assurance/FR-044` produces the report.
- **Downstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-144](./FR-144-soup-report.md)
