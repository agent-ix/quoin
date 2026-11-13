---
id: FR-163
title: "Anomaly report release rationale: stated as authored, attributed by git, never written by the tool"
type: FR
relationships:
  - target: "ix://agent-ix/spec-artifacts-process/US-003"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-162"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-136"
    type: "depends_on"
  - target: "ix://agent-ix/spec-artifacts-process/FR-016"
    type: "depends_on"
---

# FR-163: Anomaly report release rationale: stated as authored, attributed by git, never written by the tool

## Description

Tracked by Linear PLAT-1528 and PLAT-1585. A release that carries an open anomaly needs a person's reason for
carrying it. That reason is the `## Release Rationale` section of the anomaly (`agent-ix/spec-artifacts-process`
FR-016), written by a named person. This requirement defines how `quoin anomaly report` (FR-162) shows it and how it
shows who wrote it. Acceptance of a risk is a human decision; quoin states what was written and offers no wording of
its own.

## Behavior

### As authored

- Each row of the report SHALL state `releaseRationale` as the entries `{ author, rationale }` of the record key `releaseRationale`, in document order, byte-equal to the record's values.
- If the anomaly has no `## Release Rationale` section, then the row SHALL state `releaseRationale` as `[]` (JSON) and `none` (Markdown).
- The command SHALL NOT edit the value of an entry, including its case. Whitespace around a table cell is padding the record extraction removes, and the report states the extracted value.
- The command SHALL NOT judge an entry, and SHALL NOT state it adequate, sufficient or acceptable.

### Never written

- Quoin SHALL provide no command, option or output that creates, edits, fills in, suggests or defaults a release rationale.
- The command SHALL NOT take a rationale `author` from `closed_by`, a commit author or the environment.
- The Markdown for an anomaly with no rationale SHALL NOT contain a template, a placeholder row or a suggested wording.

### Attribution by git

- Quoin SHALL derive `rationaleChange` for each row (FR-162 states its encoding and place), from the first-parent history of the repository's default branch for the file, following renames: the most recent commit that changed the text of the `## Release Rationale` section.
- Quoin SHALL report `rationaleChange` as that commit's `commit`, `author`, `authoredAt`, `signature` and `signatureCode`, with `signature` and `signatureCode` derived as FR-136 derives them.
- If the working tree has the section and its text differs from the section text at the tip of the default branch, or the section is absent at that tip, then Quoin SHALL report `rationaleChange` as `pending`.
- If the anomaly has no section in the working tree, then Quoin SHALL report `rationaleChange` as `absent`.
- If the file is not under git, then Quoin SHALL report `rationaleChange` as `not-applicable`.
- Quoin SHALL NOT take an author or date from a field typed into the document.
- Quoin SHALL NOT compare the typed `author` with the commit author and SHALL emit no diagnostic for a difference; both values are stated.
- Quoin SHALL emit no diagnostic for an absent, pending or not-applicable `rationaleChange`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-163-AC-1 | An open anomaly with a two-row `## Release Rationale` yields `releaseRationale` with both entries in document order, `author` and `rationale` byte-equal to the record's extracted values, with mixed case preserved. | Test |
| FR-163-AC-2 | An open anomaly with no section yields `[]` in JSON and `none` in Markdown, and the Markdown holds no template, placeholder row or suggested wording. | Test |
| FR-163-AC-3 | The help output of every `quoin anomaly` subcommand lists no command or option that creates, edits, fills in or suggests a release rationale, and running `quoin anomaly report` changes no byte of any file. | Test |
| FR-163-AC-4 | A `closed` `wont-fix` anomaly (a fixture the module ships under FR-016) whose `closed_by` and commit author differ from the rationale `author` yields an `author` equal to the typed value. | Test |
| FR-163-AC-5 | For a history that adds the section in one commit and edits it in a later one, `rationaleChange` names the later commit with its author, authored date, `signature` and `signatureCode`; for a history that edits only other parts of the file after adding the section, it names the commit that added the section. | Test |
| FR-163-AC-6 | A working tree whose section text differs from the default branch yields `rationaleChange` `pending`; a history that removed the section on the default branch, with a working tree that re-adds the old text, yields `pending`; an anomaly with no section yields `absent`; a file outside git yields `not-applicable`; none yields a diagnostic. | Test |
| FR-163-AC-7 | A document carrying a typed commit hash, author or date in its body or frontmatter yields a `rationaleChange` that contains none of them. | Test |
| FR-163-AC-8 | An entry whose typed `author` differs from the commit author yields both values and no diagnostic. | Test |
| FR-163-AC-9 | Signature letters map to `signed`, `unsigned` and `unknown` exactly as FR-136-AC-12 states, in `rationaleChange`. | Test |

## Dependencies

- **Upstream**: [FR-162](./FR-162-anomaly-report.md), [FR-136](./FR-136-anomaly-record-mapping.md); `agent-ix/spec-artifacts-process` [FR-016](ix://agent-ix/spec-artifacts-process/FR-016).
- **Downstream**: [FR-164](./FR-164-anomaly-report-gate.md).
