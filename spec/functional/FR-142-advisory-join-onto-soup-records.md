---
id: FR-142
title: "Join advisory findings onto SOUP records"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-034"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-133"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-141"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-149"
    type: "depends_on"
  - target: "ix://agent-ix/engineering-assurance/FR-044"
    type: "references"
  - target: "ix://agent-ix/spec-objects-security/FR-011"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-012"
    type: "depends_on"
---

# FR-142: Join advisory findings onto SOUP records

## Description

`cargo-audit`, `cargo-deny` and `grype` findings are already retained as `FindingRecord` entries through the adapters
of the FR-033 registry, FR-034 and [FR-149](./FR-149-grype-advisory-report-adapter.md). This requirement joins them onto `soup_record` entries at read time. The
`soup_record` schema admits no advisory list and no severity (`agent-ix/spec-objects-security` FR-012), so the
join is computed when a report is produced and is never written into a record. `grype` is the one scanner for the non-Cargo components of the Syft SBOM (npm, PyPI, Go module and
container OS packages), run through `producer-execute` by `ix://agent-ix/engineering-assurance/FR-044`. No new
advisory parser is added beyond the FR-149 adapter, and no severity is translated or scored (FR-034-CON-2).

The join is scan-only. The `vulnerability` object of spec-objects-security, whose `affects` edge may name a
`soup_record` (FR-012 there), stays authored context: the join does not read it. The `finding_refs` key of a
record is not read either, because a FR-034 finding has no id to resolve.

### What counts as an advisory

An advisory is a `grype` finding (every match is one), a `cargo-audit` finding of any kind (a vulnerability, or a warning of kind `unmaintained`,
`unsound` or `yanked`), or a `cargo-deny` finding whose rule id begins `RUSTSEC-`, `GHSA-` or `CVE-`.
`cargo-deny` ban, licence, source and `unclassified` findings are not advisories; the join does not read
them, and the report states how many it set aside.

### Coverage of the scanners

The ecosystems each tool assesses are a property of the tool, stated here and not configured:

| Tool | Purl types assessed |
|---|---|
| `cargo-audit`, `cargo-deny` | `cargo` |
| `grype` | `npm` (covering npm, pnpm and yarn lock files), `pypi`, `golang`, `deb`, `apk`, `rpm` |

Neither Syft nor Grype has a cataloger or advisory source for Lean lake manifests. A record whose purl type is
assessed by no tool (for example `pkg:lake/…` or `pkg:generic/…`), or is assessed only by tools whose scans
were not read, has its advisories **not assessed**: the report states the record as `unsupported` for the
purl type, and the join never reports it free of advisories. A grype finding whose purl matches a `cargo`
record still attaches to it.

### Match rule

A finding carries no purl. Both adapters record the crate in `Finding.path` as `<name>@<version>`. That is
the primary match rule: the finding attaches to the record whose purl is `pkg:cargo/<name>@<version>`,
ignoring qualifiers and subpath. Where a finding's path is itself a purl, as every grype finding's is, purl equality applies: the type, namespace,
name and version segments, percent-decoded, are equal, and the type is compared case-insensitively. Qualifiers and
subpath are ignored. No ecosystem-specific name normalisation is applied, so a finding whose purl differs from the
record's in spelling is listed as unmatched and is not dropped. A finding never attaches to a record of another
purl type, whatever the name.

### Resolution

A record names one clause in `disposition_clause`, in the clause set of the `--binding` report. An advisory
on a record is resolved only when a current approved disposition fact under that clause was attested
at or after the newest retained scan, of either tool, that reports that advisory on that record. A later scan that reports a further
advisory therefore leaves that advisory unresolved until a person attests again.

## Behavior

- When a SOUP report is generated, the join SHALL read, for each of `cargo-audit`, `cargo-deny` and `grype`, the retained `FindingRecord` of that tool with the newest `timestamp` (FR-034-CON-4).
- If no `FindingRecord` of a tool is retained, then the join SHALL report that tool's scan as not run.
- The join SHALL report a record free of a tool's advisories only when the record's purl type is assessed by that tool, that tool's newest retained scan was read and is not vacuous, no advisory of that tool is attached to the record, and no unmatched finding of that tool could belong to it: an ambiguous finding whose candidates include it, a finding whose path names its crate without a version, or a finding with no path.
- The join SHALL read one scan per tool, the newest across all suites.
- The join SHALL state in a tool's scan state the suites of its older retained scans that it did not read.
- Where two retained scans of a tool have the same instant, the join SHALL read the one whose suite id sorts last.
- If a retained scan of a tool has a `timestamp` that is not an instant, then the join SHALL report that tool's scan as unordered.
- If a tool's scan is unordered, the join SHALL NOT read any scan of that tool.
- If a tool's scan is unordered, the join SHALL NOT report any record free of that tool's advisories.
- If a cargo-deny log was refused at recording by FR-034-AC-23 or FR-034-AC-25, or a grype report was refused at recording by FR-149-AC-3 or FR-149-AC-4, then the join SHALL report that tool as not run, because a refused input leaves no retained record.
- If the result of the producer run that made a grype report was not `completed`, then no report exists to retain, and the join SHALL report grype as not run.
- If a record's purl type is assessed by no tool, then the join SHALL report the record's advisories as `unsupported`.
- If a record's purl type is assessed by no tool, the join SHALL NOT report the record free of advisories.
- If the newest retained scan of a tool evaluated no rules, then the join SHALL report it as vacuous as FR-034-AC-10 does.
- If the newest retained scan of a tool is vacuous, the join SHALL NOT count it as evidence of no advisory.
- The join SHALL attach an advisory finding to the `soup_record` selected by the match rule.
- If a finding has no path, or its path carries no version, then the join SHALL list it as unmatched with the reason `no version`.
- If a finding matches no `soup_record`, then the join SHALL list it as unmatched with its rule id and package.
- If a finding matches more than one `soup_record`, then the join SHALL list it as unmatched with the reason `ambiguous`.
- The join SHALL count each finding once, attached to one record or listed once as unmatched, and a repeated finding with the same rule id and path in one tool's scan counts once; the same advisory reported by two tools is attached once per tool.
- The join SHALL NOT read `finding_refs`.
- The join SHALL NOT read the `vulnerability` object.
- When an advisory is attached, the join SHALL look up the fact stored under the record's `disposition_clause` in the clause set of the `--binding` report, as FR-133 does.
- If the record has no `disposition_clause`, then the join SHALL list the advisory as unresolved with the reason `no disposition clause`.
- If the `disposition_clause` is not a clause of the binding report, or the clause does not bind, then the join SHALL list the advisory as unresolved with that reason.
- If no fact is stored under the clause, or the fact is not an approved disposition, or its attestation has expired or is not yet valid at `asOf`, then the join SHALL list the advisory as unresolved with the matching reason.
- If the fact's `attestedAt` is earlier than the `timestamp` of the newest retained scan, of either tool, that reports the advisory on that record, then the join SHALL list the advisory as unresolved with the reason `scan newer than attestation`.
- Where a current approved fact was attested at or after that scan, the join SHALL show the advisory resolved.
- The join SHALL carry the fact's `decision`, `rationale`, `approvalRef` and `attestedBy` unchanged.
- The join SHALL NOT compute a score, class or ranking of an advisory.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-142-AC-1 | Given a retained cargo-audit scan with an advisory whose path is `a@1.0.0` and a fixture record with purl `pkg:cargo/a@1.0.0`, the joined output attaches that advisory to that record and to no other. | Test |
| FR-142-AC-2 | The same holds for a retained cargo-deny scan whose advisory diagnostic names the crate, and for a cargo-audit warning of kind `unmaintained`. | Test |
| FR-142-AC-3 | An advisory whose package matches no record, one whose path has no version, and one matching two records each appear once in the unmatched list with rule id, package and reason, and attached + unmatched equals the distinct findings read. | Test |
| FR-142-AC-4 | With no retained scan of a tool, the output states that tool not run; with a retained clean scan it states a clean scan; the two outputs differ. | Test |
| FR-142-AC-5 | Recording a cargo-deny log whose summary counts more errors than its diagnostics is refused (FR-034-AC-25), and a join over the scans then retained shows cargo-deny not run and no record free of its advisories. | Test |
| FR-142-AC-6 | A scan that evaluated zero rules is reported vacuous, and no record is shown as free of advisories because of it. | Test |
| FR-142-AC-7 | A record with an attached advisory and a current approved disposition fact, attested at or after the scan, under its `disposition_clause` shows the advisory resolved with the fact's decision, rationale, approval reference and attester copied unchanged. | Test |
| FR-142-AC-8 | The same record with an expired fact, an absent fact, a fact that is not an approved disposition, no `disposition_clause`, or a clause the binding report does not contain or does not bind, shows the advisory unresolved with a reason naming which. | Test |
| FR-142-AC-9 | The joined output contains no score, severity-derived rank or class, and carries each scanner's own severity word unchanged. | Test |
| FR-142-AC-10 | After the join runs, every `soup_record` file in the fixture directory is byte-identical to before. | Test |
| FR-142-AC-11 | No advisory parser module is added by the join; it obtains scans through the FR-033 registry's finding adapters, including the FR-149 adapter. | Inspection |
| FR-142-AC-12 | A record accepted for advisory A, then a newer retained scan that also reports advisory B, shows both A and B unresolved with the reason `scan newer than attestation`; attesting again after that scan shows both resolved. | Test |
| FR-142-AC-13 | A cargo-deny licence finding, a ban finding and an `unclassified` finding attach to no record, appear in no unmatched list, and are counted as set aside. | Test |
| FR-142-AC-14 | A record carrying `finding_refs` and an authored `vulnerability` edge to it gets exactly the advisories its crate name and version select, and each finding is counted once. | Test |
| FR-142-AC-15 | With two retained cargo-audit scans in different suites, only the one with the newer `timestamp` is read, whatever the file names, and the scan state names the suite not read; with two at the same instant, the one whose suite id sorts last is read. | Test |
| FR-142-AC-16 | A finding whose path is `pkg:cargo/a@1.0.0?arch=x86#sub` attaches to the record with purl `pkg:cargo/a@1.0.0`. | Test |
| FR-142-AC-17 | With a retained cargo-audit scan whose `timestamp` is not an instant beside a well-formed one, the output states cargo-audit unordered, reads neither scan, and shows no record free of cargo-audit advisories. | Test |
| FR-142-AC-18 | The same advisory reported by cargo-audit at an earlier scan and by cargo-deny at a later one, on one record attested between them, is attached once per tool and both are unresolved with the reason `scan newer than attestation`. | Test |
| FR-142-AC-19 | Given retained grype scans over fixture SBOMs holding npm (including a scoped package), PyPI, Go module and container OS-package components, each fixture component with a known fixture advisory receives exactly the matches whose purl equals its own, and no component receives another's advisory, including a PyPI and an npm record of the same name. | Test |
| FR-142-AC-20 | A record of purl type `lake` or `generic` is reported `unsupported` and is not reported free of advisories, even when every retained scan is clean. | Test |
| FR-142-AC-21 | With no retained grype scan the output states grype not run; with a retained clean grype scan it states a clean scan for an npm record; the two differ. | Test |
| FR-142-AC-22 | A grype report refused at recording (FR-149-AC-3, FR-149-AC-4) and a producer result of state `failed` each leave grype not run and no record free of grype advisories. | Test |
| FR-142-AC-23 | A grype finding whose purl matches no record, and one whose purl differs from a record's only in spelling, each appear once in the unmatched list with rule id and package, and attached + unmatched equals the distinct findings read. | Test |
| FR-142-AC-24 | A grype advisory on a record with a current approved disposition attested at or after the grype scan shows resolved with the fact copied unchanged; attested earlier it shows unresolved with `scan newer than attestation`. | Test |
| FR-142-AC-25 | An npm record is shown free of grype advisories only from a read, non-vacuous grype scan, and is not shown free of `cargo-audit` advisories, which do not assess its purl type. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-142-CON-1 | The join SHALL NOT write an advisory, severity or disposition into a `soup_record`. | Design | Test |
| FR-142-CON-2 | Whether an unresolved advisory blocks a release stays with the consumer's gate policy (FR-034-CON-3). | Design | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md), [FR-034](./FR-034-finding-shaped-evidence.md), [FR-149](./FR-149-grype-advisory-report-adapter.md), [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md), [FR-141](./FR-141-promote-sbom-components-to-soup-records.md); `agent-ix/spec-objects-security` FR-011, FR-012.
- **Downstream**: [FR-144](./FR-144-soup-report.md)
