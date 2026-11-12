---
id: FR-140
title: "Pack-level requirements mapping report: one matrix per clause set and a pack index"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-135"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-137"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-139"
    type: "depends_on"
---

# FR-140: Pack-level requirements mapping report: one matrix per clause set and a pack index

## Description

`quoin discharge --rmm --pack <name>` SHALL report on an installed pack (FR-137):
one FR-135 requirements mapping matrix for every clause set of the pack, preceded by
a pack index. The report carries clause ids, force, applicability, compliance and
evidence references only. It SHALL NOT carry clause text or clause titles.

## Rationale

A pack bundles several clause sets, and a project answers to all of them at once.
FR-135 renders one set per call and needs a hand-built binding report for each. The
pack report evaluates every set and reuses the FR-135 matrix unchanged, so the pack
view cannot disagree with the single-set view. Pack content is licensed, so the
report is built to be safe to share: it names clauses by id and never reproduces
what they say.

## Interface

```
quoin discharge --rmm --pack <name> --repo <repo> --as-of <instant>
                [--context KEY=VALUE]... [--json] [--out <path>]
```

- `<name>` is the installed module name, as `quoin module list` shows it.
- `--context KEY=VALUE` (repeatable) supplies the context facts, parsed by the
  same rule as the single-set evaluation path (`quoin-quire` `Context::from_entries`).
  The same context is used for every set. With none, the context is empty.
- `--repo` supplies the discharge facts: for each set, the facts stored for that
  set's key in the repository's evidence store (FR-133). `--facts` is not accepted
  with `--pack`, because a facts array names clause ids only and ids can repeat
  across sets.
- `--binding` is not accepted with `--pack`; each set's binding is obtained by
  evaluating the set (below).
- `--as-of` is required, as for FR-135.
- `--json` emits the JSON report; otherwise Markdown.
- Output goes to stdout. With `--out <path>` it is written to exactly that path
  and stdout stays empty.

## Behavior

- **Sets.** The report covers every clause set the installed module declares, ordered
  by `authority/id@version`, byte-wise. A set is named by that key everywhere.
- **Binding per set.** Each set is evaluated through the engine over the installed
  module with the `--context` facts, producing a `clause-binding-v1` report. The
  report is not read from a file and not supplied by the caller.
- **Matrix per set.** The binding report and the stored facts for that set go
  through the FR-135 build and render. The pack report states no row, compliance or
  tailoring rule of its own; FR-135 governs every row, every unused fact and every
  escape. A set with no stored facts is rendered as FR-135 renders it.
- **Pack index.** A table before the matrices, one entry per set in report order, with
  the set key, the row count, and the number of rows at each Compliance value. Every
  Compliance value of FR-135 is listed for every set, including those with count 0.
  The counts are derived values of the report. The report carries no score,
  percentage, ratio, overall verdict, or pass/fail line.
- **Markdown layout.** A title naming the pack and `asOf`; the context echoed as
  sorted `KEY=VALUE` lines or `none`; the index; then each set's FR-135 Markdown in
  index order. Strings from the pack name, context and set keys go through the
  FR-135 inline escape.
- **JSON.** Schema version `requirements-mapping-pack-v1`: `schemaVersion`, `pack`,
  `asOf`, `context`, `index` (per set: `clauseSet`, `rowCount`, `counts` keyed by
  the FR-135 compliance values) and `matrices` (the `requirements-mapping-v1`
  documents, in index order). The type derives its JSON Schema under the same
  `schema` feature as `RequirementsMapping`, and `--json` output validates against
  it.
- **Unknown pack.** If `<name>` is not an installed module, then the command SHALL
  refuse with status 2, naming the unknown name, and write nothing.
- **Not a pack.** If `<name>` is installed and FR-137 pack derivation reports it is
  not a pack, then the command SHALL refuse with status 2, naming the module and
  stating it is not a pack, and write nothing. A module whose pack derivation fails
  to load is refused with its load error.
- **Load failure.** If the module's clause sets fail to load or any set fails to
  evaluate, then the command SHALL refuse with status 2, naming the set as
  `authority/id@version` where one is known and the load error, and SHALL emit no
  partial report and no index.
- **Bad facts.** A stored disposition with an empty rationale refuses the whole
  report as FR-135-AC-6 does; no matrix is emitted for any set.
- **Usage.** `--pack` without `--rmm`, without `--repo`, with `--binding`, with
  `--facts`, or with `--inward` is refused with the usage status of FR-135-AC-7, and
  the message names the conflict. `--out` without `--pack` is refused the same way.
- **Determinism.** Two runs over the same inputs render byte-identical output,
  whatever the order of the stored facts.

## Constraints

- **Licensing boundary.** The report, its diagnostics and its errors SHALL NOT
  contain the text or the title of any clause, or any other clause-set field that
  the engine treats as licensed text. A clause is named by its id.
- **No writes.** The command SHALL NOT write into the repository named by `--repo`,
  into the current working directory's repository, into the evidence store, or
  anywhere under `IX_HOME`. The one permitted write is the file named by `--out`,
  and only when `--out` is given. `--out` SHALL be refused when the path already
  exists, and SHALL be written only after the whole report has been built.
- The pack report SHALL NOT persist the context, the index or the matrices.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-140-AC-1 | Over a synthetic `example.invalid` pack of three clause sets installed in a temp `IX_HOME`, `--rmm --pack` emits an index with three entries and exactly three FR-135 matrices, in `authority/id@version` order whatever the manifest order. Each matrix is byte-equal to the single-set FR-135 render for the same binding and facts. | Test |
| FR-140-AC-2 | Each set's binding is produced by evaluating the set with `--context`: changing one context value so a clause stops binding changes that clause's row to `Not applicable` in that set's matrix and in no other set's, and the index counts change accordingly. | Test |
| FR-140-AC-3 | With facts recorded in the repo's store for one set only, that set shows `Compliant` rows and the other sets show `Not compliant - undischarged` for every binding clause; the index counts for every set equal the number of rows with that Compliance value in its matrix, and every Compliance value is listed for every set, zero included. | Test |
| FR-140-AC-4 | The Markdown and JSON outputs contain no score, percentage, ratio, or overall verdict: no `%`, no `score`, `ratio`, `verdict`, `pass` or `fail` member or line, and the JSON members are exactly those named under Behavior. | Test |
| FR-140-AC-5 | The fixture pack's clause texts and titles each carry a unique sentinel string. Searching the Markdown, the JSON, stderr and every refusal message of AC-7 to AC-9 for each sentinel finds none; a control run that prints a sentinel is caught by the same search, so the check can fail. | Test |
| FR-140-AC-6 | In a sandbox with two initialised git repositories (one the working directory, one as `--repo`) and a snapshot of every file outside `IX_HOME` taken before, a run without `--out` leaves `git status --porcelain` unchanged in both, the snapshot identical, and the evidence store and `IX_HOME` byte-identical. A control run that deliberately writes a file into the repository is detected by the same oracle. | Test |
| FR-140-AC-7 | An unknown name and an installed non-pack module (all sets `text: none` or `original`) each exit with status 2, with a message naming the name, and produce no stdout and no `--out` file. | Test |
| FR-140-AC-8 | A pack with a clause-set file that fails to load, and a pack whose set fails to evaluate, each exit with status 2, name the set as `authority/id@version` where known, and emit no index, no matrix and no `--out` file even when other sets are sound. | Test |
| FR-140-AC-9 | `--pack` with `--binding`, `--facts`, `--inward`, no `--repo`, or no `--rmm`, and `--out` without `--pack`, are each refused with the FR-135-AC-7 usage status and a message naming the conflict. A stored disposition with a whitespace-only rationale refuses the whole report with the FR-135-AC-6 refusal. | Test |
| FR-140-AC-10 | `--out <path>` writes exactly the bytes stdout would have carried to that path and leaves stdout empty; a second run with the same path is refused and leaves the file unchanged; with no `--out`, no file is created anywhere in the sandbox. | Test |
| FR-140-AC-11 | Two runs from the same inputs with facts recorded in a different order produce byte-identical Markdown and JSON; and a clause id, pack name or context value containing a line break, a pipe, a backtick, a link or raw HTML renders as the literal text in the title, the index and the matrices. | Test |
| FR-140-AC-12 | The `--json` output of AC-1 validates against the JSON Schema generated from the pack report type, and the output carries `schemaVersion` `requirements-mapping-pack-v1` with each embedded matrix carrying `requirements-mapping-v1`. | Test |
| FR-140-AC-13 | The retained `quoin discharge` command help lists `--pack`, `--context` and `--out`, and the help fixture matches the command. | Test |

## Dependencies

- [FR-135](./FR-135-requirements-mapping-matrix-view.md) supplies the matrix, its rows, its escape and the usage and facts refusals.
- [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md) supplies the stored facts per clause set key.
- [FR-137](./FR-137-detect-and-place-licensed-packs.md) supplies pack derivation.
- [FR-139](./FR-139-pack-writes-nothing-into-a-repository.md) supplies the write boundary this report keeps.
- Fixtures are original synthetic content with `example.invalid` authorities.
