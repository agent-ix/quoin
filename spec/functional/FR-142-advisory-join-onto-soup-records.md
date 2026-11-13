---
id: FR-142
title: "Join advisory findings onto SOUP records by purl"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
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

# FR-142: Join advisory findings onto SOUP records by purl

## Description

`cargo-audit` and `cargo-deny` findings are already retained as `FindingRecord` entries through the
FR-034 adapters. This requirement joins them onto `soup_record` entries at read time, by purl. The
`soup_record` schema admits no advisory list and no severity (`agent-ix/spec-objects-security` FR-012),
so the join is computed when a report is produced and is never written into a record. No new advisory
parser is added, and no severity is translated or scored (FR-034-CON-2).

## Behavior

- When a SOUP report is generated, the join SHALL read advisory findings from the retained scans of the FR-034 adapters.
- The join SHALL attach a finding to the `soup_record` whose purl equals the finding's package purl.
- If a finding names a package without a purl, then the join SHALL match on the crate name and version only where the record's purl is a `cargo` purl with that name and version.
- If a finding matches no `soup_record`, then the join SHALL list it as unmatched with its rule id and package.
- If a finding matches no `soup_record`, the join SHALL NOT drop it.
- If a scan has no retained record, then the join SHALL report the scan as not run.
- If a scan has no retained record, the join SHALL NOT report its absence as zero advisories.
- If a retained scan is a cargo-deny run that FR-034-AC-25 or FR-034-AC-23 would refuse, then the join SHALL refuse that scan.
- If a retained scan is a cargo-deny run that FR-034-AC-25 or FR-034-AC-23 would refuse, the join SHALL NOT read it as clean.
- If a retained scan evaluated no rules, then the join SHALL report it as vacuous as FR-034-AC-10 does.
- If a retained scan evaluated no rules, the join SHALL NOT count it as evidence of no advisory.
- Where a record carries `finding_refs`, the join SHALL also attach the findings those ids name.
- When a record has at least one attached advisory, the join SHALL treat the advisory as resolved only if the record has a `disposition_clause` and the FR-133 partition places that clause under an approved disposition at the report's `asOf`.
- If the record has no `disposition_clause`, or its fact is absent, expired or not yet valid, then the join SHALL list the advisory as unresolved with the reason.
- The join SHALL carry the disposition's `decision`, `rationale`, `approvalRef` and `attestedBy` from the stored fact unchanged.
- The join SHALL NOT synthesise a disposition.
- The join SHALL NOT compute a score, class or ranking of an advisory.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-142-AC-1 | Given a retained cargo-audit scan with an advisory on a package whose purl equals a fixture record's purl, the joined output attaches that advisory to that record and to no other. | Test |
| FR-142-AC-2 | The same holds for a retained cargo-deny scan whose advisory diagnostic names the crate. | Test |
| FR-142-AC-3 | An advisory whose package matches no record appears in the unmatched list with rule id and package, and the count of attached plus unmatched equals the findings read. | Test |
| FR-142-AC-4 | With no retained scan, the output states the scan was not run; with a retained clean scan it states a clean scan; the two outputs differ. | Test |
| FR-142-AC-5 | A cargo-deny log recorded with a summary counting more errors than its diagnostics is refused by the join and is not read as clean. | Test |
| FR-142-AC-6 | A scan that evaluated zero rules is reported vacuous, and no record is shown as free of advisories because of it. | Test |
| FR-142-AC-7 | A record with an attached advisory and a current approved disposition fact under its `disposition_clause` shows the advisory resolved with the fact's decision, rationale, approval reference and attester copied unchanged. | Test |
| FR-142-AC-8 | The same record with an expired fact, an absent fact or no `disposition_clause` shows the advisory unresolved with a reason naming which. | Test |
| FR-142-AC-9 | The joined output contains no score, severity-derived rank or class, and carries each scanner's own severity word unchanged. | Test |
| FR-142-AC-10 | After the join runs, every `soup_record` file in the fixture directory is byte-identical to before. | Test |
| FR-142-AC-11 | No new adapter or parser module for advisories is added; the join calls the FR-034 adapters. | Inspection |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-142-CON-1 | The join SHALL NOT write an advisory, severity or disposition into a `soup_record`. | Design | Test |
| FR-142-CON-2 | Whether an unresolved advisory blocks a release stays with the consumer's gate policy (FR-034-CON-3). | Design | Inspection |

## Dependencies

- **Upstream**: [FR-034](./FR-034-finding-shaped-evidence.md), [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md), [FR-141](./FR-141-promote-sbom-components-to-soup-records.md); `agent-ix/spec-objects-security` FR-011, FR-012.
- **Downstream**: [FR-144](./FR-144-soup-report.md)
