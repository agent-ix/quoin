---
id: FR-188
title: "quoin dossier generates, checks and exports the selected dossier documents"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-007"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
---

# FR-188: quoin dossier generates, checks and exports the selected dossier documents

## Description

Quoin SHALL provide `quoin dossier generate`, `quoin dossier check` and
`quoin dossier export`, which write, compare and export the selected dossier
documents from the repository's records. The documents are SRS (`srs`),
architecture (`architecture`), trace matrix (`trace-matrix`) and requirements
mapping matrix (`rmm`).

## Inputs

- `generate [--doc srs|architecture|trace-matrix|rmm]... [--out <DIR>] [--scope <DIR>] [--module <PATH>]... [--clause-binding <PATH>]... [--pack <PACK>] [--as-of <ISO-8601>] [--version <TEXT>] [--json]`.
- `check` takes the same selectors and writes nothing.
- `export --to pdf|docx --output <PATH> [--doc ...] [--release <TAG>] [--approval <PATH>]... [--template <PATH>] [--quire <PATH>] [--json]`.

## Outputs

- `generate`: one Markdown file per selected document at `<DIR>/<document-id>.md`. `<DIR>` is `dossier` unless `--out` names another. With no `--doc` all four documents are selected.
- `check`: per document, `matches` or `differs`.
- `export`: the file `quire export` writes.
- With `--json`: `{documents:[{id, kind, path, status, gaps:[{gap_kind, subject, detail}]}]}` where `status` is `complete` or `incomplete`. `check` adds `comparison` (`matches` or `differs`) to each document.

## Behavior

### Commit then export

- `generate` SHALL write Markdown into the repository and SHALL NOT call `quire export`. A person reviews and commits the files.
- `export` SHALL run `quire export` as an external process, as one bundle of the committed Markdown files, with `--bundle-title` and `--bundle-id` taken from this spec, `--source-rev` set to `HEAD`, and `--release`, `--approval` and `--template` passed through unchanged.
- The flow is commit-then-export because `quire export` takes identity and content from the blob committed at `--source-rev` and refuses a working file that differs from that commit (quire-cli FR-034 section A), and refuses identity options on JSON input.
- Quoin SHALL NOT export an uncommitted file, SHALL NOT render a document itself, and SHALL NOT bypass `quire export`.

### Selection and check

- A `--doc` value outside the four names SHALL exit 2 and write nothing.
- `check` SHALL regenerate each selected document in memory and compare its bytes with the file at its path, and SHALL write nothing.
- A missing file counts as `differs`.

### Document set

- The generated files SHALL carry the frontmatter of the `DossierDocument` archetype that spec-artifacts-process defines for this feature: `id`, `title`, `version`, `type: DossierDocument`, `document_kind`, `generated_by: quoin dossier` and, where the source needs it, `as_of`.
- `--version` is required by `generate` because `quire export` needs a Version or a Release (quire-cli FR-034).
- Each file SHALL end with the level-2 `Gaps` section below.

### Gaps section

- Each missing input SHALL be one bullet under the final level-2 heading `Gaps`, written `` `<gap_kind>` <subject>: <detail> ``. The section reads `Gaps: none` when empty.
- `gap_kind` is closed: `no_requirements`, `criticality_unassigned`, `no_design_element`, `dangling_design_reference`, `no_allocation`, `unbacked_criterion`, `no_evidence`, `no_clause_binding`, `clause_open`.
- A document with at least one gap has status `incomplete`. The same gaps appear in the `--json` report and in the Markdown, in the same order.
- Gaps are ordered by requirement id, then criterion id, then element id, then clause id.
- A gap SHALL be reported, never silently dropped or defaulted.

### Exit status

| Code | Meaning |
|------|---------|
| 0 | Every selected document is complete (and, for `check`, matches). |
| 1 | Documents were generated but at least one is incomplete, or `check` found a difference. |
| 2 | Usage error or invalid input. |
| 3 | A source command failed; the report names it. |
| 4 | `quire export` failed or could not start; its exit code and stderr are passed through, and a missing executable names the path. |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-188-AC-1 | `generate --doc srs --doc rmm` writes exactly two files and no other | Test |
| FR-188-AC-2 | `generate` with no `--doc` writes the four files at `dossier/<document-id>.md` | Test |
| FR-188-AC-3 | `generate --doc nope` exits 2 and writes no file | Test |
| FR-188-AC-4 | `check` on a committed file with one hand-edited line reports that document as `differs` and exits 1; with no edit it reports `matches` and exits 0 | Test |
| FR-188-AC-5 | `check` writes no file and a deleted file reports `differs` | Test |
| FR-188-AC-6 | `export --quire <missing path>` exits 4 and the report names that path | Test |
| FR-188-AC-7 | `export` over a generated file that is not committed surfaces `quire export`'s refusal with its exit code on stderr and leaves no output file | Test |
| FR-188-AC-8 | `export` invokes `quire export` once with every selected committed file, `--source-rev HEAD`, a bundle title and id, and `--approval`, `--template` and `--release` unchanged | Test |
| FR-188-AC-9 | A source command that fails makes `generate` exit 3 and the report names that command | Test |
| FR-188-AC-10 | `generate` without `--version` exits 2 and writes nothing | Test |
| FR-188-AC-11 | A fixture with two gaps of different kinds gives a `Gaps` section with two bullets in id order, the same two entries in `--json`, and `Gaps: none` for a complete document | Test |

## Dependencies

- **Upstream**: quire-cli FR-031 (`quire export`), FR-034, FR-038, FR-039.
- **Downstream**: [FR-189](./FR-189-dossier-srs-content.md), [FR-190](./FR-190-dossier-architecture-content.md), [FR-191](./FR-191-dossier-trace-matrix-content.md), [FR-192](./FR-192-dossier-rmm-content.md).
