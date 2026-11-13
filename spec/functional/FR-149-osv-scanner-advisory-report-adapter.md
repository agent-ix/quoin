---
id: FR-149
title: "OSV-Scanner advisory report adapter"
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
    type: "depends_on"
---

# FR-149: OSV-Scanner advisory report adapter

## Description

OSV-Scanner (`google/osv-scanner`, Apache-2.0) is the one scanner that reads the Syft CycloneDX SBOM against
public advisory databases, chosen by the owner and run through `producer-execute` by
`ix://agent-ix/engineering-assurance/FR-044`. Its JSON report reaches quoin as a finding-shaped evidence
adapter of the FR-033 registry, beside `sarif`, `cargo-audit` and `cargo-deny`, and is retained as a FR-034
`FindingRecord`. The adapter is pure over text: it runs no scanner, reads no SBOM and fetches no database
(FR-033). The join onto `soup_record` entries is [FR-142](./FR-142-advisory-join-onto-soup-records.md).

### Adapter boundary

The adapter is the only place that knows the OSV-Scanner JSON layout. It emits the same finding shape as every
other finding adapter and records the tool name `osv-scanner`. The join (FR-142) and the report (FR-144) read
`FindingRecord` entries by tool name and know nothing of OSV-Scanner's layout, so a later scanner using a new adapter and tool name is added, and the join is unchanged except for one row of its assessed-types table.

### Research status

The documented report shape below was reviewed from OSV-Scanner project documentation and source. A fixture-only runtime
check confirmed `-L` accepts a retained SBOM named with a `.cdx.json` suffix; with no local offline database, the scan stopped
with an error before producing a report. This did not verify vulnerability matching, report completeness, or adapter behavior.
The Rust tests use authentic captured reports and fixture-only malformed inputs; no captured report is fabricated.

| # | Statement | Status |
| --- | --- | --- |
| 1 | `--format json` writes one JSON object; its `results` array holds one entry per scanned source, each with `source` and `packages`; a package has `package` (`name`, `version`, `ecosystem`, optional `purl`), `vulnerabilities` (full OSV records) and `groups` (`ids`, `aliases`, `max_severity`). Source: `pkg/models/results.go`, `docs/output.md`. | VERIFIED |
| 2 | The JSON object names no database, zip or database version. Source: `pkg/models/results.go` (top level is `results`, `experimental_config`, optional `image_metadata`, `license_summary`). | VERIFIED absent |
| 3 | The JSON object states no ignore or suppression record, so a vulnerability set aside by an `osv-scanner.toml` ignore rule is absent without a trace. | UNVERIFIED (no such field is in `pkg/models/results.go`; the suppression itself was not exercised) |
| 4 | With `--all-packages`, the JSON lists packages that have no vulnerabilities, so a clean scan of a non-empty SBOM names its packages. `docs/usage.md` states the flag outputs all packages in JSON. | UNVERIFIED (documented, not run) |
| 5 | The JSON for a scan that found no packages is not distinguishable from a clean scan by content alone; exit code 128 ("no packages found") distinguishes it, and that is a `failed` state upstream (FR-044). | VERIFIED for the exit code, UNVERIFIED for the content |
| 6 | `group.max_severity` is a string; its exact vocabulary (a number or a word) is not stated in the documentation read. | UNVERIFIED |
| 7 | `package.purl` is present for a Syft-purl input; for npm, PyPI and Go it equals the Syft purl up to spelling. | UNVERIFIED |

## What the report must prove

As with cargo-deny (FR-034), the existence of the `FindingRecord` is the proof the scan executed, so a report
is recorded only when it proves a scan ran over packages: a JSON object whose `results` is an array holding at
least one source, whose `packages` array is non-empty. A package with no `vulnerabilities` is a clean package. A
report whose packages all have none is a clean scan. A document that is not an object, an empty or truncated
file, an object with no `results` array, an empty `results` array, or a source with no `packages` array is
refused and never read as clean.

The adapter cannot see an ignore rule, a config file or an offline database (rows 2 and 3), so suppression is
excluded by the run definition of FR-044 (an empty configuration and no filter option), not by this adapter.
The adapter states that limit in its refusal and success output: the scan's database is `not recorded`.

## Finding shape

One finding per vulnerability per package.

| Finding field | Source in the report |
| --- | --- |
| rule id | `id` of the vulnerability, as written (`GHSA-`, `CVE-`, `GO-`, `PYSEC-` or any other OSV id). Aliases are not merged. |
| severity | `max_severity` of the `groups` entry whose `ids` holds the rule id, as written and unmapped (FR-034-CON-2); unset where no group names it. |
| message | `summary` of the vulnerability where present, else the rule id. |
| path | A valid `package.purl` of the package; where a package has no purl but has a non-empty `package.name` and `package.version`, `<package.name>@<package.version>`. |

OSV-Scanner reports no count of the rules it evaluated, so `rulesEvaluated` stays unset and the vacuity check
stays silent rather than guess. The ruleset is the database identity if the report exposes one (row 2: none is
documented in the reviewed report shape), otherwise unset; the evidence then states the database as `not recorded`.

## Data terms

An OSV record keeps the license of its advisory source. The adapter keeps the `id` of every finding, which
names the source by prefix, and records no advisory text beyond `summary`. A retained `FindingRecord` that is
redistributed carries attribution to the advisory sources of its findings (GitHub Security Advisories, PyPI
advisories and the Go vulnerability database are CC-BY-4.0; Ubuntu and Alpine advisories are CC-BY-SA; Wolfi
advisories are non-commercial, so no Wolfi-sourced record is retained where it can be redistributed). These
terms are as recorded in `ix://agent-ix/engineering-assurance/FR-044` and are not legal advice.

## Behavior

- The adapter SHALL be selected by `--adapter osv-scanner` and by `--tool osv-scanner`.
- The adapter SHALL read each vulnerability of each package of each source of `results` as one finding of the shape above.
- Where the report exposes a database identity, the adapter SHALL record it as the scan's ruleset.
- Where the report exposes no database identity, the adapter SHALL leave the ruleset unset.
- The adapter SHALL leave `rulesEvaluated` unset.
- If a vulnerability names no `id`, then the adapter SHALL refuse the document, because a dropped vulnerability could leave a component read as clean.
- For every package containing a vulnerability, the adapter SHALL require a package object.
- If `package.purl` is present for a package containing a vulnerability, then the adapter SHALL require that purl to be syntactically valid and use it as the finding path.
- If a package containing a vulnerability has no purl, then the adapter SHALL require non-empty package name and version and use `<name>@<version>` as the finding path.
- If a package containing a vulnerability has neither a valid purl nor a non-empty name and version, then the adapter SHALL refuse the entire document and write no `FindingRecord`.
- The adapter SHALL NOT record an ambiguous path or allow the affected component to appear clean.
- If the document is not a JSON object, has no `results` array, has an empty `results` array, or has a source with no non-empty `packages` array, then the adapter SHALL refuse it.
- If the document is refused, then `quoin evidence record` SHALL write no `FindingRecord` and exit non-zero with an error naming the reason.
- The adapter SHALL NOT run OSV-Scanner, read an SBOM, read a vulnerability database or open the network.
- The adapter SHALL NOT compute a score, class or ranking.
- The adapter SHALL NOT map a severity word.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-149-AC-1 | `quoin evidence record --adapter osv-scanner --results <file>` over a captured OSV-Scanner report with vulnerabilities writes a `FindingRecord` under `scans/`, one finding per vulnerability per package, with rule id, the group's own severity string and the package purl as path, and writes nothing under `runs/`. | Test |
| FR-149-AC-2 | A captured OSV-Scanner report whose packages all have no vulnerabilities is recorded as a scan with `findings: []`, distinguishable from no record. | Test |
| FR-149-AC-3 | A truncated file, a non-JSON file, a JSON object with no `results` array and a JSON object with an empty `results` array are each refused with a non-zero exit and no `FindingRecord`. | Test |
| FR-149-AC-4 | A source with no `packages` array, or an empty one, is refused with an error naming `packages`, and writes no `FindingRecord`. | Test |
| FR-149-AC-5 | A report with a vulnerability lacking an `id` is refused with an error naming the vulnerability and its package, and writes no `FindingRecord`. | Test |
| FR-149-AC-6 | `--tool osv-scanner` selects the adapter, and `--tool cargo-audit` and `--tool cargo-deny` still select theirs. | Test |
| FR-149-AC-7 | A package with no purl and non-empty name and version is recorded with path `<name>@<version>`; the adapter test asserts only this fallback path. FR-142-AC-26 tests the separate unmatched-join behavior. | Test |
| FR-149-AC-8 | A report that exposes no database identity is recorded with an unset ruleset and unset `rulesEvaluated`, and the vacuity check is silent. | Test |
| FR-149-AC-9 | Rust tests in the existing harness exercise the adapter boundary and assert that recording reads only the supplied input, invokes no subprocess and opens no network connection. | Test |
| FR-149-AC-10 | Rust adapter tests in the existing harness parse authentic, unedited OSV-Scanner reports captured through the FR-044 request over the npm, Python, Go and container OS-package fixtures defined by FR-044; malformed and truncated inputs are derived test fixtures and are not represented as captured scanner outputs. The tests assert parsing and package identity behavior without using tool results as compliance evidence. | Test |
| FR-149-AC-11 | When a captured report exposes no database identity, a Rust test asserts the retained finding record or success output states the database as `not recorded`. | Test |
| FR-149-AC-12 | Rust tests in the existing harness each supply a vulnerability with a missing package, an empty package name, an empty package version, or a present malformed purl and assert the entire report is refused with a package-identity error and no `FindingRecord` is written. | Test |
| FR-149-AC-13 | The Research status table states VERIFIED or UNVERIFIED for every documented claim, and does not mark a fixture-only path-recognition check as proof of advisory matching or report completeness. | Inspection |
## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md), [FR-034](./FR-034-finding-shaped-evidence.md); [engineering-assurance FR-044](ix://agent-ix/engineering-assurance/FR-044) produces the report.
- **Downstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-144](./FR-144-soup-report.md)
