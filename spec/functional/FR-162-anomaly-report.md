---
id: FR-162
title: "quoin anomaly report: the anomalies a release still carries, grouped by affected hazard and requirement"
type: FR
relationships:
  - target: "ix://agent-ix/spec-artifacts-process/US-003"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-136"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-process/FR-014"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-process/FR-015"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-process/FR-016"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-144"
    type: "references"
---

# FR-162: quoin anomaly report: the anomalies a release still carries, grouped by affected hazard and requirement

## Description

Tracked by Linear PLAT-1528 (specification) and PLAT-1585 (implementation). `quoin anomaly report` lists the
anomalies a release still carries, which the report calls **carried**: every `Anomaly` document that is not `closed`
(including one at `resolved`, whose fix has landed but which no one has closed), every `closed` one whose
`resolution` is `wont-fix` (a defect deliberately shipped unfixed), and every one whose safety impact is still
`unassessed`, with the severity, safety impact, status and affected requirements and hazards the
authors recorded. It groups them by affected hazard and by affected requirement, so that a safety reviewer sees each
hazard's open anomalies in one place. Its Markdown and JSON feed the dossier export. The content follows the
publicly available FDA premarket software guidance on unresolved anomalies, which asks for the anomalies still
open with their effect on safety; it reproduces no standard's text.

The report is a view, as the SOUP report of FR-144 is: it persists nothing, adds no record type and changes no
document. It reads the `Anomaly` records that `agent-ix/spec-artifacts-process` FR-014, FR-015 and FR-016 declare
and the derived values of FR-136, and defines no vocabulary of its own. The release rationale columns are FR-163;
the gate is FR-164; the rules that bind all three are NFR-035.

## Behavior

### Selection

- When a user runs `quoin anomaly report [<scope>]`, the command SHALL evaluate every `Anomaly` document in the scope, as `quoin anomaly check` does (FR-136).
- The command SHALL list every anomaly whose working-tree `status` is not `closed`.
- The command SHALL list every anomaly whose working-tree `status` is `closed` and whose `resolution` is `wont-fix`.
- The command SHALL list every anomaly whose `safety_impact` is `unassessed`, whatever its `status`.
- The command SHALL list each anomaly once.
- The command SHALL flag `unassessed` on every row whose `safety_impact` is `unassessed`.
- The command SHALL NOT list an anomaly that is `closed` with a `resolution` other than `wont-fix` and whose `safety_impact` is not `unassessed`.
- The command SHALL order the rows by anomaly id.

### Row

- Each row SHALL state the anomaly `id`, `title`, `path`, `status`, `resolution`, `severity` and `safetyImpact`, as authored.
- If a value is absent in the record, then its row SHALL state `not stated` (Markdown) or `null` (JSON) for it.
- The command SHALL NOT substitute a default or derive a value for `severity`, `resolution` or `safetyImpact`.
- Each row SHALL state its `assessment` entries as `{ kind, target, impact, rationale }`, in document order, as the record carries them, and `[]` for an anomaly with no assessment section.
- Each row SHALL state `affected`: the `target` of each `assessment` entry of `kind` `Requirement` or `Hazard`, with its `kind` and `impact`, in document order.
- The command SHALL NOT resolve an `affected` target against the corpus; a dangling or wrong-kind target is `quire validate`'s (FR-136) and the row states the id as authored.
- Each row SHALL state `releaseRationale` as FR-163 defines.
- Each row SHALL state `fixingChange` as FR-136 derives it and `rationaleChange` as FR-163 derives it, each in the encoding below.
- In the JSON, every row key above, `fixingChange` and `rationaleChange` included, SHALL be a key of the element of `anomalies`; none is nested in another key.

### Derived git values

`fixingChange` and `rationaleChange` are not record keys; each has four states, and the report encodes them alike.

- In the JSON, each SHALL be an object with the key `state`, one of `present`, `absent`, `pending` or `not-applicable`, and the keys `commit`, `author`, `authoredAt`, `signature` and `signatureCode`.
- When `state` is `present`, the five value keys SHALL carry the values FR-136 or FR-163 reports; in every other state they SHALL be `null`.
- `pending` SHALL mean absent because the qualifying change exists only in the working tree, and `absent` SHALL mean no qualifying change exists.
- In the Markdown, a `present` value SHALL read `<commit> <author> <authoredAt> <signature> (<signatureCode>)`, and the other states SHALL read `absent`, `absent (pending)` and `not applicable`.

### Grouping

- The command SHALL state `byHazard`: one group for each distinct `target` of an `assessment` entry of `kind` `Hazard` on a listed anomaly, ordered by target id.
- The command SHALL state `byRequirement`: one group for each distinct `target` of an `assessment` entry of `kind` `Requirement` on a listed anomaly, ordered by target id.
- Each group SHALL carry the `target` and one element `{ anomaly, impact }` for each listed anomaly that names it, ordered by anomaly id.
- A group for an acceptance-criterion id SHALL stand apart from the group of its requirement; the command SHALL NOT roll one up into the other.
- The command SHALL include an element whose `impact` is `Unaffected`, stating the `impact`.
- The command SHALL make no group for a `None` row.
- The command SHALL state `ungrouped`: each listed anomaly that is in no group, as `{ anomaly, reason }`, the `reason` being `unassessed` when `safety_impact` is `unassessed` and `no-affected-id` otherwise.
- The command SHALL state no group for a hazard or requirement that no listed anomaly names, and SHALL NOT state that such a target has no anomaly.

### Unreadable documents

- If an anomaly document in scope has a mapping error and so no record (FR-136 `anomaly.no-record`), then the command SHALL state it in `unreadable` as `{ path, code }`, the code being the mapping error's code, and SHALL still render every other row.
- If `unreadable` is not empty, then the command SHALL exit non-zero.
- If scope holds no `Anomaly` document, then the command SHALL render empty sections and exit zero.

### Output

- The command SHALL render Markdown by default and one JSON object with `--json`.
- The JSON object SHALL have exactly the keys `anomalies`, `byHazard`, `byRequirement`, `ungrouped` and `unreadable`, and `gate` when FR-164 applies.
- The Markdown SHALL hold, in this order, a rows table, a section for each of `byHazard`, `byRequirement`, `ungrouped` and `unreadable`, and the gate section when FR-164 applies; each section SHALL be present and state `none` when empty.
- The command SHALL write nothing into the repository.
- The command SHALL read only the `Anomaly` documents in scope and, for FR-136's derived values, the git history of those files.
- The command SHALL render the same bytes for the same working tree, history, options and git signature-verification configuration. `signature` and `signatureCode` are what git reports against the local keyring and allowed-signers configuration, so the claim excludes them across machines that differ in it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-162-AC-1 | Over a corpus with anomalies at `identified` (unassessed), `assessed`, `resolved`, `closed` with `fixed` and `closed` with `wont-fix`, the report lists all but the `closed` `fixed` one, once each, ordered by id; `closed` with `duplicate` or `not-a-defect` is not listed either. | Test |
| FR-162-AC-2 | A `closed` anomaly whose `safety_impact` is `unassessed` is listed and flagged `unassessed`; every row with `safety_impact` `unassessed` carries the flag and no other row does. | Test |
| FR-162-AC-3 | Each row carries `id`, `title`, `path`, `status`, `resolution`, `severity`, `safetyImpact`, `assessment`, `affected`, `releaseRationale`, `fixingChange` and `rationaleChange`, each a key of the JSON element of `anomalies`, with values as authored; an `identified` anomaly with no `severity` states `not stated` in Markdown and `null` in JSON, and no row carries a defaulted value. | Test |
| FR-162-AC-4 | An anomaly naming two hazards and one requirement appears in the `byHazard` group of each hazard and the `byRequirement` group of the requirement, with its `impact` in each, and appears once in `anomalies`. | Test |
| FR-162-AC-5 | A hazard named by an open anomaly and an anomaly closed as `fixed` has a group listing the open one only; a hazard no listed anomaly names has no group. | Test |
| FR-162-AC-6 | An anomaly naming `FR-001` and another naming `FR-001-AC-1` produce two groups, and neither group lists the other's anomaly. | Test |
| FR-162-AC-7 | A `Unaffected` row produces an element stating `Unaffected`; a `None` row produces no group; an `unassessed` anomaly and an anomaly whose only row is a `None` row are in `ungrouped` with the reasons `unassessed` and `no-affected-id`. | Test |
| FR-162-AC-8 | An anomaly with a `Requirement` row targeting `FR-999` is grouped under `FR-999` as authored and the command prints no dangling-reference diagnostic. | Test |
| FR-162-AC-9 | A document with a mapping error appears in `unreadable` with the mapping error's code, every other row still renders, and the exit status is non-zero; a scope with no `Anomaly` document renders every section as `none` and exits zero. | Test |
| FR-162-AC-10 | `--json` prints one object whose keys are exactly `anomalies`, `byHazard`, `byRequirement`, `ungrouped` and `unreadable`; the Markdown sections occur in the stated order, each present, with `none` for an empty one. | Test |
| FR-162-AC-11 | Two runs over the same working tree and history give byte-identical Markdown and byte-identical JSON, and no file in the repository changes or is created. | Test |
| FR-162-AC-12 | The documents the tests use are those shipped by `agent-ix/spec-artifacts-process` FR-014, FR-015 and FR-016, which include open and `closed` `wont-fix` anomalies that carry a release rationale; the commit sequences are authored by this repository's tests, and the quoin repository contains no copy of the documents. | Inspection |
| FR-162-AC-13 | The report prints a snapshot-tested Markdown and JSON over one fixture corpus holding every case above, and the snapshots contain no text of any standard. | Test |
| FR-162-AC-14 | For `fixingChange` and `rationaleChange`, the JSON holds `state` `present` with the five values, and `absent`, `pending` and `not-applicable` each with the five values `null`; the Markdown reads as the encoding above in each case. | Test |

## Dependencies

- **Record reader**: the `Anomaly` record on main is read by the temporary reader of FR-136's CR note (PLAT-1415 exception, `rust/crates/quoin-anomaly/src/record.rs`), which reads only `status`, `resolution`, `closed_by`, `safety_impact` and the assessment table. PLAT-1585 SHALL implement this report by widening that reader to read `severity`, `title` and `releaseRationale`, and SHALL NOT wait for the extractor `agent-ix/quire-rs#393`. The reader is deleted when that extractor lands, as FR-136 says. The `fixingChange` derivation of FR-136 is implemented in the same work if it is not already.
- **Upstream**: [FR-136](./FR-136-anomaly-record-mapping.md) (the record, `fixingChange`, the `anomaly.no-record` condition); `agent-ix/spec-artifacts-process` [FR-014](ix://agent-ix/spec-artifacts-process/FR-014), [FR-015](ix://agent-ix/spec-artifacts-process/FR-015), [FR-016](ix://agent-ix/spec-artifacts-process/FR-016).
- **Context only**: [FR-144](./FR-144-soup-report.md), whose report shape this follows; [FR-147](./FR-147-risk-advise-evidence-gaps.md), which lists open anomalies per risk estimate and is not changed by this requirement.
- **Downstream**: the dossier export of the Submission Dossier project, which reads the JSON.
