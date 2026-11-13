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
  - target: "ix://agent-ix/quoin/FR-150"
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

`cargo-audit`, `cargo-deny` and `osv-scanner` findings are already retained as `FindingRecord` entries through the adapters
of the FR-033 registry, FR-034 and [FR-149](./FR-149-osv-scanner-advisory-report-adapter.md). This requirement joins them onto `soup_record` entries at read time. The
`soup_record` schema admits no advisory list and no severity (`agent-ix/spec-objects-security` FR-012), so the
join is computed when a report is produced and is never written into a record. `osv-scanner` (OSV-Scanner, behind the adapter boundary of FR-149, so a later scanner reuses this join) is the one scanner for the non-Cargo components of the Syft SBOM (npm, PyPI and Go module;
OS packages are not assessed until OSV-Scanner's handling of a Syft OS-package purl is verified, research row 7 of engineering-assurance FR-044), run through `producer-execute` by `ix://agent-ix/engineering-assurance/FR-044`. No new
advisory parser is added beyond the FR-149 adapter, and no severity is translated or scored (FR-034-CON-2).

The join is scan-only. The `vulnerability` object of spec-objects-security, whose `affects` edge may name a
`soup_record` (FR-012 there), stays authored context: the join does not read it. The `finding_refs` key of a
record is not read either, because a FR-034 finding has no id to resolve.

### What counts as an advisory

An advisory is a `osv-scanner` finding (every match is one), a `cargo-audit` finding of any kind (a vulnerability, or a warning of kind `unmaintained`,
`unsound` or `yanked`), or a `cargo-deny` finding whose rule id begins `RUSTSEC-`, `GHSA-` or `CVE-`.
`cargo-deny` ban, licence, source and `unclassified` findings are not advisories; the advisory join does not read
them. The ban and licence ones are *policy findings* (below); the source and `unclassified` ones are set aside, and
the report states how many it set aside.

### Policy findings

A policy finding is a licence or ban finding: a finding of the `licence-policy` tool ([FR-150](./FR-150-licence-and-ban-policy-findings.md), over
any purl type of the SBOM) or a `cargo-deny` finding whose code is one of the following. From the `licenses` check:
`rejected` and `unlicensed`. From the `bans` check: `banned` and `not-allowed`. Every other `cargo-deny` code (for
example `duplicate`, `wildcard`, `skipped`, `unmatched-skip`, `license-not-encountered`, or any code not named here) is
set aside and counted, as source and `unclassified` findings are. The set is a claim about cargo-deny 0.19.8 output;
the implementation SHALL capture, unedited, a licence diagnostic of each of `rejected` and `unlicensed` beside the
existing `banned` fixture, and a code that capture shows differently amends this list here before the join ships.
Policy findings use this requirement's rules unchanged: the newest retained scan per tool, the match rule, the
`unmatched` and `unordered` rules, vacuity, and the resolution rule below. A refused recording is treated as a refused osv-scanner report is: it retains nothing, so with no earlier scan the tool is not run and with an earlier scan
that scan is read, with its timestamp and ruleset stated. Policy findings are attached and counted apart from
advisories, and a policy finding is never counted as an advisory.

`licence-policy` assesses exactly the components in the `evaluated` list of its newest retained scan (FR-150), whatever
their purl type (so a `lake` or `generic` record is assessed here though `unsupported` for advisories); `cargo-deny`
assesses `cargo`. A record is stated **no policy findings in the scan read**, with the scan's timestamp and ruleset, only
when the newest `licence-policy` scan was read, is not vacuous or unordered, lists the record's purl in `evaluated`,
attached no policy finding to the record, and left no unmatched `licence-policy` finding that could belong to it (a
finding with no purl). A record not listed in `evaluated` is stated **not evaluated by the licence-policy scan read**,
never *no policy findings*. A record is never stated *allowed*. With no `licence-policy` scan retained, the output
states the tool not run. A `licence-unknown` finding is a finding: it is unresolved until a person's disposition resolves
it.

Resolution of a policy finding follows the resolution rule above, applied to that finding's own scan: a record's single
`disposition_clause` fact resolves a finding of the record, advisory or policy, only when it is current, approved and
attested at or after the `timestamp` of the newest scan reporting that finding. One fact therefore covers every
finding of the record that was reported by a scan at or before its attestation, and no finding reported by a later
scan. The report states this scope on the row (FR-144). The merged test tagged FR-142-AC-13 (`tc_1516_009`) asserts that
`rejected` and `banned` findings are set aside; the implementation slice rewrites that test and the `set_aside` count
together with the join, in the same change.

### Coverage of the scanners

The ecosystems each tool assesses are a property of the tool, stated here and not configured:

| Tool | Purl types assessed |
|---|---|
| `cargo-audit`, `cargo-deny` | `cargo` |
| `osv-scanner` | `npm` (covering npm, pnpm and yarn lock files), `pypi`, `golang` |

Neither Syft nor OSV-Scanner has a cataloger or advisory source for Lean lake manifests. A record whose purl type is
assessed by no tool in the table (for example `pkg:lake/…`, `pkg:generic/…` or an OS-package type such as `pkg:deb/…`, `pkg:apk/…` or `pkg:rpm/…`, which no tool assesses until verified) has its advisories **not
assessed**: the join states the record `unsupported` for its purl type, and never reports it free of advisories.
A record whose purl type a tool assesses, but whose scan was not read, is not `unsupported`; it is not free of that
tool's advisories, and the output names the tool's scan state (not run, unordered or vacuous).

A record is **free of advisories** only when every tool assessing its purl type meets the free-of rule below.
A record that is not free of advisories, has none attached and is not `unsupported` shows the state of each
assessing tool whose scan blocks it. A osv-scanner finding whose purl matches a `cargo`
record still attaches to it.

### Match rule

A finding carries no purl. Both adapters record the crate in `Finding.path` as `<name>@<version>`. That is
the primary match rule: the finding attaches to the record whose purl is `pkg:cargo/<name>@<version>`,
ignoring qualifiers and subpath. Where a finding's path is itself a purl, as an osv-scanner finding's is when the report carries `package.purl`, purl equality applies: the type, namespace,
name and version segments, percent-decoded, are equal, and the type is compared case-insensitively. Qualifiers and
subpath are ignored. No ecosystem-specific name normalisation is applied, so a finding whose purl differs from the
record's in spelling is listed as unmatched and is not dropped. A finding never attaches to a record of another
purl type, whatever the name. The `<name>@<version>` rule belongs to `cargo-audit` and `cargo-deny` findings only: a
`osv-scanner` finding whose path is not a purl (FR-149 records `<name>@<version>` for a match with no purl) is never matched
by it, and is listed as unmatched with the reason `no purl`.

### Resolution

A record names one clause in `disposition_clause`, in the clause set of the `--binding` report. An advisory
on a record is resolved only when a current approved disposition fact under that clause was attested
at or after the newest retained scan, of any tool, that reports that advisory on that record. A later scan that reports a further
advisory therefore leaves that advisory unresolved until a person attests again.

## Behavior

- When a SOUP report is generated, the join SHALL read, for each of `cargo-audit`, `cargo-deny` and `osv-scanner`, the retained `FindingRecord` of that tool with the newest `timestamp` (FR-034-CON-4).
- If no `FindingRecord` of a tool is retained, then the join SHALL report that tool's scan as not run.
- The join SHALL report a record free of a tool's advisories only when the record's purl type is assessed by that tool, that tool's newest retained scan was read and is not vacuous, no advisory of that tool is attached to the record, and no unmatched finding of that tool could belong to it: an ambiguous finding whose candidates include it, a finding whose path names its crate without a version, a finding with no path, or an osv-scanner finding with no purl (which could belong to any record of a purl type osv-scanner assesses).
- The join SHALL read one scan per tool, the newest across all suites.
- The join SHALL state in a tool's scan state the suites of its older retained scans that it did not read.
- Where two retained scans of a tool have the same instant, the join SHALL read the one whose suite id sorts last.
- If a retained scan of a tool has a `timestamp` that is not an instant, then the join SHALL report that tool's scan as unordered.
- If a tool's scan is unordered, the join SHALL NOT read any scan of that tool.
- If a tool's scan is unordered, the join SHALL NOT report any record free of that tool's advisories.
- If a cargo-deny log was refused at recording by FR-034-AC-23 or FR-034-AC-25, or a osv-scanner report was refused at recording by FR-149-AC-3, FR-149-AC-4 or FR-149-AC-5, then the join SHALL report that tool as not run, because a refused input leaves no retained record.
- The join SHALL observe only retained `FindingRecord` entries.
- The join SHALL NOT observe the producer run: a run whose result was not `completed` leaves no report to record, so nothing is retained from it.
- If an older osv-scanner scan is retained while a later producer run failed, then the join SHALL read the older scan as the newest.
- The join SHALL state, in the scan state of each tool read, the scan's `timestamp` and its ruleset (for osv-scanner, the database identity where the report exposes one, otherwise the statement `database not recorded`), so a stale scan is visible and an unknown database age stays unknown.
- If a record's purl type is assessed by no tool, then the join SHALL state the record `unsupported`.
- If a record's purl type is assessed by no tool, the join SHALL NOT report the record free of advisories.
- If a osv-scanner finding's path is not a purl, then the join SHALL list it as unmatched with the reason `no purl`.
- If the newest retained scan of a tool evaluated no rules, then the join SHALL report it as vacuous as FR-034-AC-10 does.
- If the newest retained scan of a tool is vacuous, the join SHALL NOT count it as evidence of no advisory.
- The join SHALL attach an advisory finding to the `soup_record` selected by the match rule.
- If a finding has no path, or its path carries no version, then the join SHALL list it as unmatched with the reason `no version`.
- If a finding matches no `soup_record`, then the join SHALL list it as unmatched with its rule id and package.
- If a finding matches more than one `soup_record`, then the join SHALL list it as unmatched with the reason `ambiguous`.
- The join SHALL count each finding once, attached to one record or listed once as unmatched.
- The join SHALL count a repeated finding with the same rule id and path in one tool's scan once.
- The join SHALL attach the same advisory reported by two tools once per tool.
- The join SHALL NOT read `finding_refs`.
- The join SHALL NOT read the `vulnerability` object.
- When an advisory is attached, the join SHALL look up the fact stored under the record's `disposition_clause` in the clause set of the `--binding` report, as FR-133 does.
- If the record has no `disposition_clause`, then the join SHALL list the advisory as unresolved with the reason `no disposition clause`.
- If the `disposition_clause` is not a clause of the binding report, or the clause does not bind, then the join SHALL list the advisory as unresolved with that reason.
- If no fact is stored under the clause, or the fact is not an approved disposition, or its attestation has expired or is not yet valid at `asOf`, then the join SHALL list the advisory as unresolved with the matching reason.
- If the fact's `attestedAt` is earlier than the `timestamp` of the newest retained scan, of any tool, that reports the advisory on that record, then the join SHALL list the advisory as unresolved with the reason `scan newer than attestation`.
- Where a current approved fact was attested at or after that scan, the join SHALL show the advisory resolved.
- The join SHALL carry the fact's `decision`, `rationale`, `approvalRef` and `attestedBy` unchanged.
- The join SHALL NOT compute a score, class or ranking of an advisory.
- The join SHALL read, for `licence-policy`, the retained `FindingRecord` with the newest `timestamp`.
- The join SHALL read a `cargo-deny` finding with code `rejected`, `unlicensed`, `banned` or `not-allowed` as a policy finding.
- The join SHALL set aside and count every other `cargo-deny` code.
- If no `FindingRecord` of `licence-policy` is retained, then the join SHALL report that tool's scan as not run.
- If no `FindingRecord` of `licence-policy` is retained, the join SHALL NOT report any record free of policy findings.
- If a `licence-policy` recording was refused and an earlier scan is retained, then the join SHALL read the earlier scan and state its timestamp and ruleset.
- If the newest `licence-policy` scan is vacuous or unordered, then the join SHALL NOT report any record free of policy findings.
- If a record's purl is not in the `evaluated` list of the `licence-policy` scan read, then the join SHALL state the record not evaluated.
- The join SHALL attach a policy finding to the `soup_record` selected by the match rule.
- The join SHALL list a policy finding that matches none, more than one, or has no purl or version as unmatched with the same reasons as an advisory.
- The join SHALL NOT drop a policy finding for lack of a matching record.
- The join SHALL resolve a policy finding by the disposition rule of an advisory, against the `timestamp` of the scan that reported that finding.
- The join SHALL carry the rule id, the tool's severity word and the message of a policy finding unchanged.
- The join SHALL NOT state any record `allowed`.

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
| FR-142-AC-13 | A cargo-deny source finding and an `unclassified` finding attach to no record, appear in no unmatched list, and are counted as set aside; a cargo-deny licence or ban finding is not an advisory, is not counted among them, and is joined as a policy finding (FR-142-AC-28). | Test |
| FR-142-AC-14 | A record carrying `finding_refs` and an authored `vulnerability` edge to it gets exactly the advisories its crate name and version select, and each finding is counted once. | Test |
| FR-142-AC-15 | With two retained cargo-audit scans in different suites, only the one with the newer `timestamp` is read, whatever the file names, and the scan state names the suite not read; with two at the same instant, the one whose suite id sorts last is read. | Test |
| FR-142-AC-16 | A finding whose path is `pkg:cargo/a@1.0.0?arch=x86#sub` attaches to the record with purl `pkg:cargo/a@1.0.0`. | Test |
| FR-142-AC-17 | With a retained cargo-audit scan whose `timestamp` is not an instant beside a well-formed one, the output states cargo-audit unordered, reads neither scan, and shows no record free of cargo-audit advisories. | Test |
| FR-142-AC-18 | The same advisory reported by cargo-audit at an earlier scan and by cargo-deny at a later one, on one record attested between them, is attached once per tool and both are unresolved with the reason `scan newer than attestation`. | Test |
| FR-142-AC-19 | Given retained osv-scanner scans over fixture SBOMs holding npm (including a scoped package), PyPI and Go module components (the Go module fixture is added to the engineering-assurance fixtures by the implementation of PLAT-1601, because the existing FR-039 fixtures have none), each fixture component with a known fixture advisory receives exactly the matches whose purl equals its own, and no component receives another's advisory, including a PyPI and an npm record of the same name. | Test |
| FR-142-AC-20 | A record of purl type `lake`, `generic` or `deb` is reported `unsupported` and is not reported free of advisories, even when every retained scan is clean. | Test |
| FR-142-AC-21 | With no retained osv-scanner scan the output states osv-scanner not run; with a retained clean osv-scanner scan it states a clean scan for an npm record; the two differ. | Test |
| FR-142-AC-22 | A osv-scanner report refused at recording (FR-149-AC-3 to FR-149-AC-5) leaves osv-scanner not run and no record free of osv-scanner advisories; with an older osv-scanner scan retained, the older scan is read and its timestamp and database identity (or `database not recorded`) are stated. | Test |
| FR-142-AC-23 | A osv-scanner finding whose purl matches no record, and one whose purl differs from a record's only in spelling, each appear once in the unmatched list with rule id and package, and attached + unmatched equals the distinct findings read. | Test |
| FR-142-AC-24 | A osv-scanner advisory on a record with a current approved disposition attested at or after the osv-scanner scan shows resolved with the fact copied unchanged; attested earlier it shows unresolved with `scan newer than attestation`. | Test |
| FR-142-AC-25 | An npm record is shown free of osv-scanner advisories only from a read, non-vacuous osv-scanner scan, and is not shown free of `cargo-audit` advisories, which do not assess its purl type. | Test |
| FR-142-AC-26 | A osv-scanner finding with no purl is listed unmatched with the reason `no purl` and attaches to no record, including a `pkg:cargo/` record of the same name and version, and no npm, PyPI or Go module record is shown free of osv-scanner advisories while it is read. | Test |
| FR-142-AC-27 | An npm record with a clean osv-scanner scan and a PyPI record whose purl type is also assessed are each free of advisories; a `lake` record is `unsupported` and a cargo record with no read cargo scan is not free of advisories. | Test |
| FR-142-AC-28 | A retained cargo-deny diagnostic of each of `rejected`, `unlicensed`, `banned` and `not-allowed` attaches to its `pkg:cargo/` record as a policy finding, is not counted as an advisory, and is resolved by the disposition rule; a `duplicate`, `wildcard`, `license-not-encountered` and `unmatched-skip` diagnostic are set aside and counted. The `rejected` and `unlicensed` fixtures are captured from cargo-deny unedited. | Test |
| FR-142-AC-29 | Given a retained `licence-policy` scan over a fixture SBOM holding an allowed, a denied, a not-listed, an unlicensed and a banned npm or PyPI component, each finding attaches to the record with its own purl and to no other, carrying its rule id, severity word and message unchanged, and the allowed component's record has none. | Test |
| FR-142-AC-30 | With no retained `licence-policy` scan the output states it not run and shows no record free of policy findings; with a retained clean scan listing the record it states `no policy findings in the scan read` with the timestamp and ruleset, and the two outputs differ. A policy or SBOM refused at recording (FR-150-AC-7) with no earlier scan leaves it not run, and with an earlier scan retained the earlier scan is read with its timestamp and ruleset stated. | Test |
| FR-142-AC-31 | A `licence-unknown` finding attaches to its record, is unresolved with no matching disposition, and is never stated allowed; with an approved disposition attested at or after the scan it shows resolved with the fact copied unchanged, attested earlier it shows `scan newer than attestation`. | Test |
| FR-142-AC-32 | A `licence-policy` finding whose purl matches no record, and one with no purl, each appear once in the unmatched list with rule id and package (the latter with reason `no purl`), and attached + unmatched equals the distinct findings read; a `lake` record is assessed by `licence-policy` yet reported `unsupported` for advisories. | Test |
| FR-142-AC-33 | A vacuous `licence-policy` scan (an SBOM that listed nothing) and an unordered one each show no record free of policy findings, each with its own stated reason. | Test |
| FR-142-AC-34 | The joined output carries a policy finding in a list separate from advisories, and no counter or field merges the two. | Test |
| FR-142-AC-35 | A record promoted after the `licence-policy` scan, or from another SBOM, is absent from `evaluated` and is stated not evaluated, never `no policy findings`. | Test |
| FR-142-AC-36 | A record with a disposition attested after an advisory scan but before a later `licence-policy` scan that reports a ban finding on it shows the advisory resolved and the ban finding unresolved with `scan newer than attestation`; attesting again after the later scan resolves both. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-142-CON-1 | The join SHALL NOT write an advisory, severity or disposition into a `soup_record`. | Design | Test |
| FR-142-CON-2 | Whether an unresolved advisory blocks a release stays with the consumer's gate policy (FR-034-CON-3). | Design | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md), [FR-034](./FR-034-finding-shaped-evidence.md), [FR-149](./FR-149-osv-scanner-advisory-report-adapter.md), [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md), [FR-141](./FR-141-promote-sbom-components-to-soup-records.md); `agent-ix/spec-objects-security` FR-011, FR-012.
- **Downstream**: [FR-144](./FR-144-soup-report.md)
