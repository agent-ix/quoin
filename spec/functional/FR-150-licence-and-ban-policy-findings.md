---
id: FR-150
title: "Licence and ban policy findings from the ingested SBOM"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-033"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-034"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-041"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-142"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-144"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-038"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-039"
    type: "references"
---

# FR-150: Licence and ban policy findings from the ingested SBOM

## Description

`cargo-deny` gives Rust components licence and ban findings (FR-034). Components of the other ecosystems in the
Syft CycloneDX SBOM (npm, PyPI, Go module, container OS packages) get comparable findings from a person-authored
policy evaluated over the licences the SBOM already states.

**Data source.** The SBOM that `producer-execute` already produces with Syft
(`ix://agent-ix/engineering-assurance/FR-038`, `FR-039`) and that [FR-041](./FR-041-sbom-inventory-evidence.md)
already reads. Syft records the licences it detects in each CycloneDX component's `licenses` array. Nothing else is
read: no package registry, no licence database and no package file is consulted.

**No new producer.** The evaluation needs no execution path of its own. The SBOM is an existing retained input, the
policy is a text file, and the evaluation is a pure function of the two, so it is a finding-shaped adapter of the
FR-033 registry beside `grype` (FR-149): it runs no subprocess, opens no network connection and fetches no data. No
engineering-assurance change is made. The SBOM is read through the FR-041 reader, which is amended to expose each
component's declared licences (FR-041-AC-10); FR-150 contains no SBOM reader of its own (FR-150-CON-1).

**The policy is authored by a person and none is shipped.** It is a file of format `licence-policy-v1`:

```yaml
format: licence-policy-v1
allow: [MIT, Apache-2.0]      # SPDX licence identifiers (or exact expression strings) the project accepts
deny: [GPL-3.0-only]          # optional: identifiers the project rejects outright
ban:                          # optional: components the project does not use, by purl
  - purl: pkg:npm/left-pad    # no version bans every version; a version bans that exact version
    reason: "abandoned"       # optional free text, copied into the finding
```

`allow` is required and non-empty. quoin ships no policy, no default and no list of licences, and none is applied
when `--policy` is omitted. quoin does not decide which licences are acceptable; it reports where the SBOM departs
from what the person wrote.

### Evaluation

Each component the FR-041 reader returns is evaluated once. Licence identifiers are compared as written, ASCII
case-insensitively, with no other normalisation and no expression parsing. A ban entry matches a component by purl
equality as FR-142 defines it (type compared case-insensitively; namespace, name and version percent-decoded;
qualifiers and subpath ignored); an entry without a version matches every version.

| Component state | Finding | Rule id | Severity word |
|---|---|---|---|
| a declared licence id is in `deny` | yes | `licence-denied` | `denied` |
| a declared licence id is in neither `allow` nor `deny` | yes | `licence-not-allowed` | `not-allowed` |
| no licence declared | yes | `licence-unknown` | `unknown` |
| a licence is declared only as an expression or free-text name not in `allow` verbatim | yes | `licence-unknown` | `unknown` |
| every declared licence is in `allow` | no | | |
| purl matches a `ban` entry | yes, beside any licence finding | `package-banned` | `banned` |

A component declaring several licences is evaluated on each: any denied makes `licence-denied`, otherwise any not
in `allow` makes `licence-not-allowed`, and it has no licence finding only when all are in `allow`. The severity
word is the adapter's own and is not mapped to any other tool's vocabulary (FR-034-CON-2). The finding path is the
component purl; for a component with no purl it is `<name>@<version>` (or `<name>`), which the join lists as
unmatched with the reason `no purl` and which no ban entry can match.

`rulesEvaluated` is the number of components evaluated, so an SBOM listing nothing is vacuous under FR-034-AC-10.
The ruleset states the policy's `allow`, `deny` and `ban` entry counts.

**Unknown is never allowed.** A component without usable licence data is a finding, not an absence of one.

## Behavior

- The adapter SHALL be selected by `--adapter licence-policy` and by `--tool licence-policy`, with the SBOM as `--results <file>` and the policy as `--policy <file>`.
- The adapter SHALL record a `FindingRecord` of tool `licence-policy` with one finding per row of the table above that applies.
- If `--policy` is omitted, then `quoin evidence record` SHALL exit non-zero with a usage message and write no `FindingRecord`.
- If the policy is not valid YAML, lacks `format: licence-policy-v1`, has an absent or empty `allow`, contains an unknown key, or has a `ban` entry with no parseable purl, then the adapter SHALL refuse it, name the reason, and write no `FindingRecord`.
- If the SBOM is refused by the FR-041 reader, then the adapter SHALL refuse it and write no `FindingRecord`.
- A refused policy or SBOM SHALL NOT be recorded as a clean scan.
- The adapter SHALL record a component with no declared licence as `licence-unknown`.
- The adapter SHALL record a component whose licences are all expressions or names not listed verbatim in `allow` as `licence-unknown`.
- The adapter SHALL NOT parse a licence expression.
- The adapter SHALL set `rulesEvaluated` to the number of components evaluated.
- The adapter SHALL record, for each finding, the licence text or the ban `reason` in the message.
- The adapter SHALL NOT run a subprocess, read a file other than its two inputs, or open the network.
- The adapter SHALL NOT ship, embed or default a licence list.
- The adapter SHALL NOT reproduce the text of any licence or standard.
- The adapter SHALL NOT compute a score, class or ranking.
- The adapter SHALL NOT map a severity word.
- The adapter SHALL NOT write into a `soup_record`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-150-AC-1 | `quoin evidence record --adapter licence-policy --results <sbom> --policy <file>` over a Syft CycloneDX fixture holding an allowed, a denied, a not-listed, an unlicensed and a banned component writes one `FindingRecord` under `scans/` with findings `licence-denied`, `licence-not-allowed`, `licence-unknown` and `package-banned` on the matching purls, none for the allowed component, and writes nothing under `runs/`. | Test |
| FR-150-AC-2 | A component with an empty or absent `licenses` array is recorded `licence-unknown`, never omitted and never read as allowed. | Test |
| FR-150-AC-3 | A component whose licence is an expression (for example `MIT OR Apache-2.0`) is `licence-unknown` while the expression is not in `allow` verbatim, and has no licence finding when the exact string is in `allow`. | Test |
| FR-150-AC-4 | A component banned by a version-less entry is banned at every version; by an entry with a version, only at that version; and a banned component that also has a denied licence yields both findings. | Test |
| FR-150-AC-5 | An SBOM with all components allowed and unbanned is recorded as a scan with `findings: []`, distinguishable from no record; an SBOM listing no component is recorded vacuous, so no record is read clean from it. | Test |
| FR-150-AC-6 | Omitting `--policy` exits non-zero with a usage message and writes nothing. | Test |
| FR-150-AC-7 | Each of an unparseable policy, a policy without the format key, one with an absent or empty `allow`, one with an unknown key, and one with an unparseable ban purl is refused with a reason naming it, and writes no `FindingRecord`. | Test |
| FR-150-AC-8 | A document that is neither CycloneDX nor SPDX is refused as FR-041-AC-4 refuses it and writes no `FindingRecord`; an SPDX document is read through the same reader. | Test |
| FR-150-AC-9 | A component with no purl is recorded with the path `<name>@<version>` and no ban entry matches it. | Test |
| FR-150-AC-10 | The adapter source opens no file other than its two inputs, runs no subprocess, opens no network connection, and contains no licence identifier list, no default policy and no standard text. | Inspection |
| FR-150-AC-11 | The recorded findings carry the adapter's own severity words, and the recorded scan carries no score, class or rank. | Test |
| FR-150-AC-12 | The fixtures for AC-1 are Syft output captured from the engineering-assurance FR-039 fixtures and checked in unedited; a case no captured fixture holds (an unlicensed component) uses a minimal hand-written CycloneDX document, marked as such in its file. | Inspection |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-150-CON-1 | FR-150 SHALL NOT contain an SBOM reader of its own; the reading is the FR-041 reader's. | Design | Inspection |
| FR-150-CON-2 | quoin SHALL NOT decide which licences a project accepts; whether a finding blocks a release stays with the consumer's gate policy (FR-034-CON-3). | Design | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md), [FR-034](./FR-034-finding-shaped-evidence.md), [FR-041](./FR-041-sbom-inventory-evidence.md); `ix://agent-ix/engineering-assurance/FR-038` and `FR-039` produce the SBOM.
- **Downstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-144](./FR-144-soup-report.md)
