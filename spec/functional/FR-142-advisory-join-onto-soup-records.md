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
  - target: "ix://agent-ix/spec-objects-security/FR-011"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-012"
    type: "depends_on"
---

# FR-142: Join advisory findings onto SOUP records

## Description

`cargo-audit` and `cargo-deny` findings are already retained as `FindingRecord` entries through the adapters
of the FR-033 registry and FR-034. This requirement joins them onto `soup_record` entries at read time. The
`soup_record` schema admits no advisory list and no severity (`agent-ix/spec-objects-security` FR-012), so the
join is computed when a report is produced and is never written into a record. No new advisory parser is
added, and no severity is translated or scored (FR-034-CON-2).

The join is scan-only. The `vulnerability` object of spec-objects-security, whose `affects` edge may name a
`soup_record` (FR-012 there), stays authored context: the join does not read it. The `finding_refs` key of a
record is not read either, because a FR-034 finding has no id to resolve.

### What counts as an advisory

An advisory is a `cargo-audit` finding of any kind (a vulnerability, or a warning of kind `unmaintained`,
`unsound` or `yanked`), or a `cargo-deny` finding whose rule id begins `RUSTSEC-`, `GHSA-` or `CVE-`.
`cargo-deny` ban, licence, source and `unclassified` findings are not advisories; the join does not read
them, and the report states how many it set aside.

### Match rule

A finding carries no purl. Both adapters record the crate in `Finding.path` as `<name>@<version>`. That is
the primary match rule: the finding attaches to the record whose purl is `pkg:cargo/<name>@<version>`,
ignoring qualifiers and subpath. Where a finding's path is itself a purl, purl equality applies.

### Resolution

A record names one clause in `disposition_clause`, in the clause set of the `--binding` report. An advisory
on a record is resolved only when a current approved disposition fact under that clause was attested
at or after the newest retained scan, of either tool, that reports that advisory on that record. A later scan that reports a further
advisory therefore leaves that advisory unresolved until a person attests again.

## Behavior

- When a SOUP report is generated, the join SHALL read, for each of `cargo-audit` and `cargo-deny`, the retained `FindingRecord` of that tool with the newest `timestamp` (FR-034-CON-4).
- If no `FindingRecord` of a tool is retained, then the join SHALL report that tool's scan as not run.
- The join SHALL report a record free of a tool's advisories only when that tool's newest retained scan was read and is not vacuous, no advisory of that tool is attached to the record, and no unmatched finding of that tool could belong to it: an ambiguous finding whose candidates include it, a finding whose path names its crate without a version, or a finding with no path.
- The join SHALL read one scan per tool, the newest across all suites.
- The join SHALL state in a tool's scan state the suites of its older retained scans that it did not read.
- Where two retained scans of a tool have the same instant, the join SHALL read the one whose suite id sorts last.
- If a retained scan of a tool has a `timestamp` that is not an instant, then the join SHALL report that tool's scan as unordered.
- If a tool's scan is unordered, the join SHALL NOT read any scan of that tool.
- If a tool's scan is unordered, the join SHALL NOT report any record free of that tool's advisories.
- If a cargo-deny log was refused at recording by FR-034-AC-23 or FR-034-AC-25, then the join SHALL report cargo-deny as not run, because a refused log leaves no retained record.
- If the newest retained scan of a tool evaluated no rules, then the join SHALL report it as vacuous as FR-034-AC-10 does.
- If the newest retained scan of a tool is vacuous, the join SHALL NOT count it as evidence of no advisory.
- The join SHALL attach an advisory finding to the `soup_record` selected by the match rule.
- If a finding has no path, or its path carries no version, then the join SHALL list it as unmatched with the reason `no version`.
- If a finding matches no `soup_record`, then the join SHALL list it as unmatched with its rule id and package.
- If a finding matches more than one `soup_record`, then the join SHALL list it as unmatched with the reason `ambiguous`.
- The join SHALL count each finding once, attached to one record or listed once as unmatched, and a repeated finding with the same rule id and path in one tool's scan counts once; the same advisory reported by both tools is attached once per tool.
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
| FR-142-AC-11 | No new adapter or parser module for advisories is added; the join obtains scans through the FR-033 registry's FR-034 adapters. | Inspection |
| FR-142-AC-12 | A record accepted for advisory A, then a newer retained scan that also reports advisory B, shows both A and B unresolved with the reason `scan newer than attestation`; attesting again after that scan shows both resolved. | Test |
| FR-142-AC-13 | A cargo-deny licence finding, a ban finding and an `unclassified` finding attach to no record, appear in no unmatched list, and are counted as set aside. | Test |
| FR-142-AC-14 | A record carrying `finding_refs` and an authored `vulnerability` edge to it gets exactly the advisories its crate name and version select, and each finding is counted once. | Test |
| FR-142-AC-15 | With two retained cargo-audit scans in different suites, only the one with the newer `timestamp` is read, whatever the file names, and the scan state names the suite not read; with two at the same instant, the one whose suite id sorts last is read. | Test |
| FR-142-AC-16 | A finding whose path is `pkg:cargo/a@1.0.0?arch=x86#sub` attaches to the record with purl `pkg:cargo/a@1.0.0`. | Test |
| FR-142-AC-17 | With a retained cargo-audit scan whose `timestamp` is not an instant beside a well-formed one, the output states cargo-audit unordered, reads neither scan, and shows no record free of cargo-audit advisories. | Test |
| FR-142-AC-18 | The same advisory reported by cargo-audit at an earlier scan and by cargo-deny at a later one, on one record attested between them, is attached once per tool and both are unresolved with the reason `scan newer than attestation`. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-142-CON-1 | The join SHALL NOT write an advisory, severity or disposition into a `soup_record`. | Design | Test |
| FR-142-CON-2 | Whether an unresolved advisory blocks a release stays with the consumer's gate policy (FR-034-CON-3). | Design | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md), [FR-034](./FR-034-finding-shaped-evidence.md), [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md), [FR-141](./FR-141-promote-sbom-components-to-soup-records.md); `agent-ix/spec-objects-security` FR-011, FR-012.
- **Downstream**: [FR-144](./FR-144-soup-report.md)
