---
id: TM-002
title: quoin Authoring Eval Matrix
type: TestMatrix
relationships:
  - target: "ix://agent-ix/quoin/US-001"
    type: "covers"
  - target: "ix://agent-ix/quoin/US-002"
    type: "covers"
  - target: "ix://agent-ix/quoin/US-003"
    type: "covers"
  - target: "ix://agent-ix/quoin/US-004"
    type: "covers"
  - target: "ix://agent-ix/quoin/US-005"
    type: "covers"
  - target: "ix://agent-ix/quoin/US-008"
    type: "covers"
  - target: "ix://agent-ix/quoin/US-011"
    type: "covers"
---

# quoin Authoring Eval Matrix

## Purpose

This matrix defines the minimum agent-facing eval set for spec creation and
maintenance. Unit tests prove command behavior; these evals measure whether an
agent can use the commands efficiently to produce valid spec artifacts.

These evals are also `quoin`'s real end-to-end coverage layer: each scenario runs
the real `quoin` CLI and a real `quire validate`. They therefore exercise the
same external boundaries as the integration tests in `spec/integration/`, with an
agent in the loop. The two integration tests
([IT-001](./integration/IT-001-default-module-reconcile.md),
[IT-002](./integration/IT-002-github-plugin-install.md)) name the live-git
boundaries that warrant a deterministic test in addition to this agent-driven
coverage; the metric-capture requirement for this matrix is
[NFR-006](./non-functional/NFR-006-eval-metric-capture.md).

> **Runner status (2026-06-15):** the **agent-pty-driven** harness is built and lives
> in `evals/` (`node evals/run.mjs`, or `make evals` / `make evals-all`). It drives
> the real agent (Claude) through this CLI + the `/spec-*` skills via `agent-pty`
> (tmux), and records the metrics below **for real** from the Claude Code session
> transcript (token usage, tool calls, latency, validation attempts, context
> fetches). The prior deterministic runner — which faked `tokenUsage` (always null)
> and counted `contextFetches` mechanically — was removed. Each scenario runs in an
> isolated repo + `IX_HOME` seeded once from the default modules; success is asserted
> by file presence + an independent `quire validate` + a completion sentinel. The
> scenario definitions (EV-001..EV-015) below remain the source of truth; the harness
> reads them from `evals/scenarios/index.mjs`. See `evals/README.md` for usage.

## Metrics

| Metric              | Definition                                                      | Target                      |
| ------------------- | --------------------------------------------------------------- | --------------------------- |
| Success             | Eval finishes with valid expected files and passing validation. | 100% for release candidates |
| Latency             | Wall-clock time from first command to final validation result.  | Track p50/p95 by scenario   |
| Tool calls          | Number of shell/CLI/tool invocations used by the agent.         | Track and minimize          |
| Input tokens        | Prompt/context tokens consumed by the agent.                    | Track and minimize          |
| Output tokens       | Agent response/output tokens.                                   | Track and minimize          |
| Validation attempts | Number of Quire validation runs before success.                 | Prefer 1                    |
| Context fetches     | Number of repeated template/catalog fetches for the same type.  | Prefer 1 per type pack      |

## Variation Coverage

| Dimension             | Required Variations                                                                                           |
| --------------------- | ------------------------------------------------------------------------------------------------------------- |
| Authoring mode        | greenfield repo, brownfield edit, multi-document creation, validation repair loop                             |
| Type source           | bundled artifact, bundled object, installed local plugin, installed GitHub/plugin package, sibling dev module |
| Type lookup           | canonical casing, lowercase input, mixed artifact/object request, unknown type                                |
| Validation scope      | exact module scope, repo search scope, multiple glob arguments, changed-file subset, invalid document         |
| Workflow lifecycle    | review launch, matrix launch, to-plan launch, status inspection, human gate handoff                           |
| Config state          | clean isolated `~/.ix`, existing plugin registry, plugin removal/reinstall                                    |
| Agent efficiency      | one authoring pack reused across multiple files, no repeated template fetch for same type                     |
| Artifact completeness | request → required artifact set: new project, add US only, edit FR only, add US+FR, backport → FR artifacts   |

## Scenarios

| Eval   | Use Case       | Prompt                                                                           | Expected Outcome                                                                                                                                                                                                                                                                                                                                                                                                                             | Required Measurements                                     |
| ------ | -------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| EV-001 | US-001         | Create an FR plus `domain` and `entity` objects for a small feature.             | Agent uses one `quoin write` pack, creates all files, and Quire validates them.                                                                                                                                                                                                                                                                                                                                                              | success, latency, tool calls, tokens, validation attempts |
| EV-002 | US-002         | Edit an existing FR after discovering its authoring contract.                    | Agent uses `catalog show` or `write`, edits only needed sections, and validates changed files.                                                                                                                                                                                                                                                                                                                                               | success, latency, tool calls, tokens                      |
| EV-003 | US-003         | Install a local fixture plugin and author one plugin-defined object.             | Plugin appears in catalog, `write --types` resolves it, created file validates.                                                                                                                                                                                                                                                                                                                                                              | success, latency, tool calls, tokens                      |
| EV-004 | US-004         | Validate a mixed changed set of artifact and object files.                       | Agent runs scoped Quire validation over the changed file set and reports failures clearly.                                                                                                                                                                                                                                                                                                                                                   | success, latency, tool calls, validation attempts         |
| EV-005 | US-005         | Start a review workflow for a spec directory and inspect status.                 | `quoin` starts the workflow and `ix-flow status` reports the run and gate state.                                                                                                                                                                                                                                                                                                                                                             | success, latency, tool calls                              |
| EV-006 | US-001, US-004 | Create multiple artifacts that share object templates.                           | Agent fetches each type contract once, reuses it across files, and validates all changed files.                                                                                                                                                                                                                                                                                                                                              | success, context fetches, tokens, tool calls              |
| EV-007 | US-002         | Use lowercase type names in an authoring request.                                | `quoin write . --types fr,domain` resolves canonical types and prints usable contracts.                                                                                                                                                                                                                                                                                                                                                      | success, latency, tool calls                              |
| EV-008 | US-004         | Repair a spec file after Quire reports validation diagnostics.                   | Agent interprets diagnostics, edits the failing file, and reaches passing validation.                                                                                                                                                                                                                                                                                                                                                        | success, latency, tool calls, tokens, validation attempts |
| EV-009 | US-003         | Install, list, remove, and reinstall a local plugin fixture.                     | Plugin registry changes are isolated to `IX_HOME`; catalog reflects each lifecycle step.                                                                                                                                                                                                                                                                                                                                                     | success, latency, tool calls                              |
| EV-010 | US-003         | Install a GitHub/package plugin fixture and request one of its types.            | `write --types` resolves the installed type and prints usable paths.                                                                                                                                                                                                                                                                                                                                                                         | success, latency, tool calls, tokens                      |
| EV-011 | US-002         | Request an unknown type in an authoring pack.                                    | CLI reports the missing type clearly and exits non-zero.                                                                                                                                                                                                                                                                                                                                                                                     | success, latency, tool calls                              |
| EV-012 | US-004         | Validate multiple globs for a changed-file subset.                               | Quire validates only matching files and reports each invalid file with a path.                                                                                                                                                                                                                                                                                                                                                               | success, latency, tool calls, validation attempts         |
| EV-013 | US-005         | Start matrix and to-plan workflows after accepted requirements.                  | `quoin matrix` and `quoin to-plan` create runs that `ix-flow status` can inspect.                                                                                                                                                                                                                                                                                                                                                            | success, latency, tool calls                              |
| EV-014 | US-001, US-002 | Author a complete spec set from a settled conversation.                          | Agent creates spec, use cases, matrix, plan, and eval matrix, then validates all spec files.                                                                                                                                                                                                                                                                                                                                                 | success, latency, tool calls, tokens, context fetches     |
| EV-015 | US-001, US-004 | Author with sibling development modules present.                                 | Catalog prefers intended dev modules deterministically and validation uses matching contracts.                                                                                                                                                                                                                                                                                                                                               | success, latency, tool calls, tokens                      |
| EV-016 | US-001, US-004 | Author into two sibling repos in one session.                                    | Agent authors under `core/spec/` and `service/spec/` and validates both with one scoped run.                                                                                                                                                                                                                                                                                                                                                 | success, latency, tool calls, tokens                      |
| EV-017 | US-001, US-002 | Author a larger feature spec set with cross-references.                          | Agent authors StR + 2 US + 3 FR + NFR + domain + 2 entity, fetching each contract once; all validate.                                                                                                                                                                                                                                                                                                                                        | success, latency, tool calls, tokens, context fetches     |
| EV-018 | US-001         | Author objects drawn from three different object modules.                        | One `write` pack resolves `domain`/`api_endpoint`/`configuration` across modules; all validate.                                                                                                                                                                                                                                                                                                                                              | success, latency, tool calls, tokens                      |
| EV-020 | US-003         | Install a module from GitHub via a subdir source and author its type.            | Agent runs `quoin plugin install github:owner/repo//subdir@ref` (real network clone), then authors and validates one of its types.                                                                                                                                                                                                                                                                                                           | success, latency, tool calls, tokens                      |
| EV-021 | US-001         | Start a new spec from a settled idea (greenfield).                               | Agent creates `spec.md` plus ≥1 US and ≥1 FR **as discrete files**; all validate. (`artifacts` check)                                                                                                                                                                                                                                                                                                                                        | success, latency, tool calls, tokens                      |
| EV-022 | US-001         | Add one user story to an existing spec.                                          | Agent adds a new `US` file and creates **no** FR/NFR/IT artifacts; all validate. (`artifacts` require + absent)                                                                                                                                                                                                                                                                                                                              | success, latency, tool calls                              |
| EV-023 | US-002         | Edit an existing FR in place.                                                    | Agent modifies the FR file only, creating no new artifact types; the FR validates. (`artifacts` require + absent)                                                                                                                                                                                                                                                                                                                            | success, latency, tool calls, validation attempts         |
| EV-024 | US-001         | Add a user story **and** the FR that implements it.                              | Agent authors **both** a `US` and an `FR` artifact (FR traced to the US); all validate. (`artifacts` require)                                                                                                                                                                                                                                                                                                                                | success, latency, tool calls, tokens                      |
| EV-025 | US-001         | Backport a spec from a small source file.                                        | Agent authors ≥1 `FR` artifact under `spec/functional/` (not just a `spec.md` table) capturing the code's behavior; all validate.                                                                                                                                                                                                                                                                                                            | success, latency, tool calls, tokens                      |
| EV-026 | US-005         | Run a subset spec-review producing per-analysis SpecReview docs.                 | Agent runs a `subset` spec-review (integrity + dependency) and authors one **validated** `SpecReview` doc per selected analysis under `spec/reviews/` (Summary + Findings table, Severity enum). (`files` + `validate`)                                                                                                                                                                                                                      | success, latency, tool calls, tokens, validation attempts |
| EV-027 | US-008         | Create an implementation plan as a multi-plan bundle via spec-to-plan.           | Agent uses spec-to-plan to author a plan bundle `plan/<Plan-id>-<slug>/` — a `type: Plan` plan.md, reserved index.md/log.md, and ≥3 `type: Task` files whose `relationships:` carry the DAG (`depends_on`) + `references`/`verifies`; the whole bundle validates. plan.md follows the lean shape — a single `Task File Mapping` table, no separate `Execution Tracks` section. (`artifacts` require + `files` + `validate` + `fileContains`) | success, latency, tool calls, tokens, validation attempts |
| EV-028 | US-008         | Start a second, independent plan in a project that already has one.              | Given an existing `plan/Plan-001-*/` bundle, the agent uses spec-to-plan and starts a NEW plan (Step 0 selection), producing a `plan/Plan-002-*/` bundle (`type: Plan` + `type: Task` files) without disturbing Plan-001; both validate. (`artifacts` Plan≥2 + `files` + `validate`)                                                                                                                                                         | success, latency, tool calls, tokens, validation attempts |
| EV-029 | US-008         | Regenerate an existing plan after a spec change (update in place).               | Given a `plan/PLAN-001-core/` bundle covering FR-001 and a newly-added FR-002, the agent uses spec-to-plan to UPDATE the same plan — adds a task for FR-002 and refreshes index/log — without spawning a second plan; the bundle still validates. (`artifacts` Plan=1 & Task≥3 + `absentFiles` no Plan-002 + `validate`)                                                                                                                     | success, latency, tool calls, tokens, validation attempts |
| EV-030 | US-005         | Gap-analysis SAD (combined) → FAIL verdict.                                      | Given a plan with an incomplete task, a matrix TC with no backing tagged test, AND an untraced function, the agent runs gap-analysis and authors ONE validated `SpecReview` (`analysis: gap-analysis`) whose Verdict is **FAIL** and whose Findings name the incomplete task + unbacked TC. (`agentRan` + `artifacts` + `validate` + `fileContains` FAIL)                                                                                    | success, latency, tool calls, tokens, validation attempts |
| EV-031 | US-005         | Gap-analysis HAPPY → PASS verdict.                                               | Given a clean plan (all tasks done, every matrix TC backed by a real tagged test, no untraced code), the agent authors a validated `SpecReview` with Verdict **PASS** (no FAIL/CONDITIONAL). Proves the clean-pass branch still validates. (`fileContains` PASS + excludes)                                                                                                                                                                  | success, latency, tool calls, tokens, validation attempts |
| EV-032 | US-005         | Gap-analysis SAD (medium-only) → CONDITIONAL verdict.                            | Plan done and matrix fully backed, but the source has an untraced read-only `listCodes` API with no owning requirement. The review flags `listCodes` and is NOT a clean PASS — isolating the reverse-gap (Step 4) path + CONDITIONAL gate. (`fileContains` listCodes, excludes PASS)                                                                                                                                                         | success, latency, tool calls, tokens, validation attempts |
| EV-033 | US-005         | Gap-analysis OPTIONAL semantic review catches a hollow test.                     | Matrix TC-002 is backed by a tagged test so Step 3 passes — but that test asserts nothing. Only the opted-in semantic review (Step 5) catches it; the review flags TC-002 and is not a clean PASS. (`fileContains` TC-002 + hollow indicator, excludes PASS)                                                                                                                                                                                 | success, latency, tool calls, tokens, validation attempts |
| EV-040 | US-001         | Author an FR whose requirement statements are EARS-clean from the start.         | Agent authors `spec/functional/FR-100.md` and revises against `quire validate --summary` until grammar-clean; the FR passes `quire validate --strict` (structurally valid AND zero EARS findings). Exercises the EARS skeleton guidance + `/specify`. (`files` + `validate` strict)                                                                                                                                                          | success, latency, tool calls, tokens, validation attempts |
| EV-041 | US-004         | Repair a seeded FR that trips the EARS requirement-grammar check.                | A structurally-valid FR seeded with three EARS defects (non-singular + vague response + non-canonical trigger). Agent reads the `[ears:…]` warnings and rewrites the Description until `quire validate --strict` passes (grammar-clean). The EARS analogue of the EV-008 repair loop. (`files` + `validate` strict)                                                                                                                          | success, latency, tool calls, tokens, validation attempts |
| EV-042 | US-001         | Author an FR using a project term + define it in a domain's Ubiquitous Language. | Agent authors `FR-100` ("shall provide a Sprocket") AND a `domain` object whose `## Ubiquitous Language` defines Sprocket. `quire validate --strict` passes only because the harvested project term makes the otherwise-vague object concrete (FR-044 project Ubiquitous-Language). (`files` + `validate` strict)                                                                                                                            | success, latency, tool calls, tokens, validation attempts |
| EV-043 | US-004         | Repair a flagged project term by DEFINING it (not rewording).                    | A seeded FR trips the grammar check on the project term `Sprocket`. The agent resolves it by adding the term to a `domain` object's `## Ubiquitous Language` — leaving the requirement wording intact — until `quire validate --strict` passes. Exercises the FR-044 harvest+inject repair path. (`files` + `validate` strict)                                                                                                               | success, latency, tool calls, tokens, validation attempts |
| EV-050 | US-011         | Generate property tests from settled criteria.                                   | Given a fixture repo with fast-check installed and an FR whose criteria mix `extractable` and `example`, the agent runs spec-correctness and writes property tests under `tests/props/` for the extractable criteria only. Every emitted tag names a `row_id` present in `quire properties --json`; no unattended test exists for an `example` or `unclassified` criterion. (`files` + `fileContains` row ids + `absentFiles`)                | success, latency, tool calls, tokens                      |
| EV-051 | US-011         | Queue a candidate criterion, then accept it.                                     | Given a criterion classified `extraction: candidate`, the agent writes its test under `tests/props/_review/` carrying the harness skip marker, and the suite reports it skipped rather than passing. On acceptance the skip is removed and the file moved, and the `Trace:`/`row=` tags are byte-identical before and after. (`files` + `fileContains` skip marker + tag equality)                                                            | success, latency, tool calls, tokens, validation attempts |
| EV-052 | US-011, US-005 | Reconcile generated tests through gap-analysis.                                  | After spec-correctness emits tests, the agent runs gap-analysis over the same repo and its matrix verification finds every emitted `row_id` in the tag index — no unbacked row for a generated `Property` test, and no `✅` row backed by a skipped queued test. (`agentRan` + `artifacts` SpecReview + `fileContains` row ids)                                                                                                               | success, latency, tool calls, tokens, validation attempts |
| EV-053 | US-011         | Refuse to ground, and report without a verdict.                                  | Given an FR whose criteria include an adjectival oracle ("actionable") and a symbol absent from the source, and a repo whose manifest names no generator library, the agent writes no test for the ungroundable criteria, records each refusal reason and the dependency remedy in `tests/props/QUEUE.md`, and installs nothing. The report contains no threshold, verdict, grade, or rewording suggestion. (`files` + `fileContains` reasons + excludes) | success, latency, tool calls, tokens                      |

> EV-042..EV-043 exercise the **project Ubiquitous-Language** layer (quire-rs
> FR-044): a repo's own authored terms — here a `domain` object's `## Ubiquitous
Language` (the harvester also reads a dedicated `Glossary` `## Terms` table) —
> are harvested and accepted as concrete in its grammar check. EV-042 authors
> with the definition present, EV-043 repairs a flagged project term by defining
> it (not rewording). The DDD UL form keeps these evals release-decoupled — they
> need only the FR-044 engine, not the unreleased `Glossary` archetype.

> EV-050..EV-053 exercise the **`spec-correctness`** skill
> ([FR-028](./functional/FR-028-generate-property-tests-from-criteria.md)), which
> consumes `quire properties --json` (quire-rs FR-052) and generates property
> tests. EV-050 covers the unattended lane, EV-051 the review-gated lane and the
> byte-identical tag through acceptance, EV-052 the handoff to `gap-analysis`,
> EV-053 the refusals and the CON-1 no-verdict rule. These four are **defined but
> not yet implemented** in `evals/scenarios/` — until they are, FR-028 and US-011
> are marked spec-ahead-of-coverage in `spec/matrix.md`.

> EV-040..EV-041 exercise the **EARS requirement-grammar** check (quire-rs
> FR-042): EV-040 authors EARS-clean, EV-041 repairs an EARS-dirty FR. Both
> assert via `validate: { strict: true }` — `--strict` escalates the advisory
> EARS warnings to a failing exit, so a clean pass means structurally valid AND
> grammar-clean.

> EV-016..EV-018 and EV-020 extend the original EV-001..EV-015 set (multi-repo,
> larger spec-set, multi-module, real GitHub subdir install). EV-021..EV-025 are
> the **artifact-completeness** set: they drive `/specify` for spec-change request
> shapes (new project, add US, edit FR, add US+FR, backport) and assert via the
> `artifacts` check that the request produced the exact artifact **types** as
> discrete files — guarding against requirements authored only as `spec.md` table
> rows. They live alongside the rest in `evals/scenarios/index.mjs`.

## Harness Requirements

The executable harness SHOULD reuse the existing eval layers from
`ix-spec-workflows` and `ix-agent-skills/packages/workflow-evals`:

- static scenario validation for `scenario.yaml`, `user-turns.yaml`, and
  `expect.yaml` shape;
- deterministic pipeline evals for Quire/catalog behavior that can run without
  an LLM;
- opt-in agent evals through `workflow-evals` and `agent-pty` for real
  Claude/Codex sessions in tmux.

The eval harness SHOULD run scenarios in disposable repositories with isolated
`IX_HOME` values. Each run SHOULD record command transcript, wall-clock timing,
agent token usage, and tool-call count in a machine-readable result file. Agent
evals that invoke Claude/Codex require explicit operator approval because they
may send workspace context to an external model provider.

The harness SHOULD store per-scenario fixtures for:

- empty repo with no spec files;
- repo with existing artifact/object files;
- local plugin module fixture;
- GitHub/package plugin fixture or mocked install source;
- sibling development module fixture;
- changed-file manifest for validation-only scenarios.

## Current Harness Inventory

`quoin` includes an executable deterministic eval runner at `evals/run.mjs`.
`pnpm run test:evals` builds the CLI, runs EV-001 through EV-015 in disposable
repositories with isolated `IX_HOME` values, and writes the latest metrics to
`evals/reports/latest.json`.

The deterministic runner records latency, tool-call count, validation attempts,
context fetches, command transcripts, and token fields. Token usage is `null`
for deterministic runs because no LLM is invoked.

The reference `agent-pty`/tmux runner pattern remains available in
`ix-agent-skills/packages/workflow-evals` for a future opt-in agent layer. Real
Claude/Codex evals require explicit operator approval because they may send
workspace context to an external model provider.

## Release Gate

Before a stable release, every eval in this matrix SHOULD pass on the target
agent runners. Any scenario without automated token/tool-call instrumentation
MUST still run the functional command transcript and record latency manually.
