---
id: REV-001
title: quoin Spec Review
type: Review
---

# quoin Spec Review

## Scope

Review of the spec **overhaul** — a complete, code-faithful backport of
`src/` (`cli`, `catalog`, `write`, `plugins`, `modules`, `flows`). The prior
coarse FR-001…FR-011 set was superseded by a capability-grouped FR-001…FR-021 /
NFR-001…NFR-008 set (NFR-007/008 added during this review); user stories were
re-traced and US-006 (duplicate detection) was added; the test matrix was
regenerated against `tests/`.

## Automated Checks

- **ID format & sequence:** `FR-001…FR-021`, `NFR-001…NFR-006`, `US-001…US-006`
  — all 3-digit, sequential, no gaps or duplicates. No malformed IDs (no `FR-1`
  / `US-12` forms).
- **Link integrity:** every `US → FR` `traces_to` target (FR-010/011/012/013/
  014/015/017/018/019/020/021) resolves to a declared FR.
- **Quire validation:** `quire validate --scope . "spec/**/*.md"` exits 0.
- **Test suite:** `make test` → 99 passed (9 files); `pnpm run test:coverage`
  passes the 100% gate.

## Checklist Results

- ✅ ID format/uniqueness — pass (see Automated Checks).
- ✅ US "As a / I want / So that" — all six now use the bold-marker shape the
  `spec-artifacts-iso` US archetype asserts (US-002…US-005 were prose, fixed).
- ✅ FR clarity/specificity — each FR names concrete inputs, outputs, and
  observable behavior (flags, exit codes, file paths, error text).
- ✅ FR → behavior verifiable — every FR/NFR maps to a named `tests/` case,
  found by the computed matrix (`quoin matrix`).
- ✅ Error paths documented — unknown command/subcommand, missing repo_dir,
  unknown type, missing/empty manifest name, non-zero/signal ix-flow exit,
  spawn error.
- ⚠️ US acceptance criteria — only US-001 and US-006 carry Given/When/Then
  examples; US-002…US-005 do not. Accepted: the ISO US archetype treats
  acceptance examples as _illustrative, non-normative_, and verification lives
  in the matrix, not in the US. Noted for completeness, non-blocking.

## Findings

1. **Coverage gap (FR-017) — RESOLVED.** `cli.test.ts` :: "ensure-defaults
   reports installed plugin names from a non-empty registry" now exercises the
   `(p) => p.name` mapping at `cli.ts:322`. `pnpm run test:coverage` passes at
   100% (branches/functions/lines/statements), 99 tests.

2. **Framework FRs are intentionally not US-traced.** FR-001…FR-009 and FR-016
   describe CLI-framework and catalog-assembly mechanics (arg grammar, version,
   help, config root, module-root location, dedup, manifest parsing, the
   committed default set). They are enabling requirements with no standalone
   user story and are covered directly by the matrix → `tests/`. This is an
   accepted structure, not a traceability gap.

3. **Scope boundaries are explicit and match code.** Validation execution
   (`quire`), workflow lifecycle (`ix-flow`), and install/registry mechanics
   (`ts-plugin-kit`) are stated Out of Scope and are invoked as external
   processes / delegated calls — consistent with `flows.ts` (spawns `ix-flow`),
   `write.ts` (emits, does not run, the `quire` command), and `plugins.ts`
   (delegates to `ts-plugin-kit`). NFR-004 captures the standalone-dependency
   boundary.

4. **`package:`/npm source is forward-declared.** FR-018 and US-003 correctly
   state that `package:` maps to an npm source that `ts-plugin-kit` rejects at
   install time; quoin only maps the argument. Behavior matches
   `plugins.ts:parseSourceArg`.

5. **Doc drift (out of spec scope).** Repo `CLAUDE.md` still says
   `make test # run jest`; the project runs vitest. Flagged for a follow-up
   doc fix; no spec change required.

## Analysis Lenses (spec-integrity + spec-failure-domain)

Both lenses from the `spec-review` process were run against `spec.md` + `src/`:

**Integrity (completeness / consistency / atomicity):**

- ✅ Multi-source lookup tie-break is an explicit decision — FR-008 (first-wins
  by module name / resolved root) and FR-012 (duplicate type detection).
- ✅ Forward-declared dependency (`package:`/npm) documents its interim
  behavior — FR-018 (rejected by ts-plugin-kit at install time).
- ➕ **External-CLI probe (fixed):** FRs delegate to `ix-flow`/`quire` with no
  invocation contract. Added **NFR-007** (PATH resolution, no version pin in
  no version pin, absence/non-zero surfaces as non-zero exit, `quire` emitted not run).
- ⚠️ **No StR layer (accepted).** Stakeholder intent is not derivable from code;
  reverse-engineering an StR set would be fabrication. User stories carry the
  stakeholder framing informally. A real StR layer is deferred to a phase with
  product input — not a code-backport task.
- ⚠️ **Atomicity (accepted).** FR-011 (list/show), FR-012 (detect/validate),
  FR-019 (install/list/remove), and FR-021 (spawn/exit/handoff) each bundle a
  small command family. Kept grouped for readability; each sub-behavior still
  has its own named test. Splitting is a future option.

**Failure domain (extension / identity / purity / topology):**

- ✅ Entity identity keys are all explicit — module by `name`, catalog entry by
  `kind:name`, module root by resolved path, registry plugin by `name`.
- ✅ Topology/purity: catalog assembly is a flat, one-level-deep filesystem scan
  with guaranteed termination and no user-evaluated logic — no cycle or
  deep-nesting exposure.
- ➕ **Extension-point failure policy (fixed + flagged):** a corrupt installed
  `manifest.yaml` aborts _all_ catalog operations (strict), while a missing one
  is skipped. Captured as **NFR-008**. Open design question: strict-abort means
  one bad module blocks even `catalog list`; a resilient skip-and-warn may be
  better UX. Flagged for a product decision; current behavior documented as-is.

## Findings (analysis)

6. **NFR-008 strict-abort — RESOLVED.** `catalog.test.ts` :: "aborts (strict)
   on a present but unparseable manifest.yaml" now asserts the throw, alongside
   the existing missing-manifest skip test.
7. **External-tool versions unpinned (NFR-007) — known limitation.** `ix-flow`
   and `quire` are resolved from `PATH` with no minimum-version check. Acceptable
   for now; revisit when either tool's CLI contract changes.

## Analysis Lenses (dependency / evidence / risk / scope)

The remaining four lenses from the `spec-review` process were also run. Kept
inline here (the dependency skill permits inline analysis) rather than as
separate `spec/analysis/*.md` files, since no `analysis` archetype exists and
loose files would not validate under Quire.

### Dependency & Ordering — no cycles

Logical implementation order (enablement → feature):

1. **Root enablement:** FR-001 (arg parsing), FR-004 (config root / runtime
   context).
2. **Catalog enablement:** FR-006 (module-root location) → FR-007 (assembly
   order) → FR-008 (dedup) / FR-009 (manifest parse) / FR-010 (lookup).
3. **Module-store enablement:** FR-016 (committed default set), FR-018 (source
   grammar).
4. **Features:** FR-002/003/005 (version/help/errors), FR-011/012 (list/show/
   validate), FR-013/014/015 (write), FR-017/019 (reconcile / plugin lifecycle),
   FR-020/021 (workflow launch).

No cyclic prerequisites. (Retrospective DAG — the code already exists; this
orders a re-implementation and feeds `spec-to-plan`.)

### Verification & Evidence — no open gaps

Every FR is `test`; the computed matrix (`quoin matrix`) names the case. Non-test methods:
NFR-004 = inspection (`package.json` deps), NFR-005 = inspection
(`flows.ts` + bundled skills), NFR-006 = metric (`evals/` harness), NFR-008 =
test (skip + strict-abort paths). **Open evidence gaps:** none — the FR-017 and
NFR-008 gaps flagged during review (Findings 1 and 6) were closed with new
tests; `pnpm run test:coverage` passes at 100%.

### Risk & Complexity — no high tech risk

quoin is a low-novelty CLI: no concurrency, cryptography, or perf SLA, and is
not `security_critical`. Tech risk is low across the board. The volatility
cluster is **external-contract coupling**:

| Req        | Tech | Volatility | Driver                                                |
| ---------- | ---- | ---------- | ----------------------------------------------------- |
| FR-017/019 | Low  | Medium     | `ts-plugin-kit` install/reconcile/registry contract   |
| FR-018     | Low  | Med-High   | `package:`/npm source is forward-declared; will churn |
| FR-020/021 | Low  | Medium     | `ix-flow` CLI argument contract (unpinned, NFR-007)   |
| NFR-006    | Low  | Medium     | agent-pty evals depend on the external model provider |

**Top hazard:** FR-018's npm source — the highest-churn item — but it is
isolated in `parseSourceArg`, so the blast radius is contained.

### Scope & Boundary — external deps are all _assumed_

In scope: arg dispatch, catalog assembly, authoring-pack emission, default-set
reconcile trigger, plugin source mapping, workflow launch. External
dependencies and how they are verified **locally**:

| Dependency                | Relationship                           | Verified locally?                                                      |
| ------------------------- | -------------------------------------- | ---------------------------------------------------------------------- |
| `@agent-ix/ts-plugin-kit` | delegated (install/registry/reconcile) | Assumed — exercised via path-source fixtures, not a contract test      |
| `ix-flow`                 | spawned process                        | Assumed — tested against a **fake** `ix-flow` binary, not the real CLI |
| `quire`                   | command emitted, never run by quoin    | Assumed — only the command string is asserted                          |
| `@agent-ix/ix-cli-core`   | linked runtime context                 | Assumed                                                                |

## Findings (analysis, continued)

8. **No local contract tests against external tools (scope).** Every external
   boundary is `assumed`. `ix-flow` is covered only by a fake-binary stand-in;
   `quire`/`ts-plugin-kit` behavior is not contract-verified here (cross-repo
   contracts such as the quire-rs shared store are asserted in those repos). If
   an upstream CLI contract drifts, quoin's tests would not catch it. Low
   priority for now; candidate for an integration-test layer later.
9. **`package:`/npm source is the top churn item (risk).** FR-018 forward-
   declares npm support that `ts-plugin-kit` rejects today. Isolated in
   `parseSourceArg`; revisit when npm support lands.

## Gate

**Accepted — clean.** The spec is a faithful backport of current behavior,
validates under Quire, and every requirement traces to a real test:

- Findings 1 (FR-017) and 6 (NFR-008) — the two test gaps found during review —
  were **closed** with new tests; `pnpm run test:coverage` passes the 100% gate
  (99 tests).
- Findings 2–5, 7, 8, 9 and the StR/atomicity notes are accepted properties,
  documented limitations, or low-priority follow-ups (cross-repo contract tests,
  the forward-declared npm source) — none block the spec or a release.

All six `spec-review` analysis lenses (integrity, failure-domain, dependency,
evidence, risk, scope) were run. The spec accurately reflects `src/` as of this
review with no open action items.
