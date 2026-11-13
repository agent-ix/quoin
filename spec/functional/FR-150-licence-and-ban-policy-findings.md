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
    type: "depends_on"
  - target: "ix://agent-ix/quoin/NFR-030"
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
FR-033 registry beside `osv-scanner` (FR-149): it runs no subprocess, opens no network connection and fetches no data. No
new producer request is added. The one engineering-assurance change is a licence-bearing fixture for its tests
(`ix://agent-ix/engineering-assurance/FR-039-AC-8`), because the existing FR-039 fixtures give Syft no licence to
record. The SBOM is read through the FR-041 reader, which is amended to expose each component's declared licences
(FR-041-AC-10); FR-150 contains no SBOM reader of its own (FR-150-CON-1).

**The policy is authored by a person and none is shipped.** It is a file of format `licence-policy-v1`:

```yaml
format: licence-policy-v1
allow: [MIT, Apache-2.0]      # licence texts the project accepts: SPDX identifiers, names or exact expression strings
deny: [GPL-3.0-only]          # optional: licence texts the project rejects outright
ban:                          # optional: components the project does not use, by purl
  - purl: pkg:npm/left-pad    # no version bans every version; a version bans that exact version
    reason: "abandoned"       # optional free text
```

`allow` is required and non-empty. quoin ships no policy, no default and no list of licences, and none is applied
when `--policy` is omitted. quoin does not decide which licences a project accepts; it reports where the SBOM departs
from what the person wrote.

### Evaluation

Each component the FR-041 reader returns is evaluated once. The reader exposes each declared licence with a kind:
`id` (a CycloneDX `license.id`, or an SPDX value that is a single identifier with no whitespace, parenthesis or
operator), `name` (a CycloneDX `license.name`) or `expression` (a CycloneDX `expression`, or any other SPDX value).
The SPDX values `NOASSERTION` and `NONE` are no declared licence. Matching is of the declared text against `allow`
and `deny` entries, verbatim and ASCII case-insensitively, for all three kinds alike; no expression is parsed and no
other normalisation is applied. A ban entry matches a component by purl equality as FR-142 defines it (type compared
case-insensitively; namespace, name and version percent-decoded; qualifiers and subpath ignored); an entry without a
version matches every version.

Each declared licence is classed: **denied** if its text is in `deny`; else **accepted** if its text is in `allow`;
else **not allowed** if its kind is `id`; else **unknown** (a `name` or `expression` the person has not listed). A
component then receives **at most one licence finding**, by precedence denied, then not allowed, then unknown:

| Component state | Rule id | Severity word | Message |
|---|---|---|---|
| any declared licence denied | `licence-denied` | `denied` | the denied licence texts, joined with `, ` |
| else any `id` licence neither denied nor in `allow` | `licence-not-allowed` | `not-allowed` | the offending texts, joined with `, ` |
| else any `name` or `expression` not in `allow` verbatim, or no licence declared | `licence-unknown` | `unknown` | the unlisted texts joined with `, `, or `no licence declared` |
| every declared licence in `allow` | none | | |

A component whose purl matches one or more `ban` entries additionally receives one `package-banned` finding
(severity word `banned`) per matching entry, with message `banned by <entry purl>` followed by `: <reason>` where
the entry states one. The severity word is the adapter's own and is not mapped to any other tool's vocabulary
(FR-034-CON-2). The finding path is the component purl; for a component with no purl it is `<name>@<version>` (or
`<name>`), which the join lists as unmatched with the reason `no purl` and which no ban entry can match.

### What the scan records, so coverage is known

`rulesEvaluated` is the number of components evaluated, so an SBOM listing nothing is vacuous under FR-034-AC-10.
The scan also records an `evaluated` list: the FR-041 symbol (the purl, else `name@version`) of every component
evaluated. This is an added optional field of the FR-034 `FindingRecord` (FR-034-AC-27). Without it a scan stores
only counts, and the join could not tell a record the scan evaluated from one added after the scan or promoted
from another SBOM. The ruleset records the policy's `allow`, `deny` and `ban` entries themselves, so a scan state is
traceable to the policy that produced it.

**Unknown is never allowed.** A component without usable licence data is a finding, not an absence of one.

## Behavior

- When `--adapter licence-policy` or `--tool licence-policy` is given, `quoin evidence record` SHALL select the licence-policy adapter, with the SBOM as `--results <file>` and the policy as `--policy <file>`.
- The adapter SHALL record a `FindingRecord` of tool `licence-policy` holding at most one licence finding per component and one `package-banned` finding per matching ban entry.
- If `--policy` is omitted, then `quoin evidence record` SHALL exit non-zero with a usage message and write no `FindingRecord`.
- If `--policy` is given with an adapter other than licence-policy, then `quoin evidence record` SHALL refuse it with an error naming the adapter and record nothing.
- If the policy is not valid YAML, lacks `format: licence-policy-v1`, has an absent or empty `allow`, contains an unknown key, or has a `ban` entry with no parseable purl, then the adapter SHALL refuse it, name the reason, and write no `FindingRecord`.
- If the SBOM is refused by the FR-041 reader, then the adapter SHALL refuse it and write no `FindingRecord`.
- The adapter SHALL NOT record a clean scan for a refused policy or SBOM.
- The adapter SHALL record a component with no declared licence, or only `NOASSERTION` or `NONE`, as `licence-unknown`.
- The adapter SHALL record a component whose only non-accepted licences are names or expressions not in `allow` verbatim as `licence-unknown`.
- The adapter SHALL NOT parse a licence expression.
- The adapter SHALL set `rulesEvaluated` to the number of components evaluated.
- The adapter SHALL record the `evaluated` symbols of every component evaluated.
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
| FR-150-AC-2 | A CycloneDX component with an empty or absent `licenses` array, and an SPDX package declaring `NOASSERTION` or `NONE`, is recorded `licence-unknown` with message `no licence declared`, never omitted and never read as allowed; an SPDX package declaring `MIT` is classed as an `id`. | Test |
| FR-150-AC-3 | A component whose licence is an expression (for example `MIT OR Apache-2.0`) is `licence-unknown` while the expression is not in `allow` verbatim, and has no licence finding when the exact string is in `allow`; an expression or name equal to a `deny` entry is `licence-denied`. | Test |
| FR-150-AC-4 | A component banned by a version-less entry is banned at every version; by an entry with a version, only at that version; and a banned component that also has a denied licence yields both findings. | Test |
| FR-150-AC-5 | An SBOM with all components allowed and unbanned is recorded as a scan with `findings: []`, distinguishable from no record; an SBOM listing no component is recorded vacuous, so no record is read clean from it. | Test |
| FR-150-AC-6 | Omitting `--policy` exits non-zero with a usage message and writes nothing. | Test |
| FR-150-AC-7 | Each of an unparseable policy, a policy without the format key, one with an absent or empty `allow`, one with an unknown key, and one with an unparseable ban purl is refused with a reason naming it, and writes no `FindingRecord`. | Test |
| FR-150-AC-8 | A document that is neither CycloneDX nor SPDX is refused as FR-041-AC-4 refuses it and writes no `FindingRecord`; an SPDX document is read through the same reader. | Test |
| FR-150-AC-9 | A component with no purl is recorded with the path `<name>@<version>` and no ban entry matches it. | Test |
| FR-150-AC-10 | The adapter source opens no file other than its two inputs, runs no subprocess, opens no network connection, and contains no licence identifier list, no default policy and no standard text. | Inspection |
| FR-150-AC-11 | The recorded findings carry the adapter's own severity words, and the recorded scan carries no score, class or rank. | Test |
| FR-150-AC-12 | The fixtures for AC-1 are Syft output captured from the engineering-assurance FR-039 licence fixture (FR-039-AC-8: five entries, four with a licence) and checked in unedited, the test policy allowing one permissive id, denying the copyleft id, listing neither of the others and banning one purl; a case Syft cannot produce from it uses a minimal hand-written CycloneDX document, clearly marked as hand-written in its file. | Inspection |
| FR-150-AC-13 | `--policy` given with `--adapter sbom`, `osv-scanner` or `cargo-deny` is refused with an error naming the adapter and records nothing. | Test |
| FR-150-AC-14 | A component with one id in `allow` and one free-text name not in `allow` is `licence-unknown`; with one id in `allow` and one id in neither list, `licence-not-allowed`; with one id in `deny` and any other licence, `licence-denied`; each component has at most one licence finding. | Test |
| FR-150-AC-15 | The recorded scan lists, in `evaluated`, the symbol of every component of the SBOM, and its ruleset holds the policy's entries. | Test |
| FR-150-AC-16 | The message of a `licence-unknown` finding for an unlicensed component is `no licence declared`; of a `package-banned` finding is `banned by <entry purl>` with the entry's reason where it states one; of a licence finding the joined offending texts. | Test |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-150-CON-1 | FR-150 SHALL NOT contain an SBOM reader of its own; the reading is the FR-041 reader's. | Design | Inspection |
| FR-150-CON-2 | quoin SHALL NOT decide which licences a project accepts; whether a finding blocks a release stays with the consumer's gate policy (FR-034-CON-3). | Design | Inspection |

## Dependencies

- **Upstream**: [FR-033](./FR-033-evidence-format-adapters.md), [FR-034](./FR-034-finding-shaped-evidence.md), [FR-041](./FR-041-sbom-inventory-evidence.md); `ix://agent-ix/engineering-assurance/FR-038` and `FR-039` produce the SBOM.
- **Downstream**: [FR-142](./FR-142-advisory-join-onto-soup-records.md), [FR-144](./FR-144-soup-report.md)
