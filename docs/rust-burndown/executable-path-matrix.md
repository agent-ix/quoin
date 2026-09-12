# Executable-path disposition matrix — Quoin Rust burn-down

Satisfies **AC-3** of [quoin#373](https://github.com/agent-ix/quoin/issues/373).
Baseline for the **FP-NRX** metric (AC-7).

- **Measured at:** `quoin` commit `22db1d7`, submodule `corpus` at `qa-corpus@7b81343`.
  The first measurement in this document was taken at `e718d45`; every figure below
  is a re-measurement at `22db1d7` and says so where it matters.
- **Measured:** 2026-09-12. **Owner rulings applied:** 2026-09-12.
- **Governing policy:** [`quire-research/implementation-language-policy.md`](https://github.com/agent-ix/quire-research/blob/main/implementation-language-policy.md),
  Amendment 1 (2026-09-12).
- **Exception manifest:** [`quoin/.language-allowances.yaml`](../../.language-allowances.yaml).

## The rule, in one paragraph

**Every file in quoin that runs code should be Rust.** Applied literally that rule is
wrong — it would flag the lint config, the agent skills and the test fixtures, none of
which are engine logic and none of which anyone intends to convert. So the rule ships
with a named list of exceptions, held in `.language-allowances.yaml`. A file that runs
code and is not Rust is a **violation** unless it matches exactly one named exception,
or it is engine logic already scheduled for conversion. This document places every such
file in quoin into one of those three buckets and leaves none unplaced.

The policy numbers its exceptions 1–6. This document uses their names —
_user interface_, _generated_, _inert_, _thin host dispatch_, _staged-port retention_,
_dated owner disposition_ — because a number is not a fact a reader can check.

## Terms used here

Each of these exists only inside this program, so each is defined once, here.

| Term                             | Meaning                                                                                                                                                                                                                                                                   |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Violation**                    | Runs code, is not Rust, matches no exception, and has no conversion scheduled. The target is zero, continuously.                                                                                                                                                          |
| **Retained-with-successor**      | Engine logic that is still TypeScript _on purpose_, because its Rust replacement does not exist yet. Every such path names the ticket that will replace it and the stage it retires in. This is the debt the program burns down. Policy calls it _staged-port retention_. |
| **Allowed**                      | Matches a named exception. Not debt, not scheduled, not counted against the program.                                                                                                                                                                                      |
| **FP-NRX**                       | _First-Party Non-Rust eXecutable._ The program's metric, reported as three numbers: Violations / Retained-with-successor / Allowed. "First-party" means written by us, so vendored and third-party code is a different question.                                          |
| **Containment** vs **burn-down** | Two separate programs. Containment ([quire-research#56](https://github.com/agent-ix/quire-research/issues/56)) stops the spread and produces the inventory. Burn-down (#373) retires what the inventory found. Neither may report the other's work as its own.            |
| **Stage 0–9**                    | The ordered conversion plan in #373. Stage 0 is groundwork; Stage 9 retires the CLI shell. A path's stage is when its Rust replacement lands.                                                                                                                             |
| **Oracle**                       | Code that decides whether something passes or fails. The distinction matters because a script that _runs_ a check and a script that _is_ the check carry different risk.                                                                                                  |

## What the numbers count

**Unit:** physical lines (`wc -l`), including blank and comment lines. Not SLOC, not
statements. This is the unit #373's scope table used, so the two are comparable.

**Population:** files tracked by `git ls-files` in `agent-ix/quoin` whose extension is
one of `.ts .mjs .js .py .sh .tsp`. Excludes `node_modules/`, `dist/`, `coverage/`,
`.worktrees/` and untracked build output. Markdown, JSON, YAML and lockfiles are
outside the LOC population and appear only where a row needs them for reachability.

**Rows** are path globs, not files. Every glob is disjoint and their union is exactly
the population, so per-row counts reconcile to the total. A glob is the unit an
enforcement tool can re-measure; one row per file would be 344 rows that drift on the
next commit.

**Reachability**, one value per row:

| Value       | Meaning                                                                     |
| ----------- | --------------------------------------------------------------------------- |
| `shipped`   | Reachable from `bin/quoin.js`, the published npm entrypoint.                |
| `oracle`    | Decides a pass/fail result in `make test`, `make gate`, or CI.              |
| `gate-tool` | Runs inside a gate but decides nothing itself — builds, renders, refreshes. |
| `manual`    | Reachable only when a human types it.                                       |
| `dead`      | No caller anywhere in the repo, CI, or Makefile.                            |

## Correction to the #373 baseline

The scope table in #373 is a directory-level estimate. Three defects, all now measured:

| Defect                                          | Effect                                                                                                                                                                    |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `skills/` was never counted                     | **+23,164 lines** omitted — the largest non-Rust block in the repo                                                                                                        |
| `corpus/` was counted as quoin's                | `corpus/` is a **git submodule** (`agent-ix/qa-corpus`). Its 6,551 executable lines are another repo's tree. The estimate also undercounted it (4,996 vs 6,551 measured). |
| `bin/` and root tool configs were never counted | +302 lines                                                                                                                                                                |

Measured in-repo population at `e718d45`: **105,814 lines across 364 files** — 28%
larger than the estimate's in-repo part (82,417 after removing the submodule). `src/`
27,723, `tests/` 33,707, `scripts/` 13,485 and `smoke/` 509 reproduced the estimate
exactly; `evals/` measures 5,075 against 5,226 (−151, cause not identified at this
revision, likely a different commit); `templates/` measures 1,849 against 1,767 (+82,
the `.tsp` schema source the estimate omitted).

## FP-NRX — baseline

After the owner rulings of 2026-09-12 and the deletion of the one violation:

|                             |   Files |       Lines |
| --------------------------- | ------: | ----------: |
| **Violations**              |       0 |       **0** |
| **Retained-with-successor** |     338 | **101,833** |
| **Allowed**                 |       6 |     **415** |
| **Total population**        | **344** | **102,248** |

Nothing is unclassified — AC-3's "zero unclassified rows" is met.

Population is 102,248 rather than the 105,814 first measured at `e718d45`. Three
changes moved it, all in this branch: `scripts/storybook-deploy.js` deleted as the
sole violation (325 lines); the corpus-measurement subsystem disposed of under
[#388](https://github.com/agent-ix/quoin/issues/388) (3,200 lines across 20 files);
`scripts/check-trace-tags.mjs` added (134 lines); and 185 redundant `// Trace:`
lines merged away in the tag pass. The figures above are a fresh measurement at
this revision, not the earlier ones adjusted — reconciling by arithmetic would
have hidden the merge, which no ticket predicted.

**101,833 is the burn-down target** — the slope AC-7 measures. Violations = 0 is the
level AC-7 requires held from the day enforcement lands.

Reconciliation to the pre-ruling measurement:

|                       |   Files |       Lines |                                                         |
| --------------------- | ------: | ----------: | ------------------------------------------------------- |
| Pre-ruling Violations |       1 |         325 | → deleted, leaves the population                        |
| Pre-ruling Flagged    |      15 |      23,445 | → Allowed (skills 23,164 + tool configs 281)            |
| Pre-ruling Retained   |     346 |      81,992 | → 345 / 81,910 (`main.tsp`, 82 lines, moved to Allowed) |
|                       | **364** | **105,814** | → **363 / 105,489** at that revision                    |

Those are the figures as first measured at `e718d45`. They are **superseded** by
the table above, which is a fresh measurement at `22db1d7`. Two things moved
between them and neither is derivable from the other's deltas: the corpus-
measurement disposal under [#388](https://github.com/agent-ix/quoin/issues/388),
and the reclassification of `skills/**/workflow-assets/**` out of the manifest
and into a retained row — 23,164 lines that leave Allowed and enter Retained
because `specInvariants` determine an assertion and the `inert` test refuses
them.

Two non-file executable paths carry no LOC and are counted as paths only: the
Makefile's inline `python3` one-liner and the 35 inline CI `run:` steps.

## Matrix

Columns: **Path** · **Lang** · **Lines** · **Files** · **Reach** · **Owner capability**
· **Stage** · **Class** · **Successor / basis**.

### Violations

**None.** `scripts/storybook-deploy.js` (325 lines) was the only one and is deleted in
this change: dead k8s/Storybook deployment boilerplate, zero Storybook dependencies in
the repo (`grep -c storybook package.json pnpm-lock.yaml` → 0, 0), and no file,
Makefile target, package script or workflow named it. Debt to remove, not to port.

### Allowed

Each row names the exception it matches. Full definitions in
[`.language-allowances.yaml`](../../.language-allowances.yaml).

| Path                                                               | Lang     |   Lines | Files | Reach         | Exception              | Basis                                                                                                                                                                                                             |
| ------------------------------------------------------------------ | -------- | ------: | ----: | ------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vite.config.ts`, `cli-agent-evals.config.mjs`, `eslint.config.js` | TS/JS    |     281 |     3 | `gate-tool`   | **tool-config**        | Owner ruling 2026-09-12, as a general category rather than per-file dispositions. Build and lint tooling config is not engine logic; every repository has these files and the ruling is not repeated in each one. |
| `templates/.../typespec/main.tsp`                                  | TypeSpec |      82 |     1 | schema source | **schema-source**      | Owner ruling 2026-09-12. Schemas are data.                                                                                                                                                                        |
| `bin/quoin.js`                                                     | JS       |      21 |     1 | `shipped`     | **thin host dispatch** | 21 lines, zero branches, under the declared 40-line / 2-branch ceiling. Survives to Stage 9 as the Node shim, deleted with oclif.                                                                                 |
| `src/hooks/command-not-found.ts`                                   | TS       |      31 |     1 | `shipped`     | **thin host dispatch** | Declared in `package.json` `oclif.hooks`. Retiring it is a published-extension contract change (#373 _Known risks_). Same ceiling.                                                                                |
| **Total**                                                          |          | **415** | **6** |               |                        |                                                                                                                                                                                                                   |

### Out of scope — recorded so the boundary is explicit, not counted in the population

| Path                                                | Lines | Files | Ruling                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| --------------------------------------------------- | ----: | ----: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `corpus/**` (the whole `qa-corpus` submodule)       | 6,551 |   172 | **Owner ruling 2026-09-12: out of scope, closed.** It is a test corpus. Nothing in it is converted and no rows are opened against it — not by burn-down, not by containment. `bounds.py` (1,408), `verify.py` (964) and `corpus/scripts/*.py` (2,099) stay Python indefinitely, and `cases/**` (2,080 across 162 files: `.rs` 906 / `.ts` 613 / `.py` 525 / `.sh` 36) is inert sample input. Recorded here so a naive extension scan cannot count it as debt forever. See _Consequence for AC-5_ below. |
| `filament-core-data` `python_backend/`, `packages/` |     — |     — | Dated owner disposition. fcd is the schema source of truth and its gate tickets (#7 Phase A, #11 Phase B) block this program, but its tree is not quoin's and no quoin burn-down row may claim it. Consuming fcd's interface authorizes no change to fcd.                                                                                                                                                                                                                                               |

### Retained-with-successor

All rows are _staged-port retention_: still TypeScript on purpose, because the Rust
replacement does not exist yet.

**Successor convention:** each row names the ticket whose Rust replacement retires it.
Seven stage tickets exist and are linked inline below:

| Stage          | Ticket                                               | Covers                                                                         |
| -------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------ |
| 0              | [#375](https://github.com/agent-ix/quoin/issues/375) | workspace, boundary, CI at one revision, gates                                 |
| 0 (prereq)     | [#376](https://github.com/agent-ix/quoin/issues/376) | break the two TypeScript dependency cycles — must land before any crate exists |
| 1              | [#379](https://github.com/agent-ix/quoin/issues/379) | quire adapter collapse → `quoin-quire`                                         |
| 2              | [#377](https://github.com/agent-ix/quoin/issues/377) | validators — boundary proof-of-life                                            |
| 3              | [#378](https://github.com/agent-ix/quoin/issues/378) | semantic + completeness                                                        |
| 4 (foundation) | [#380](https://github.com/agent-ix/quoin/issues/380) | `quoin-store` — canonical JSON, JCS, blake3, atomic rename                     |
| 7              | [#381](https://github.com/agent-ix/quoin/issues/381) | config, plugins, modules                                                       |

Stage 5 and Stage 6 are decomposed, against the code rather than the directory listing:

| Stage | Ticket                                               | Crate                                                   | Lines |
| ----- | ---------------------------------------------------- | ------------------------------------------------------- | ----: |
| 5     | [#382](https://github.com/agent-ix/quoin/issues/382) | `quoin-combinatorial` — the auditor ↔ advisor cycle cut |   195 |
| 5     | [#383](https://github.com/agent-ix/quoin/issues/383) | `quoin-auditor` (auditor **+** advisor, one crate)      | 1,806 |
| 5     | [#384](https://github.com/agent-ix/quoin/issues/384) | `quoin-assurance`                                       | 1,723 |
| 5     | [#385](https://github.com/agent-ix/quoin/issues/385) | `quoin-graph-analysis`                                  | 1,373 |
| 6     | [#386](https://github.com/agent-ix/quoin/issues/386) | `quoin-measurement`                                     | 3,922 |
| 6     | [#387](https://github.com/agent-ix/quoin/issues/387) | `quoin-measurement-graph`                               | 1,812 |
| 6     | [#388](https://github.com/agent-ix/quoin/issues/388) | `measurement-run` — **liveness unconfirmed**            | 2,117 |
| 6     | [#389](https://github.com/agent-ix/quoin/issues/389) | consume EA for raw-evidence accounting — _not a port_   |     — |

The plan's crate topology was wrong in both stages, because it was written from a
directory listing. Stage 6's proposed five-way measurement split creates three
**value-level** Cargo cycles and omits 2,117 lines; Stage 5's auditor/advisor are one
crate, not two. Two hard facts fell out: `quoin-auditor` cannot compile until
[#376](https://github.com/agent-ix/quoin/issues/376) lands (`src/auditor/audit.ts:21` is a value import from `evidence`,
and `evidence ↔ change-assurance` is the repo's only genuine value cycle), and
`src/measurement/run.ts` has **no importer anywhere** — 2,117 lines that are fully
tested and unreachable from any shipped entrypoint. Reasoning and evidence on each
ticket.

**Stages 8 and 9 have no tickets yet.** Rows in those stages name `quoin#373 Stage N`
and the expiry is that stage's end.

#### Vendored skill runtime — `skills/**/workflow-assets/**` (23,164 lines, 12 files)

| Path                           | Lang  |  Lines | Files | Reach              | Owner capability                     | Stage | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| ------------------------------ | ----- | -----: | ----: | ------------------ | ------------------------------------ | ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `skills/**/workflow-assets/**` | JS/TS | 23,164 |    12 | `shipped` `oracle` | spec-review / matrix / to-plan flows | —     | **Successor [#374](https://github.com/agent-ix/quoin/issues/374). Expiry: when generator identity and source schema digest are recorded.** Not an `inert` manifest entry: `specInvariants` decide whether a review, matrix or plan flow passes, and the inert test is "determines no assertion". Not a `generated` entry either — yet. FR-099 states the rule: the bundle claims the generated allowance _only once_ its generator identity and source digest are recorded, and is **treated as first-party source until then**. So it is retained, and it leaves this table for the manifest's `generated` category the day #374 records provenance. The expiry is an event with a test attached rather than a guessed date, because NFR-024-AC-7 fails a hand edit of a covered file. Three byte-identical 7,525-line minified bundles (md5 `ea3d2c74…`) with `zod@4.4.3` inlined, shipped in `package.json` `files`, produced by no build step. |

#### Engine — `src/` (25,575 lines, 158 files, all `shipped`)

| Path                                                                                                     | Lang | Lines | Files | Reach              | Owner capability          | Stage                                                                                                                                                                    | Basis                                                                                                                                                                                                                                                                        |
| -------------------------------------------------------------------------------------------------------- | ---- | ----: | ----: | ------------------ | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/measurement/**`                                                                                     | TS   | 5,734 |    24 | `shipped` `oracle` | measurement model         | **6** ([#386](https://github.com/agent-ix/quoin/issues/386), [#387](https://github.com/agent-ix/quoin/issues/387), [#388](https://github.com/agent-ix/quoin/issues/388)) | Largest single block, last logic stage. `engine-run.ts`, `enumerate.ts`, `fixture-corpus.ts`, `modules.ts` all `execFileSync`.                                                                                                                                               |
| `src/evidence/**`                                                                                        | TS   | 3,958 |    18 | `shipped`          | evidence store            | **4** ([#380](https://github.com/agent-ix/quoin/issues/380))                                                                                                             | Quoin keeps ownership of the store (EA migration contract). The port relocates its language, not its ownership. Carries the digest/JCS surface — #373's hard gate.                                                                                                           |
| `src/commands/evidence/**`                                                                               | TS   | 1,173 |    10 | `shipped`          | evidence commands         | **8**                                                                                                                                                                    | `affirm.ts`, `audit.ts`, `baseline.ts` `execFileSync` out.                                                                                                                                                                                                                   |
| `src/commands/*.ts`                                                                                      | TS   | 1,126 |    11 | `shipped`          | command shell             | **8**                                                                                                                                                                    | `assurance.ts` `execFileSync`s.                                                                                                                                                                                                                                              |
| `src/commands/change-assurance/**`                                                                       | TS   |   796 |     9 | `shipped`          | change assurance          | **4** ([#380](https://github.com/agent-ix/quoin/issues/380))                                                                                                             |                                                                                                                                                                                                                                                                              |
| `src/commands/{catalog,measurement,graph,module,semantic,config,plugin}/**`                              | TS   |   796 |    31 | `shipped`          | command shell             | **8**                                                                                                                                                                    | catalog 179/5 · measurement 139/4 · graph 135/5 · module 99/5 · semantic 90/1 · config 88/5 · plugin 66/6. `semantic/sweep.ts` `execFileSync`s.                                                                                                                              |
| `src/change-assurance/**`                                                                                | TS   | 2,023 |     8 | `shipped`          | change assurance          | **4** ([#380](https://github.com/agent-ix/quoin/issues/380))                                                                                                             | **Cycle A:** `store.ts:22` imports `storeRoot` from `../evidence/store.js` while `evidence/index.ts:142` re-exports from `../change-assurance/schema-assets.js`. Cargo forbids this; broken by [#376](https://github.com/agent-ix/quoin/issues/376) before any crate exists. |
| `src/assurance/**`                                                                                       | TS   | 1,723 |     5 | `shipped`          | assurance records         | **5** ([#384](https://github.com/agent-ix/quoin/issues/384))                                                                                                             |                                                                                                                                                                                                                                                                              |
| `src/graph-analysis/**`                                                                                  | TS   | 1,373 |     5 | `shipped`          | graph views               | **5** ([#385](https://github.com/agent-ix/quoin/issues/385))                                                                                                             |                                                                                                                                                                                                                                                                              |
| `src/*.ts` (`base cli catalog config-schema flow-command flows index modules org plugins version write`) | TS   | 1,358 |    12 | `shipped`          | CLI core, config, modules | **7** ([#381](https://github.com/agent-ix/quoin/issues/381)), **9**                                                                                                      | `flows.ts` `spawn`s `ix-flow` — a transitive call out to a Node tool. `config-schema.ts` is 56 lines of zod, a verified non-risk.                                                                                                                                            |
| `src/quire/**`                                                                                           | TS   | 1,342 |     6 | `shipped` `oracle` | Quire adapter             | **1** ([#379](https://github.com/agent-ix/quoin/issues/379)), **8**                                                                                                      | `exec.ts` is the boundary template #373 adopts; it is the **last** file deleted (Stage 8).                                                                                                                                                                                   |
| `src/semantic/**`                                                                                        | TS   | 1,323 |     6 | `shipped`          | semantic modules          | **3** ([#378](https://github.com/agent-ix/quoin/issues/378))                                                                                                             | `manifest.ts` `mapAjvError` — the ajv ↔ `jsonschema` parity risk lands here.                                                                                                                                                                                                 |
| `src/auditor/**`                                                                                         | TS   | 1,139 |     3 | `shipped`          | auditor                   | **5** ([#383](https://github.com/agent-ix/quoin/issues/383))                                                                                                             | **Cycle B:** `auditor` ↔ `advisor`. Broken by [#376](https://github.com/agent-ix/quoin/issues/376).                                                                                                                                                                          |
| `src/advisor/**`                                                                                         | TS   |   862 |     3 | `shipped`          | advisor                   | **5** ([#383](https://github.com/agent-ix/quoin/issues/383))                                                                                                             | Cycle B, other half.                                                                                                                                                                                                                                                         |
| `src/completeness/**`                                                                                    | TS   |   685 |     5 | `shipped`          | completeness              | **3** ([#378](https://github.com/agent-ix/quoin/issues/378))                                                                                                             |                                                                                                                                                                                                                                                                              |
| `src/validators/**`                                                                                      | TS   |   164 |     2 | `shipped`          | validators                | **2** ([#377](https://github.com/agent-ix/quoin/issues/377))                                                                                                             | Smallest engine module — #373's boundary proof-of-life.                                                                                                                                                                                                                      |

#### Tests — `tests/` (32,449 lines, 100 files, all `oracle`)

| Path                                                | Lang |  Lines | Files | Reach            | Owner capability | Stage            | Basis                                                                                                                                                                                                                                                                      |
| --------------------------------------------------- | ---- | -----: | ----: | ---------------- | ---------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tests/**/*.test.ts`                                | TS   | 30,820 |    89 | `oracle`         | per-domain       | follows its code | Not debt while they run: they are the parity oracle for the TypeScript they cover. Each retires **in the same commit** as its code, with its criteria restated on a tracking-tagged Rust test (AC-4/AC-5). 33 of them `execFileSync`/`spawnSync`.                          |
| `tests/props/*.prop.test.ts`                        | TS   |  1,157 |     7 | `oracle`         | property tests   | follows its code | Header: _"Property tests generated by the `spec-correctness` skill."_ **Not the `generated` exception** — it names a generator identity but records no source digest, and the files are hand-maintainable. Governed as ordinary source. `fr-005.prop.test.ts` `execSync`s. |
| `tests/{support,fixtures,setup.ts,global-setup.ts}` | TS   |    472 |     4 | `oracle` harness | test harness     | follows its code | `support/semantic-module-template.ts` `execFileSync`s the cookiecutter.                                                                                                                                                                                                    |

#### Qualification scripts — `scripts/` (13,294 lines, 38 files)

This is where **AC-9** bites: most of this block is capability `engineering-assurance`
already owns in Rust. Marked **[EA]**.

| Path                                                                                               | Lang | Lines | Files | Reach             | Owner capability                                         | Stage                                                        | Basis                                                                                                                                                                                                                                                                                                                                               |
| -------------------------------------------------------------------------------------------------- | ---- | ----: | ----: | ----------------- | -------------------------------------------------------- | ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `scripts/verification-stack.mjs` + `-selftest.mjs`                                                 | mjs  | 1,515 |     2 | `oracle`          | **[EA]** bounded producer execution, process supervision | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | `make test` — _the_ local green bar. Runs `python3 corpus/bounds.py --json` and asserts toolchain identity. EA owns this as `producer_execution.rs` + `process_host.rs`. Consume, do not re-grow in Rust.                                                                                                                                           |
| `scripts/verification-declarations.mjs` + `-selftest.mjs`                                          | mjs  |   731 |     2 | `oracle`          | **[EA]** producer execution                              | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | Gate leg of `make validate`.                                                                                                                                                                                                                                                                                                                        |
| `scripts/verification-relock.mjs` + `-selftest.mjs` + `verification-object-integrity-selftest.mjs` | mjs  | 1,108 |     3 | `manual` `oracle` | **[EA]** evidence reporting                              | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | `make verification-relock`. Runs `python3 -I corpus/bounds.py --json`.                                                                                                                                                                                                                                                                              |
| `scripts/lib/tier1-*.mjs` (`execution comparison corpus render measurement recall scoring`)        | mjs  | 2,569 |     7 | `oracle`          | **[EA]** corpus accounting, evaluation reporting         | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | EA owns `compatibility_corpus.rs` + `evaluation_reports.rs`.                                                                                                                                                                                                                                                                                        |
| `scripts/bench-tier1.mjs`                                                                          | mjs  |   921 |     1 | `oracle`          | **[EA]** evaluation reporting                            | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | Runs `python3 corpus/bounds.py`. Asserts the attestation pins node, rust **and python** toolchains — the Python dependency is load-bearing and declared.                                                                                                                                                                                            |
| `scripts/battletest.mjs` + `lib/tier2-baseline.mjs`                                                | mjs  | 1,356 |     2 | `manual` `oracle` | **[EA]** corpus accounting                               | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | `make battletest`, needs an external pinned tree.                                                                                                                                                                                                                                                                                                   |
| `scripts/check-tool-drift.mjs` + `-selftest.mjs`                                                   | mjs  |   797 |     2 | `oracle`          | **[EA]** package audit                                   | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | `make audit-tool-drift`.                                                                                                                                                                                                                                                                                                                            |
| `scripts/lib/semantic-module-type-fit.mjs` + `semantic-module-type-fit.mjs`                        | mjs  | 1,810 |     2 | `oracle`          | semantic modules — **domain-specific**                   | **3** ([#378](https://github.com/agent-ix/quoin/issues/378)) | Three-part test answered: must it be local — **yes**, it is Quoin's own module type system; is it Rust — **no, port at Stage 3**; should it be common — **no**, recorded reason: it encodes Quoin's module archetypes, not generic assurance.                                                                                                       |
| `scripts/template-gate.mjs`                                                                        | mjs  |   191 |     1 | `oracle`          | template gate                                            | **3** ([#378](https://github.com/agent-ix/quoin/issues/378)) | `make template-gate`. **Runs `python3 -m ruff/black/pytest` on the rendered template** — a Python test oracle inside quoin's own gate, in quoin's own tree. Squarely in AC-5's path.                                                                                                                                                                |
| `scripts/verify-span-breadth.mjs`                                                                  | mjs  |   197 |     1 | `oracle`          | span breadth                                             | **6** ([#386](https://github.com/agent-ix/quoin/issues/386)) |                                                                                                                                                                                                                                                                                                                                                     |
| `scripts/workspace-policy-selftest.mjs`                                                            | mjs  |    97 |     1 | `oracle`          | workspace policy                                         | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) |                                                                                                                                                                                                                                                                                                                                                     |
| `scripts/lib/advisory-adjudication.mjs`, `lib/guidance-proof.mjs`                                  | mjs  |   460 |     2 | `oracle`          | advisor/auditor                                          | **5** ([#383](https://github.com/agent-ix/quoin/issues/383)) |                                                                                                                                                                                                                                                                                                                                                     |
| `scripts/release-drift.js`                                                                         | JS   |   308 |     1 | `oracle` (CI)     | release integrity                                        | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | The only script `release-drift.yml` runs (3 legs).                                                                                                                                                                                                                                                                                                  |
| `scripts/check-version-agreement.mjs`                                                              | mjs  |    84 |     1 | `oracle`          | version agreement                                        | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | `make check-version`.                                                                                                                                                                                                                                                                                                                               |
| `scripts/copy-quire-schemas.mjs`                                                                   | mjs  |    30 |     1 | `gate-tool`       | build step                                               | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | Part of `pnpm build`.                                                                                                                                                                                                                                                                                                                               |
| `scripts/check-trace-tags.mjs`                                                                     | mjs  |   134 |     1 | `oracle`          | trace-tag integrity                                      | **0**                                                        | Added by [#390](https://github.com/agent-ix/quoin/issues/390). Asserts every `// Trace:` tag names a criterion that exists. Deliberately unwired — wiring it changes the `Makefile` and `package.json` digests that `quality/verification-stack-lock.json` pins, so the relock is folded into [#350](https://github.com/agent-ix/quoin/issues/350). |
| `scripts/build-tools.js`, `scripts/help.js`                                                        | JS   |   198 |     2 | `gate-tool`       | version/info/help                                        | **9**                                                        | `make version`, `make info`, `pnpm help`.                                                                                                                                                                                                                                                                                                           |
| `scripts/freeze-{advisory-adjudication,guidance-review,span-breadth}.mjs`                          | mjs  |   499 |     3 | `manual`          | baseline re-pin                                          | **5**, **6**                                                 | **No caller anywhere** — see _Reachability warning_ below.                                                                                                                                                                                                                                                                                          |
| `scripts/refresh-{quire,semantic-core,manifest}-schemas.mjs`                                       | mjs  |   289 |     3 | `manual`          | schema refresh                                           | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | Only `refresh-quire-schemas.mjs` is referenced, and only by a comment in `src/quire/contract.ts`. These are the generator side of the JSON schemas in the tree: if the `generated` exception is ever claimed for anything here, the provenance recorder gets built in these files.                                                                  |

#### Eval harness — `evals/` (5,075 lines, 14 files)

| Path                          | Lang | Lines | Files | Reach             | Owner capability                         | Stage                                                        | Basis                                                                                                                                                |
| ----------------------------- | ---- | ----: | ----: | ----------------- | ---------------------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `evals/scenarios/index.mjs`   | mjs  | 2,057 |     1 | `manual` `oracle` | **[EA]** agent evaluation                | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | `make evals`. EA owns `agent_evals_host.rs` / `agent_evals_provider.rs`.                                                                             |
| `evals/lib/*.mjs` + `run.mjs` | mjs  | 3,018 |    13 | `manual` `oracle` | **[EA]** evaluation + evidence reporting | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | `quality.mjs` (1,145) is the largest. `assert.mjs`, `resolve.mjs`, `seed.mjs` spawn subprocesses. EA owns `evaluation.rs` + `evaluation_reports.rs`. |

#### Templates — `templates/` (1,767 lines, 11 files)

Policy: _"Templates that generate executable logic also need a target-language
decision; do not hide them as data."_ These are not data — they render and then run.

| Path                                                                                    | Lang    | Lines | Files | Reach                               | Owner capability       | Stage                                                        | Basis                                                                                                    |
| --------------------------------------------------------------------------------------- | ------- | ----: | ----: | ----------------------------------- | ---------------------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| `templates/semantic-module/hooks/{pre,post}_gen_project.py`                             | Py      |   289 |     2 | `oracle` (via `make gate`)          | cookiecutter render    | **3** ([#378](https://github.com/agent-ix/quoin/issues/378)) | Executes on every template render, including `make test`'s conformance leg.                              |
| `templates/semantic-module/{{cookiecutter.repo_name}}/tests/*.py`                       | Py      |   848 |     4 | `oracle` (via `make template-gate`) | rendered test suite    | **3** ([#378](https://github.com/agent-ix/quoin/issues/378)) | Python **test oracles** that `make gate` runs under pytest, in quoin's own tree. Named directly by AC-5. |
| `templates/.../scripts/{generate-schemas.mjs,stage-npm.mjs,build_tools.py,__init__.py}` | mjs, Py |   614 |     4 | `gate-tool`                         | rendered build tooling | **3** ([#378](https://github.com/agent-ix/quoin/issues/378)) | `generate-schemas.mjs` (377) `execFileSync`s the pinned TypeSpec compiler.                               |
| `templates/.../{{cookiecutter.package_name}}/__init__.py`                               | Py      |    16 |     1 | rendered                            | rendered package       | **3** ([#378](https://github.com/agent-ix/quoin/issues/378)) |                                                                                                          |

#### Install smoke — `smoke/` (509 lines, 5 files)

| Path                                      | Lang | Lines | Files | Reach         | Owner capability     | Stage | Basis                                                                                                                                                      |
| ----------------------------------------- | ---- | ----: | ----: | ------------- | -------------------- | ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `smoke/{run,entrypoint,agents,assert}.sh` | sh   |   417 |     4 | `oracle` (CI) | clean-room install   | **9** | `make install-smoke`, `install-smoke.yml`. Verifies the published package installs into four agent hosts. Survives until the CLI shell changes at Stage 9. |
| `smoke/modules.mjs`                       | mjs  |    92 |     1 | `oracle` (CI) | module install check | **9** | `execFileSync`s the installed CLI.                                                                                                                         |

#### Executable paths with no file to scan

A file-extension scanner cannot see any of these. They are executable paths all the same.

| Path                                       | Lang               | Reach    | Stage                                                        | Basis                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------ | ------------------ | -------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Makefile` → `answer-key-repin`            | Py (inline)        | `manual` | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | `python3 -c "import json,sys; ... json.dump(...)"` rewrites `bench/answer-key.json` — **the recall denominator**. An inline Python one-liner mutating the file every recall figure is scored against.                                                                                                                                                                                                                                                                                       |
| `.github/workflows/**` inline `run:` steps | sh (+`jq`, `node`) | CI       | **0** ([#375](https://github.com/agent-ix/quoin/issues/375)) | 35 steps across 4 workflows, ~118 lines of inline shell. All five workflows are `on: workflow_dispatch` only — nothing runs on push. Two steps carry semantics rather than orchestration: `build-test.yml`'s `test -n "$REGISTRY_TOKEN" \|\| exit 1` is a CI **assertion**, and `release.yml`'s 14-line publish block uses `git describe --exact-match` + `npm view` + `jq` to rewrite `package.json` and run `npm publish` — an **irreversible** shell decision. Neither is thin dispatch. |

## Consequence for AC-5 — proposed rewording

AC-5 currently reads:

> No Python or JavaScript executes as a test oracle after its capability's cutover;
> expected fixtures are checked in.

With `qa-corpus` ruled out of scope, AC-5 as written can never be satisfied: quoin's own
green bar (`make test` → `scripts/verification-stack.mjs:138`) runs
`python3 corpus/bounds.py --json`, and `bounds.py` lives in a repository this program
does not touch. The criterion overreaches — it was guarding against _quoin's_ engine
semantics hiding in a scripting language, not against a test corpus being written in
the languages it tests.

**Proposed wording:**

> **AC-5** No Python or JavaScript in quoin's own tree executes as a test oracle after
> its capability's cutover; expected fixtures are checked in. Executables inside
> out-of-scope submodules — `corpus/` (`agent-ix/qa-corpus`) — are excluded: a test
> corpus written in the languages it exercises is the corpus working as intended, not a
> non-Rust oracle. Quoin's gate may invoke them.

This keeps AC-5's teeth where they belong. Under the revised wording the criterion still
binds `scripts/template-gate.mjs` (which runs `python3 -m ruff/black/pytest` in quoin's
own tree) and `templates/.../tests/*.py` (848 lines of Python pytest oracles, also
quoin's own tree) — both still have to go at Stage 3.

## AC-9 — quoin-local capability EA already owns

Every row is capability `engineering-assurance` implements in Rust today. Per Amendment 1
these are **consumed, not re-grown** — and Stages 0–6 would otherwise rebuild all of it
in Rust, which is the same duplication in a better language.

| Quoin-local path                                                                         |           Lines | EA module that owns it                                            |
| ---------------------------------------------------------------------------------------- | --------------: | ----------------------------------------------------------------- |
| `scripts/verification-{stack,declarations,relock}*.mjs`                                  |           3,354 | `producer_execution.rs`, `process_host.rs`                        |
| `scripts/lib/tier1-*.mjs`, `bench-tier1.mjs`, `battletest.mjs`, `lib/tier2-baseline.mjs` |           4,846 | `compatibility_corpus.rs`, `evaluation_reports.rs`                |
| `evals/**`                                                                               |           5,075 | `agent_evals_host.rs`, `agent_evals_provider.rs`, `evaluation.rs` |
| `scripts/check-tool-drift*.mjs`                                                          |             797 | `package_audit.rs`                                                |
| canonical serialization inside `src/evidence/**`                                         | (part of 3,958) | `structured_yaml.rs`, `serde_json_canonicalizer`                  |
| **Total clearly EA-overlapping**                                                         |     **≈14,072** |                                                                   |

That is **17% of the retained population** whose correct disposition is _delete and
consume_, not _port_. Each needs the recorded three-part answer (must it be local / is
it Rust / should it be common); where EA does not fit, the answer is a gap ticket **in
EA**, never a local Rust harness.

Standing exception, unchanged: the **evidence store** stays Quoin's per EA's own
migration contract. That edge points EA → Quoin.

## Reachability warning — six scripts have no caller

Preserved as a finding; **no action taken**, by owner instruction.

| Path                                        | Lines | Why it is not dead                                                                                                      |
| ------------------------------------------- | ----: | ----------------------------------------------------------------------------------------------------------------------- |
| `scripts/freeze-advisory-adjudication.mjs`  |   181 | Writes the answer key `scripts/lib/advisory-adjudication.mjs` and `tests/advisory-adjudication.test.ts` assert against. |
| `scripts/freeze-span-breadth.mjs`           |   182 | Writes the baseline `verify-span-breadth.mjs` asserts against.                                                          |
| `scripts/freeze-guidance-review.mjs`        |   136 | Writes the baseline `tests/guidance-proof.test.ts` asserts against.                                                     |
| `scripts/refresh-manifest-schema.mjs`       |    68 | Regenerates a checked-in schema.                                                                                        |
| `scripts/refresh-semantic-core-schemas.mjs` |   121 | Regenerates checked-in schemas.                                                                                         |
| `scripts/refresh-quire-schemas.mjs`         |   100 | Referenced only by a comment in `src/quire/contract.ts`.                                                                |

Nothing in `package.json`, the `Makefile`, CI, any test or any other script names the
basename of any of these. They are human-typed maintenance commands whose _outputs_ are
load-bearing, so they are real work with an invisible edge. **Any enforcement tool that
keys "is this reachable?" on static callers will misreport all six** — the first five as
dead code, and the sixth on the strength of a code comment. A tool that then proposes
deleting them would delete the only way to re-pin the baselines every gate asserts
against. Reachability is not a sufficient signal here; the criterion has to be "does
anything depend on this file's output", which is a different question.

## Gaps this inventory turned up

1. **No Rust idiom document.** Quoin's future `rust/` workspace has no
   `.claude/skills/rust-style/SKILL.md`, no `clippy.toml`, no `deny.toml`, no
   `[workspace.lints]`. `filament-ide-rs` has a mature one; `engineering-assurance` has
   `deny.toml` and `rust-toolchain.toml`. The burn-down writes ~82k lines of Rust into
   that workspace — the idiom document is Stage 0 work, not a later tidy-up.
2. **Vendored bundles with no provenance** — [#374](https://github.com/agent-ix/quoin/issues/374).
3. **Six scripts with no static caller** — above, preserved, not acted on.

## Shape for the LR08 enforcement tool

Per LR08 ([quire-research#64](https://github.com/agent-ix/quire-research/issues/64)),
this matrix is a **byproduct the enforcement check emits**, not a script of its own.
The shape it must emit:

- One record per glob: `{path, lang, lines, files, reach, capability, stage, class, successor, expiry, basis}`.
- `class` ∈ `violation | retained | allowed`, exactly one — plus `flagged` as a fourth
  state for any path no exception resolves. A scanner that silently picks one of the
  three instead of flagging is the failure mode this baseline exists to prevent.
- Exceptions are read from [`.language-allowances.yaml`](../../.language-allowances.yaml),
  never hardcoded.
- The population is `git ls-files` ∩ extension set, so the tool must also walk `Makefile`
  recipes and `.github/workflows/**` `run:` blocks — 36 executable paths in this matrix
  have no file extension to scan.
- **Bypass probe required** (policy): a planted non-Rust semantic file in a non-allowed
  path must fail, and a planted hand-edit of a file claiming the `generated` exception
  must fail that exception. Without the probe the check passes over an empty population,
  which is not evidence.

## Reproducing these numbers

```
git ls-files '*.ts' '*.mjs' '*.js' '*.py' '*.sh' '*.tsp' \
  | while read f; do echo "$(wc -l < "$f") $f"; done
```

344 files, 102,248 lines at `22db1d7` (364 / 105,814 at `e718d45`, before
`scripts/storybook-deploy.js` was deleted). Submodule counts run the same command inside
`corpus/` at `7b81343`.
