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
evidence references only.

## Rationale

A pack bundles several clause sets, and a project answers to all of them at once.
FR-135 renders one set per call and needs a hand-built binding report for each. The
pack report evaluates every set and reuses the FR-135 matrix unchanged, so the pack
view cannot disagree with the single-set view. Pack content is licensed, so the
report is built to be safe to share: it names clauses by id and never reproduces
what they say. The index carries row counts only; a count per compliance value would
behave like the score FR-135 forbids.

## Interface

```
quoin discharge --rmm --pack <name> --repo <repo> --as-of <instant>
                [--context KEY=VALUE]... [--json]
```

- `<name>` is the installed module name, as `quoin module list` shows it.
- `--context KEY=VALUE` (repeatable) supplies the context facts, parsed by the same
  rule as the single-set evaluation path (`quoin-quire` `Context::from_entries`).
  With none, the context is empty.
- `--repo` supplies the discharge facts (FR-133).
- `--binding` is required unless `--pack` is given.
- `--as-of` is required, as for FR-135.
- `--json` emits the JSON report; otherwise Markdown. Output goes to stdout only.

## Behavior

- The report SHALL cover every clause set the installed module declares.
- The report SHALL order sets by `authority/id@version`, compared byte-wise, and name each set by that key everywhere.
- The command SHALL obtain each set's binding by evaluating the set through the engine over the installed module with the `--context` facts.
- The command SHALL use the same context for every set.
- The command SHALL NOT read a binding report from a file or accept one from the caller.
- The command SHALL read the discharge facts for each set from the facts stored for that set's key in the `--repo` evidence store (FR-133).
- The command SHALL render a set with no stored facts as FR-135 renders a binding report with no facts.
- The command SHALL build and render each set's matrix with the FR-135 build and render.
- The pack report SHALL state no row, compliance, tailoring or escape rule of its own.
- The pack index SHALL be a table before the matrices with one entry per set, in report order, holding the set key and the number of rows of its matrix, and nothing else per set.
- The report SHALL carry no score, percentage, ratio, count per compliance value, overall verdict, or pass/fail line.
- The Markdown SHALL begin with a title naming the pack and `asOf`, then the context as sorted `KEY=VALUE` lines or `none`, then the index, then each set's FR-135 Markdown in index order.
- The Markdown SHALL pass the pack name, context and set keys through the FR-135 inline escape.
- The JSON SHALL carry schema version `requirements-mapping-pack-v1` with the members `schemaVersion`, `pack`, `asOf`, `context`, `index` (per set: `clauseSet`, `rowCount`) and `matrices` (the `requirements-mapping-v1` documents, in index order).
- The pack report type SHALL derive its JSON Schema under the same `schema` feature as `RequirementsMapping`.
- Two runs over the same inputs SHALL render byte-identical output, whatever the order of the stored facts.
- If `<name>` is not an installed module, then the command SHALL exit with status 2 naming the unknown name.
- If `<name>` is not an installed module, then the command SHALL write no output.
- If `<name>` is installed and FR-137 pack derivation reports it is not a pack, then the command SHALL exit with status 2 naming the module and stating it is not a pack.
- If `<name>` is installed and FR-137 pack derivation reports it is not a pack, then the command SHALL write no output.
- If a clause set of the pack fails to load, then the command SHALL exit with status 2 naming the set as `authority/id@version` where known and the load error.
- If a clause set of the pack fails to load, then the command SHALL emit no index and no matrix.
- If a stored disposition has an empty rationale, then the command SHALL refuse the whole report as FR-135-AC-6 does.
- If a stored disposition has an empty rationale, then the command SHALL emit no matrix for any set.
- If `--pack` is given without `--rmm` or without `--repo`, or together with `--binding`, `--facts` or `--inward`, then the command SHALL exit with status 1, the usage status of FR-135-AC-7.
- If `--pack` is given without `--rmm` or without `--repo`, or together with `--binding`, `--facts` or `--inward`, then the command SHALL print a message naming the conflict.
- If `--context` is given without `--pack`, then the command SHALL exit with status 1 and print a message naming the conflict.

## Constraints

- **Licensing boundary.** The report, its diagnostics and its errors SHALL NOT contain a clause's `text`, a clause's `title` or a clause set's `title`, which are the licensed fields of the clause-set schema. The report names a clause by its id and a set by its key.
- **No writes.** The command SHALL write nothing: not into the repository named by `--repo`, its evidence store, the current working directory's repository, or `IX_HOME`.
- The pack report SHALL NOT persist the context, the index or the matrices.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-140-AC-1 | Over a synthetic `example.invalid` pack of three clause sets installed in a temp `IX_HOME`, `--rmm --pack` emits an index with three entries and exactly three FR-135 matrices, in `authority/id@version` order whatever the manifest order. Each matrix is byte-equal to the single-set FR-135 render for the same binding and facts. | Test |
| FR-140-AC-2 | In a fixture where only one set's clauses use the context dimension `region`, changing `--context region=` from a value that binds a clause to one that does not changes that clause's row to `Not applicable` in that set's matrix and leaves every other set's matrix byte-identical. | Test |
| FR-140-AC-3 | With facts recorded in the repo's store for one set only, that set shows a `Compliant` row and the other sets show `Not compliant - undischarged` for every binding clause; each index `rowCount` equals the number of rows in that set's matrix. | Test |
| FR-140-AC-4 | The index, title and context lines the pack report itself writes, and the JSON members, are exactly those named under Behavior: no `%`, `score`, `ratio`, `verdict`, `pass` or `fail` in them, and no per-compliance-value count. Evidence reference strings are excluded from this check. | Test |
| FR-140-AC-5 | The fixture pack's clause `text`, clause `title` and set `title` each carry a unique sentinel. Searching the Markdown, the JSON, stderr and the output of every refusal in AC-7 to AC-9 for each sentinel finds none. A control run that prints a sentinel is caught by the same search, so the check can fail. | Test |
| FR-140-AC-6 | In a sandbox with two initialised git repositories (one the working directory, one as `--repo`), a snapshot of every file in the sandbox, including `IX_HOME` and the evidence store, is identical before and after a run, and `git status --porcelain` is unchanged in both. A control run that deliberately writes a file into a repository is detected by the same oracle. | Test |
| FR-140-AC-7 | An unknown name and an installed non-pack module (all sets `text: none` or `original`) each exit with status 2, with a message naming the name, and produce no stdout. | Test |
| FR-140-AC-8 | A pack with a clause-set file that fails to load because a sentinel-bearing `text` or `title` field is malformed exits with status 2, names the set as `authority/id@version` where known, and emits no index and no matrix even when other sets are sound. The sentinel is absent from stdout and stderr. | Test |
| FR-140-AC-9 | `--pack` with `--binding`, `--facts`, `--inward`, no `--repo`, or no `--rmm`, and `--context` without `--pack`, each exit with status 1 and a message naming the conflict; `--binding` omitted without `--pack` is refused as before. A stored disposition with a whitespace-only rationale refuses the whole report with the FR-135-AC-6 refusal. | Test |
| FR-140-AC-10 | Two runs from the same inputs with facts recorded in a different order produce byte-identical Markdown and JSON; and a clause id, pack name or context value containing a line break, a pipe, a backtick, a link or raw HTML renders as the literal text in the title, the index and the matrices. | Test |
| FR-140-AC-11 | The `--json` output of AC-1 validates against the JSON Schema generated from the pack report type, and carries `schemaVersion` `requirements-mapping-pack-v1` with each embedded matrix carrying `requirements-mapping-v1`. | Test |
| FR-140-AC-12 | The retained `quoin discharge` command help lists `--pack` and `--context`, and the help fixture matches the command. | Test |

## Dependencies

- [FR-135](./FR-135-requirements-mapping-matrix-view.md) supplies the matrix, its rows, its escape and the usage and facts refusals.
- [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md) supplies the stored facts per clause set key.
- [FR-137](./FR-137-detect-and-place-licensed-packs.md) supplies pack derivation.
- [FR-139](./FR-139-pack-writes-nothing-into-a-repository.md) supplies the write boundary this report keeps.
- Fixtures are original synthetic content with `example.invalid` authorities.
