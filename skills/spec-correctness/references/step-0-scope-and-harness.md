# Step 0: Scope and Harness

**Goal**: know what you are classifying, what harness you are writing into, and what
tracking-tag form the repo already uses.

## 1. Resolve scope

Repo root = the directory holding the spec tree. Default glob `spec/**/*.md`; a multi-repo
layout uses `specs/<category>/<component>/spec/**/*.md`.

## 2. Run the classifier

```
quire properties --scope <repo> --json 'spec/**/*.md'
```

Requires quire-cli ≥ 0.12.0 (`quire --version`). Output shape:

```json
{"documents": [{
  "document": "<abs path>",
  "archetype": "FR",
  "criteria": [{
    "row_id": "FR-027-AC-1",
    "statement": "A stored organization is used ahead of the `origin` remote…",
    "line": 74,
    "shape": "assertion",
    "property": "universal",
    "extractable": true,
    "extraction": "extractable",
    "domain": null, "precondition": null, "oracle": null,
    "signals": ["universal:determiner"]
  }]
}]}
```

Field notes that matter:

- `row_id` — the only identifier shared with the matrix and with `gap-analysis`. Never
  synthesize one, never alter one. A record with `row_id: null` cannot be tagged, so it
  goes straight to the queue with reason `no-row-id`.
- `line` is 1-based and **file**-relative. Span `start`/`end` are **statement**-relative
  byte offsets into `statement`. Do not mix them.
- Spans are usually `null` (they reach ~4% of the ecosystem corpus). Their absence says
  nothing about the criterion; step 2 grounds the clauses regardless.
- `signals` is an audit trail, useful for diagnosis. `span:refused-weak-boundary` means the
  engine declined to guess a boundary — read it as "the oracle must come from the code."

Add `--pretty` while developing. Stderr carries module diagnostics (`DuplicateArchetype`
etc.); they are noise for this skill, not findings.

## 3. Detect the harness

Probe the repo root, in this order. Record harness, generator library presence, test dir,
and runner.

| Harness | Manifest | Generator library | Test path |
| --- | --- | --- | --- |
| Rust / proptest | `Cargo.toml` | `proptest`, `proptest-state-machine`, `loom`, `arbitrary` in `[dev-dependencies]` | `tests/props_fr_NNN.rs` (flat — cargo discovers only `tests/*.rs`) |
| TypeScript / fast-check | `package.json` | `fast-check` in `devDependencies`; runner from `vitest.config.*`, `jest.config.*`, or `package.json#scripts.test` | `tests/props/fr-NNN.prop.test.ts` |
| Python / hypothesis | `pyproject.toml`, `setup.cfg`, `requirements*.txt` | `hypothesis`; runner from `pytest.ini` / `tox.ini` / `conftest.py` | `tests/props/test_fr_NNN.py` |

Rules:

- **Manifest present, generator library absent** — install nothing, and **write no test
  files**. A file that imports a missing generator library breaks the suite even when every
  test in it is skipped, because the import is evaluated at collection time. Record the
  proposals as fenced code blocks in the queue instead, and report one line:
  `harness <lib> not installed — add <lib> to <dev-deps>, then re-run to emit these`.
  Adding a dependency is the user's call, not this skill's.
- **Polyglot repo** — resolve per-FR from the AC table's `Verification` column, which names
  the test file (`Test (org.test.ts)` → TypeScript). If that column is absent, use the tree
  holding the symbol step 2 grounded. Never pick one harness repo-wide by guessing.
- **No harness at all** — stop after steps 1 and 2. Emit the grounded strategies as prose
  in the queue file. Write no test files.

## 4. Detect the existing tag style

Grep the existing test tree for the four forms `gap-analysis` accepts
(`skills/gap-analysis/references/step-3-matrix-verification.md`):

```
Trace: FR-
FR-[0-9]{3}-AC-
TC-[0-9]{3}
Tests for FR-
```

Mimic the dominant form. The tags in step 4 are additive to it, never a replacement — if
this repo writes `// FR-025-AC-7:` line comments, keep writing those too.

## Output of this step

`{repo, glob, records[], harness, generator_lib_present, test_dir, runner, tag_style}`.
