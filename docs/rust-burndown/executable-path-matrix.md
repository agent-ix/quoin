# Executable-path disposition matrix — Quoin Rust burn-down

Satisfies **AC-3** of [quoin#373](https://github.com/agent-ix/quoin/issues/373).
Baseline for the **FP-NRX** metric (AC-7).

- **Measured at:** `quoin` commit `e718d45`, submodule `corpus` at `qa-corpus@7b81343`.
- **Measured on:** 2026-09-12.
- **Governing policy:** [`quire-research/implementation-language-policy.md`](https://github.com/agent-ix/quire-research/blob/main/implementation-language-policy.md),
  Amendment 1 (2026-09-12), section *Containment and burn-down are two programs*.
- **Containment counterpart:** quire-research [#56](https://github.com/agent-ix/quire-research/issues/56).
  Burn-down may not open a debt row containment's matrix does not carry; this matrix
  is offered to #56 as the shared population.

## What the numbers count

**Unit:** physical lines (`wc -l`), including blank and comment lines. Not SLOC, not
statements. This is the same unit the #373 scope table used, so the two are comparable.

**Population:** files tracked by `git ls-files` in `agent-ix/quoin` at the measured
commit whose extension is one of `.ts .mjs .js .py .sh .tsp`. Excludes `node_modules/`,
`dist/`, `coverage/`, `.worktrees/`, and untracked build output. Markdown, JSON, YAML
and lockfiles are excluded from the LOC population and appear only where a row needs
them for reachability.

**Rows** are path globs, not files. Every glob is disjoint and their union is exactly
the 364-file population, so the per-row counts reconcile to the total. A glob is the
unit an enforcement tool can emit and re-measure; one row per file would be 364 rows
that drift on the next commit.

**Reachability vocabulary**, one value per row:
- `shipped` — reachable from `bin/quoin.js`, the published npm entrypoint.
- `oracle` — determines a pass/fail assertion in `make test`, `make gate`, or CI.
- `gate-tool` — runs inside a gate but asserts nothing itself (build, render, refresh).
- `manual` — reachable only when a human types it.
- `dead` — no caller anywhere in the repo, CI, or Makefile.

## Correction to the #373 baseline

The scope table in #373 is a directory-level estimate. Three defects, all now measured:

| Defect | Effect |
|---|---|
| `skills/` was never counted | **+23,164 lines** omitted, and it is the single largest non-Rust block in the repo |
| `corpus/` was counted as quoin's | `corpus/` is a **git submodule** (`agent-ix/qa-corpus`). Its 6,551 executable lines are another repo's tree. The estimate also undercounted it (4,996 vs 6,551 measured). |
| `bin/` and root tool configs were never counted | +302 lines |

Measured in-repo population: **105,814 lines across 364 files** — 28% larger than the
estimate's in-repo part (82,417 after removing the submodule). `src/` 27,723, `tests/`
33,707, `scripts/` 13,485 and `smoke/` 509 reproduced the estimate exactly; `evals/`
measures 5,075 against the estimate's 5,226 (−151, cause not identified at this
revision, likely a different commit); `templates/` measures 1,849 against 1,767 (+82,
the `.tsp` schema source the estimate omitted).

## FP-NRX — measured baseline

| | Files | Lines |
|---|---:|---:|
| **Violations** | 1 | **325** |
| **Retained-with-successor** | 346 | **81,992** |
| **Allowed** | 2 | **52** |
| *Flagged — classification blocked on an owner ruling* | 15 | *23,445* |
| **Total in-repo population** | **364** | **105,814** |

Plus 2 non-file executable paths (Makefile inline Python, CI `run:` steps), classified
Retained-with-successor and carrying no LOC. Allowed counts only in-tree rows; the two
out-of-tree boundary rows (`corpus/cases/**`, `filament-core-data`) are recorded below
but excluded from the population.

Flagged lines are excluded from all three numbers rather than guessed into one. The
flagged block is 22% of the population and every line of it sits in one question
(§ Flagged, F-1). Until F-1 is ruled, **the metric has a ±23,164-line band and cannot
be reported as a single number.** The other three flags move 281 lines.

## Matrix

Columns: **Path** · **Lang** · **Lines** · **Files** · **Reach** (see vocabulary above) ·
**Owner capability** · **Stage** (0–9 per #373) · **Class** · **Successor / basis**.

### Violations

| Path | Lang | Lines | Files | Reach | Owner capability | Stage | Class | Successor / basis |
|---|---|---:|---:|---|---|---|---|---|
| `scripts/storybook-deploy.js` | JS | 325 | 1 | `dead` | none — k8s/Storybook deploy boilerplate | — | **Violation** | Delete. Repo has zero Storybook dependencies (`grep -c storybook package.json pnpm-lock.yaml` → 0,0) and no file, Makefile target, package script or workflow names it. No successor needed; this is not debt to port, it is debt to remove. |

### Allowed

| Path | Lang | Lines | Files | Reach | Owner capability | Stage | Class | Successor / basis |
|---|---|---:|---:|---|---|---|---|---|
| `bin/quoin.js` | JS | 21 | 1 | `shipped` | CLI entry | 9 | **Allowed** | Allowance 4 — thin host dispatch. 21 lines, zero branches. Survives to Stage 9 as the Node shim; deleted with oclif. Ceiling to declare in the manifest: **40 lines / 2 branches**. |
| `src/hooks/command-not-found.ts` | TS | 31 | 1 | `shipped` | oclif hook contract | 9 | **Allowed** | Allowance 4 — thin host dispatch. Declared in `package.json` `oclif.hooks`; retiring it is a published-extension contract change (#373 *Known risks*). Same ceiling. |
| `corpus/cases/**` | Rust/TS/Py/sh | 2,080 | 162 | inert input | `agent-ix/qa-corpus` | — | **Allowed** | Allowance 3 — inert sample inputs, owner-decided. Recorded so a naive extension scan cannot count them as debt forever. **Not in quoin's tree** — `corpus` is a submodule; these lines are excluded from quoin's population totals above. Composition: `.rs` 906 / `.ts` 613 / `.py` 525 / `.sh` 36. |
| `filament-core-data` `python_backend/`, `packages/` | Py/TS | — | — | out of tree | `agent-ix/filament-core-data` | — | **Allowed** | Allowance 6 — dated owner disposition, retained TypeSpec source and generator-wrapper harness. **Boundary statement:** fcd is the schema source of truth and its gate tickets (#7 Phase A, #11 Phase B) block this program, but its tree is not quoin's and no quoin burn-down row may claim it. Consuming fcd's interface authorizes no change to fcd. |

Allowed line total counts only the two in-tree rows (52). The other two rows are outside
quoin's measured population and are recorded for completeness of the boundary, not for
the metric.

### Retained-with-successor

All rows are allowance 5. **Successor convention:** until a stage's own ticket is cut,
the named successor is `quoin#373 Stage N` and the expiry is that stage's end. See
Flagged F-4 — this convention needs an owner ruling before it can be relied on.

#### Engine — `src/` (27,692 lines, 169 files, all `shipped`)

| Path | Lang | Lines | Files | Reach | Owner capability | Stage | Class | Successor / basis |
|---|---|---:|---:|---|---|---|---|---|
| `src/measurement/**` | TS | 7,851 | 35 | `shipped` + `oracle` | measurement model | **6** | Retained-w/successor | Largest single block and the last logic stage. Shells out: `engine-run.ts`, `enumerate.ts`, `fixture-corpus.ts`, `modules.ts` all `execFileSync`. |
| `src/evidence/**` | TS | 3,958 | 18 | `shipped` | evidence store | **4** | Retained-w/successor | Quoin keeps ownership of the store (EA migration contract). Port relocates language, not ownership. Carries the digest/JCS surface — #373's hard gate. |
| `src/commands/evidence/**` | TS | 1,173 | 10 | `shipped` | evidence commands | **8** | Retained-w/successor | `affirm.ts`, `audit.ts`, `baseline.ts` `execFileSync` out. |
| `src/commands/*.ts` | TS | 1,126 | 11 | `shipped` | command shell | **8** | Retained-w/successor | `assurance.ts` `execFileSync`s. |
| `src/commands/change-assurance/**` | TS | 796 | 9 | `shipped` | change assurance | **4** | Retained-w/successor | |
| `src/commands/{catalog,measurement,graph,module,semantic,config,plugin}/**` | TS | 796 | 31 | `shipped` | command shell | **8** | Retained-w/successor | catalog 179/5 · measurement 139/4 · graph 135/5 · module 99/5 · semantic 90/1 · config 88/5 · plugin 66/6. `semantic/sweep.ts` `execFileSync`s. |
| `src/change-assurance/**` | TS | 2,023 | 8 | `shipped` | change assurance | **4** | Retained-w/successor | **Cycle A:** `store.ts:22` imports `storeRoot` from `../evidence/store.js` while `evidence/index.ts:142` re-exports from `../change-assurance/schema-assets.js`. Cargo forbids this; break before any crate exists. |
| `src/assurance/**` | TS | 1,723 | 5 | `shipped` | assurance records | **5** | Retained-w/successor | |
| `src/graph-analysis/**` | TS | 1,373 | 5 | `shipped` | graph views | **5** | Retained-w/successor | |
| `src/*.ts` (root: `base cli catalog config-schema flow-command flows index modules org plugins version write`) | TS | 1,358 | 12 | `shipped` | CLI core, config, modules | **7**, **9** | Retained-w/successor | `flows.ts` `spawn`s `ix-flow` — a transitive call out to a Node tool, see F-1. `config-schema.ts` is 56 lines of zod, a verified non-risk. |
| `src/quire/**` | TS | 1,342 | 6 | `shipped` + `oracle` | Quire adapter | **1**, **8** | Retained-w/successor | `exec.ts` is the boundary template #373 adopts; it is the **last** file deleted (Stage 8). |
| `src/semantic/**` | TS | 1,323 | 6 | `shipped` | semantic modules | **3** | Retained-w/successor | `manifest.ts` `mapAjvError` — ajv↔`jsonschema` parity risk lands here. |
| `src/auditor/**` | TS | 1,139 | 3 | `shipped` | auditor | **5** | Retained-w/successor | **Cycle B:** `auditor` ↔ `advisor`. Break before Stage 5. |
| `src/advisor/**` | TS | 862 | 3 | `shipped` | advisor | **5** | Retained-w/successor | Cycle B other half. |
| `src/completeness/**` | TS | 685 | 5 | `shipped` | completeness | **3** | Retained-w/successor | |
| `src/validators/**` | TS | 164 | 2 | `shipped` | validators | **2** | Retained-w/successor | Smallest engine module — #373's boundary proof-of-life. |

#### Tests — `tests/` (33,707 lines, 109 files, all `oracle`)

| Path | Lang | Lines | Files | Reach | Owner capability | Stage | Class | Successor / basis |
|---|---|---:|---:|---|---|---|---|---|
| `tests/**/*.test.ts` | TS | 32,131 | 100 | `oracle` | per-domain | follows its code | Retained-w/successor | Not debt while they run: they are the parity oracle for the TS they cover. Each test retires **in the same commit** as the code it covers, with its criteria restated on a tracking-tagged Rust test (#373, AC-4/AC-5). 33 of them `execFileSync`/`spawnSync`. |
| `tests/props/*.prop.test.ts` | TS | 1,157 | 7 | `oracle` | property tests | follows its code | Retained-w/successor | Header: *"Property tests generated by the `spec-correctness` skill."* **Not allowance 2** — the header names a generator identity but no source schema digest, and the files are hand-maintainable. Treated as ordinary retained tests. `fr-005.prop.test.ts` `execSync`s. |
| `tests/{support,fixtures,setup.ts,global-setup.ts}` | TS | 419 | 2 | `oracle` harness | test harness | follows its code | Retained-w/successor | `support/semantic-module-template.ts` `execFileSync`s the cookiecutter. |

#### Qualification scripts — `scripts/` (13,160 lines, 37 files)

These are the rows where **AC-9** bites: most of this block is capability
`engineering-assurance` already owns in Rust. Marked **[EA]** below.

| Path | Lang | Lines | Files | Reach | Owner capability | Stage | Class | Successor / basis |
|---|---|---:|---:|---|---|---|---|---|
| `scripts/verification-stack.mjs` + `-selftest.mjs` | mjs | 1,515 | 2 | `oracle` | **[EA]** bounded producer execution, process supervision | **0** | Retained-w/successor | `make test` — *the* local green bar. Runs `python3 corpus/bounds.py --json` and asserts toolchain identity. EA owns this as `producer_execution.rs` + `process_host.rs`. Consume, do not re-grow in Rust. |
| `scripts/verification-declarations.mjs` + `-selftest.mjs` | mjs | 731 | 2 | `oracle` | **[EA]** producer execution | **0** | Retained-w/successor | Gate leg of `make validate`. |
| `scripts/verification-relock.mjs` + `-selftest.mjs` + `verification-object-integrity-selftest.mjs` | mjs | 1,108 | 3 | `manual` + `oracle` | **[EA]** evidence reporting | **0** | Retained-w/successor | `make verification-relock`. Runs `python3 -I corpus/bounds.py --json`. |
| `scripts/lib/tier1-*.mjs` (`execution comparison corpus render measurement recall scoring`) | mjs | 2,569 | 7 | `oracle` | **[EA]** corpus accounting, evaluation reporting | **0** | Retained-w/successor | EA owns `compatibility_corpus.rs` + `evaluation_reports.rs`. |
| `scripts/bench-tier1.mjs` | mjs | 921 | 1 | `oracle` | **[EA]** evaluation reporting | **0** | Retained-w/successor | Runs `python3 corpus/bounds.py`. Asserts the attestation pins node, rust **and python** toolchains — the Python dependency is load-bearing and declared. |
| `scripts/battletest.mjs` + `lib/tier2-baseline.mjs` | mjs | 1,356 | 2 | `manual` + `oracle` | **[EA]** corpus accounting | **0** | Retained-w/successor | `make battletest`, needs an external pinned tree. |
| `scripts/check-tool-drift.mjs` + `-selftest.mjs` | mjs | 797 | 2 | `oracle` | **[EA]** package audit | **0** | Retained-w/successor | `make audit-tool-drift`. |
| `scripts/lib/semantic-module-type-fit.mjs` + `semantic-module-type-fit.mjs` | mjs | 1,810 | 2 | `oracle` | semantic modules — **domain-specific** | **3** | Retained-w/successor | Three-part test answered: must be local **yes** (Quoin's own module type system); is it Rust **no, port at Stage 3**; should it be common **no** — recorded reason: it encodes Quoin's module archetypes, not generic assurance. |
| `scripts/template-gate.mjs` | mjs | 191 | 1 | `oracle` | template gate | **3** | Retained-w/successor | `make template-gate`. **Runs `python3 -m ruff/black/pytest` on the rendered template** — a Python test oracle inside quoin's gate. AC-5 applies: it must not survive its capability's cutover. |
| `scripts/verify-span-breadth.mjs` | mjs | 197 | 1 | `oracle` | span breadth | **6** | Retained-w/successor | |
| `scripts/workspace-policy-selftest.mjs` | mjs | 97 | 1 | `oracle` | workspace policy | **0** | Retained-w/successor | |
| `scripts/lib/advisory-adjudication.mjs`, `lib/guidance-proof.mjs` | mjs | 460 | 2 | `oracle` | advisor/auditor | **5** | Retained-w/successor | |
| `scripts/release-drift.js` | JS | 308 | 1 | `oracle` (CI) | release integrity | **0** | Retained-w/successor | Only script a *scheduled-dispatch* workflow runs (`release-drift.yml`, 3 legs). |
| `scripts/check-version-agreement.mjs` | mjs | 84 | 1 | `oracle` | version agreement | **0** | Retained-w/successor | `make check-version`. |
| `scripts/copy-quire-schemas.mjs` | mjs | 30 | 1 | `gate-tool` | build step | **0** | Retained-w/successor | Part of `pnpm build`. |
| `scripts/build-tools.js`, `scripts/help.js` | JS | 198 | 2 | `gate-tool` | version/info/help | **9** | Retained-w/successor | `make version`, `make info`, `pnpm help`. |
| `scripts/freeze-{advisory-adjudication,guidance-review,span-breadth}.mjs` | mjs | 499 | 3 | `manual` | baseline re-pin | **5**, **6** | Retained-w/successor | **No caller anywhere** — not package.json, not Makefile, not CI, not any test. Human-typed baseline re-pins. They write the answer keys other gates assert against, so they are not dead, but they are unreachable-by-tooling and an enforcement scanner will see them as orphans. |
| `scripts/refresh-{quire,semantic-core,manifest}-schemas.mjs` | mjs | 289 | 3 | `manual` | schema refresh | **0** | Retained-w/successor | Only `refresh-quire-schemas.mjs` is referenced (a comment in `src/quire/contract.ts`). The other two have no caller. These are the generator side of the JSON schemas in the tree — the provenance recorder for allowance 2 has to be built here. |

#### Eval harness — `evals/` (5,075 lines, 14 files)

| Path | Lang | Lines | Files | Reach | Owner capability | Stage | Class | Successor / basis |
|---|---|---:|---:|---|---|---|---|---|
| `evals/scenarios/index.mjs` | mjs | 2,057 | 1 | `manual` `oracle` | **[EA]** agent evaluation | **0** | Retained-w/successor | `make evals`. EA owns `agent_evals_host.rs` / `agent_evals_provider.rs`. |
| `evals/lib/*.mjs` + `run.mjs` | mjs | 3,018 | 13 | `manual` `oracle` | **[EA]** evaluation + evidence reporting | **0** | Retained-w/successor | `quality.mjs` 1,145 is the largest. `assert.mjs`, `resolve.mjs`, `seed.mjs` spawn subprocesses. EA owns `evaluation.rs` + `evaluation_reports.rs`. |

#### Templates — `templates/` (1,849 lines, 12 files)

Policy: *"Templates that generate executable logic also need a target-language
decision; do not hide them as data."* These are not data.

| Path | Lang | Lines | Files | Reach | Owner capability | Stage | Class | Successor / basis |
|---|---|---:|---:|---|---|---|---|---|
| `templates/semantic-module/hooks/{pre,post}_gen_project.py` | Py | 289 | 2 | `oracle` (via `make gate`) | cookiecutter render | **3** | Retained-w/successor | Executes during every template render, including in `make test`'s conformance leg. |
| `templates/semantic-module/{{cookiecutter.repo_name}}/tests/*.py` | Py | 848 | 4 | `oracle` (via `make template-gate`) | rendered test suite | **3** | Retained-w/successor | Python **test oracles** that `make gate` runs under pytest. Directly named by AC-5: no Python may execute as a test oracle after this capability's cutover. |
| `templates/semantic-module/{{cookiecutter.repo_name}}/scripts/{generate-schemas.mjs,stage-npm.mjs,build_tools.py,__init__.py}` | mjs, Py | 614 | 4 | `gate-tool` | rendered build tooling | **3** | Retained-w/successor | `generate-schemas.mjs` (377) `execFileSync`s the pinned TypeSpec compiler. |
| `templates/.../{{cookiecutter.package_name}}/__init__.py` | Py | 16 | 1 | rendered | rendered package | **3** | Retained-w/successor | |
| `templates/.../typespec/main.tsp` | TypeSpec | 82 | 1 | schema source | schema source | **3** | Retained-w/successor | Provisional — see Flagged F-3. |

#### Install smoke — `smoke/` (509 lines, 5 files)

| Path | Lang | Lines | Files | Reach | Owner capability | Stage | Class | Successor / basis |
|---|---|---:|---:|---|---|---|---|---|
| `smoke/{run,entrypoint,agents,assert}.sh` | sh | 417 | 4 | `oracle` (CI) | clean-room install | **9** | Retained-w/successor | `make install-smoke`, `install-smoke.yml`. Verifies the published package installs into four agent hosts. Survives until the CLI shell changes at Stage 9. |
| `smoke/modules.mjs` | mjs | 92 | 1 | `oracle` (CI) | module install check | **9** | Retained-w/successor | `execFileSync`s the installed CLI. |

#### Non-file executable paths

| Path | Lang | Lines | Files | Reach | Owner capability | Stage | Class | Successor / basis |
|---|---|---:|---:|---|---|---|---|---|
| `Makefile` → `answer-key-repin` target | Py (inline) | inline | — | `manual` | answer-key pin | **0** | Retained-w/successor | `python3 -c "import json,sys; ... json.dump(...)"` rewrites `bench/answer-key.json` — **the recall denominator**. An inline Python one-liner mutating the file every recall figure is scored against. A file-extension scanner cannot see it. |
| `.github/workflows/**` inline `run:` steps | sh (+`jq`, `node`) | ~118 inline | 35 steps, 4 files | CI | CI orchestration | **0** | Retained-w/successor | All five workflows are `on: workflow_dispatch` only — nothing runs on push. Two steps carry semantics rather than orchestration: `build-test.yml`'s `test -n "$REGISTRY_TOKEN" \|\| exit 1` is a CI **assertion**, and `release.yml`'s 14-line publish block uses `git describe --exact-match` + `npm view` + `jq` to rewrite `package.json` and `npm publish` — an **irreversible** shell decision. Both need a disposition; neither is thin dispatch. |

## Flagged — owner ruling required

Stated, not decided, per the brief.

### F-1 · `skills/*/workflow-assets/**` — 23,164 lines, 22% of the population

`skills/spec-review/`, `skills/spec-matrix/` and `skills/spec-to-plan/` each carry an
identical `workflow-assets/dist/index.js` — **7,525 lines, md5 `ea3d2c74…`, three
byte-identical copies**, plus `index.d.ts` (145×3), `invariants.d.ts` (5×3) and
`skills/*/scripts/invariants.js` (137 + 1 + 1).

The facts:
- It is a **minified bundle**, and it inlines third-party code — the first
  `//#region` is `node_modules/.pnpm/zod@4.4.3/…`.
- It **executes**: `src/flows.ts:57` resolves `workflow-assets/skills/<flow>/` and
  `spawn`s `ix-flow` against it; the skill's `scripts/invariants.js` does
  `import { specInvariants } from "../../../dist/index.js"`. Those invariants decide
  whether a spec review, matrix or plan flow passes. That is assertion logic.
- It **ships**: `skills/` is in `package.json` `files`.
- It carries **no generator provenance** — no `@generated` marker, no generator
  identity, no source schema digest, nothing.

The question: **is this first-party quoin code at all?** It looks like vendored build
output of `agent-ix/ix-spec-workflows` (which `src/flows.ts` names as a sibling
checkout). Three readings, each giving a different metric:

1. **Third-party vendored dependency** → outside the first-party population entirely.
   FP-NRX unchanged. Needs a dependency/host disposition instead (policy: *"External
   reference tools … need a scoped dependency/host disposition"*), recording identity,
   version and qualification reliance.
2. **First-party generated** → allowance 2, but allowance 2 requires generator identity
   **and source schema digest** recorded, and a hand-edit forfeits it. Nothing records
   either today, and three byte-identical copies with no build step that produces them
   is exactly the shape a hand-edit hides in. Allowance 2 cannot be claimed as-is.
3. **First-party executable, unallowed** → **+23,164 to Violations**, which is 71× the
   next-largest violation, and AC-7's "Violations = 0 from the day LR08 lands" becomes
   unreachable on day one.

Recommendation to put to the owner: reading 1, with a dependency disposition and a
recorded provenance stamp so reading 2 becomes available later. The vendoring itself
(three identical 7.5k-line copies, checked in, no build step, no digest) is a separate
defect worth its own ticket regardless of the ruling.

### F-2 · Root tool configs — 281 lines

`vite.config.ts` (200), `cli-agent-evals.config.mjs` (54), `eslint.config.js` (27).
Build/lint/eval tool configuration. They execute, they are first-party, and they are
not UI, not generated, not inert, not thin dispatch, and not staged-port retention —
**no declared allowance fits them.** Read literally they are Violations, which is
plainly not the intent. Either a seventh allowance for tool configuration is declared,
or each gets a dated disposition under allowance 6. `vite.config.ts` in particular
determines what `dist/` contains, so it is not cosmetic.

### F-3 · `templates/.../typespec/main.tsp` — 82 lines

TypeSpec is a schema language, so it reads as allowance 3 inert data — but it is the
**input** the pinned TypeSpec compiler turns into the JSON Schemas the rendered
module's Python tests then assert against. Data that determines an assertion two hops
downstream. Classified Retained-with-successor provisionally; if it is ruled inert,
82 lines move to Allowed. Note this is also the one file where quoin and
`filament-core-data` use the same technology for the same job — worth an owner look
at whether it should be fcd's.

### F-4 · The successor convention itself

Allowance 5 requires *"a path listed in an open matrix row with a named successor
ticket and an expiry."* Stage tickets 0–9 do not exist yet — #373 describes the stages
in prose. Read strictly, **every one of the 330 Retained rows fails allowance 5 today
and all 81,992 lines are Violations** until stage tickets are cut.

This matrix declares the convention "successor = `quoin#373` Stage N, expiry = that
stage's end" so the metric is reportable now. That convention needs the owner to
either bless it or require the ten stage tickets before LR08's scanner can pass. The
answer changes AC-7's day-one reading, so it should land before LR08 does.

### F-5 · `corpus/` — the exclusion is narrower than it reads

The owner exclusion is `quoin/corpus/cases/**`, allowance 3, inert. That is correct and
recorded above. But **the rest of the submodule is not inert**, and quoin's own green
bar depends on it:

| Path | Lines | Files | How quoin executes it |
|---|---:|---:|---|
| `corpus/bounds.py` | 1,408 | 1 | `scripts/verification-stack.mjs:138` runs `python3 bounds.py --json`; `bench-tier1.mjs:123` and `verification-relock.mjs:120` also run it. Its output feeds tier-1 measurement. |
| `corpus/verify.py` | 964 | 1 | corpus's own verification |
| `corpus/scripts/*.py` | 2,099 | 8 | `duplicate_census`, `export_measurements`, `measurement_selftest`, `parity_selftest`, `schema_selftest`, `verify_reporting`, `external_channel_selftest`, `new_case` |

So **4,471 lines of Python outside `cases/` determine assertions in `make test`**, and
AC-5 ("no Python executes as a test oracle after cutover") cannot be satisfied by
quoin alone — `bounds.py` lives in `agent-ix/qa-corpus`. Two questions: does the
burn-down get to open rows against qa-corpus, or is that containment's (#56) to
carry; and is qa-corpus in this program's scope at all, given Amendment 1 named fcd
in and the Filament repos out but said nothing about qa-corpus. #373's "corpus
consolidation with quire-corpus" parallel track is presumably where this lands, but
it is not stated.

## AC-9 — quoin-local capability EA already owns

Every row below is capability `engineering-assurance` implements in Rust today. Per
Amendment 1 these are **consumed, not re-grown** — and Stages 0–6 would otherwise
rebuild all of it in Rust, which is the same duplication in a better language.

| Quoin-local path | Lines | EA module that owns it |
|---|---:|---|
| `scripts/verification-stack*.mjs`, `verification-declarations*.mjs`, `verification-relock*.mjs` | 3,354 | `producer_execution.rs`, `process_host.rs` |
| `scripts/lib/tier1-*.mjs`, `bench-tier1.mjs`, `battletest.mjs`, `lib/tier2-baseline.mjs` | 4,846 | `compatibility_corpus.rs`, `evaluation_reports.rs` |
| `evals/**` | 5,075 | `agent_evals_host.rs`, `agent_evals_provider.rs`, `evaluation.rs` |
| `scripts/check-tool-drift*.mjs` | 797 | `package_audit.rs` |
| canonical serialization inside `src/evidence/**` | (part of 3,958) | `structured_yaml.rs`, `serde_json_canonicalizer` |
| **Total clearly EA-overlapping** | **≈14,072** | |

That is **17% of the retained population** whose correct disposition is *delete and
consume*, not *port*. Each needs the recorded three-part answer (must it be local /
is it Rust / should it be common); where EA does not fit, the answer is a gap ticket
**in EA**, never a local Rust harness.

Standing exception, unchanged: the **evidence store** stays Quoin's per EA's own
migration contract. That edge points EA → Quoin.

## Gaps this inventory turned up

1. **No Rust idiom document.** Quoin's future `rust/` workspace has no
   `.claude/skills/rust-style/SKILL.md`, no `clippy.toml`, no `deny.toml`, no
   `[workspace.lints]`. `filament-ide-rs` has a mature one; `engineering-assurance`
   has `deny.toml` and `rust-toolchain.toml`. The burn-down writes ~82k lines of Rust
   into that workspace — the idiom doc is Stage 0 work, not a later tidy-up.
2. **Six scripts have no caller** (1,013 lines): three `freeze-*` (they write the
   answer keys other gates assert against — real, but human-typed), two
   `refresh-*-schemas`, and `storybook-deploy.js` (genuinely dead). An enforcement
   scanner that keys on reachability will misreport all six.
3. **Three byte-identical 7.5k-line vendored bundles** with no build step and no
   digest (F-1).

## Shape for the LR08 tool

Per LR08 ([quire-research#64](https://github.com/agent-ix/quire-research/issues/64)),
this matrix is a **byproduct the enforcement check emits**, not a script of its own.
The shape it must emit:

- One record per glob: `{path, lang, lines, files, reach, capability, stage, class, successor, expiry, basis}`.
- `class` ∈ `violation | retained | allowed`, exactly one, plus `flagged` as a fourth
  state for rows no allowance resolves — a scanner that silently picks one is the
  failure mode this baseline exists to prevent.
- Allowances 1–4 read from a checked-in manifest (policy requirement, **does not exist
  yet** — propose `quoin/.language-allowances.yaml`, carrying the line/branch ceiling
  for allowance 4 and the generator identity + source schema digest for allowance 2).
- The population is `git ls-files` ∩ extension set, so the tool must also walk
  `Makefile` recipes and `.github/workflows/**` `run:` blocks — 36 executable paths
  in this matrix have no file extension to scan.
- Emit the three totals plus the flagged count. A report of three numbers with no
  flagged count hides F-1's 23,164-line band.
- **Bypass probe required** (policy): a planted non-Rust semantic file in a
  non-allowed path must fail, and a planted hand-edit of a generated file must fail
  allowance 2. Without it the check passes over an empty population, which is not
  evidence.

## Reproducing these numbers

```
git ls-files '*.ts' '*.mjs' '*.js' '*.py' '*.sh' '*.tsp' \
  | while read f; do echo "$(wc -l < "$f") $f"; done
```
364 files, 105,814 lines at `e718d45`. Submodule counts run the same command inside
`corpus/` at `7b81343`.
