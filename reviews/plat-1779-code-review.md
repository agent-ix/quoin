---
id: SR-20261117175248
title: "Code review — PLAT-1779 Quoin module-set approval metadata"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs, rust/crates/quoin-core/src/capabilities.rs, rust/crates/quoin-core/src/ops/evidence/discharge.rs, rust/crates/quoin-core/src/ops/modules/support.rs, rust/crates/quoin-core/src/runtime.rs, rust/crates/quoin-quire/src/clauses.rs; FR-181-AC-11, FR-208-AC-1..5, PLAT-1779"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-181", type: references}
  - {target: "ix://agent-ix/quoin/FR-208", type: references}
---

## Summary

Ticket: PLAT-1779. The production path now validates approval metadata with the full installed module set and preserves policy ownership, but the regression omits required refusal and attribution assertions and carries a trace tag for a different criterion.

## Verdict

**CONDITIONAL** — two medium findings remain in the regression's acceptance coverage and trace binding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The cross-vocabulary regression does not verify the required strict-refusal and owner-attribution cases. | rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs:579 |
| FND-002 | medium | The regression is tagged to FR-208-AC-1 although it supplies a non-blank approval reference and asserts successful recording. | rust/crates/quoin-cli/tests/tc_1488_pack_requirements_report.rs:522 |
| FND-003 | medium | The new module-root helper repeats the existing runtime root-resolution sequence instead of replacing its duplicate call-site logic. | rust/crates/quoin-core/src/runtime.rs:317 |

## Review Scope

| Unit | Path | Role | Excerpt |
| --- | --- | --- | --- |
| `FR-181-AC-11` | `spec/functional/FR-181-source-profile-evaluation.md` | examined | “When a selected set's declaring module uses an edge type registered by another installed module, the set is reported as `evaluated` and no `UnknownEdgeType` failure is returned.” |
| `FR-208-AC-1` | `spec/functional/FR-208-module-specific-discharge-approval.md` | examined | “A binding clause with non-empty `approvalRoles` remains open when its direct fact has no `approvalRef`, or has an empty or whitespace-only reference.” |
| `FR-208-AC-2` | `spec/functional/FR-208-module-specific-discharge-approval.md` | examined | “A binding clause with non-empty `approvalRoles` that require a named human approver is direct only when its direct fact has a non-blank `approvalRef` identifying that person; the report retains the reference without changing the direct classification.” |
| `FR-208-AC-3` | `spec/functional/FR-208-module-specific-discharge-approval.md` | examined | “A binding clause with empty `approvalRoles` continues to accept a direct fact without `approvalRef` as specified by FR-046.” |
| `FR-208-AC-4` | `spec/functional/FR-208-module-specific-discharge-approval.md` | examined | “When multiple installed modules declare the same exact clause-set identity, discharge fails without selecting either module's approval policy.” |
| `FR-208-AC-5` | `spec/functional/FR-208-module-specific-discharge-approval.md` | context_only | “The report preserves a supplied approval reference and does not claim to authenticate the named approver, role, or authorization represented by it.” |
| `PLAT-1779-AC-1` | Linear PLAT-1779 | examined | “`quoin evidence record-discharge` evaluates a real security clause set when its `references` edge is declared by the installed process/ISO vocabulary module.” |
| `PLAT-1779-AC-2` | Linear PLAT-1779 | examined | “The selected clause set remains attributable to the module that declares it; other installed modules supply vocabulary only.” |
| `PLAT-1779-AC-3` | Linear PLAT-1779 | examined | “An unknown `allowed_links` verb and a strict module-set collision still fail with the relevant module/diagnostic.” |
| `PLAT-1779-AC-4` | Linear PLAT-1779 | examined | “A regression test reproduces the current published CLI failure using the equivalent module set and verifies success after the fix.” |
| `module_root` | `rust/crates/quoin-core/src/runtime.rs` | examined | “Locate the installed module's manifest and open a `ModuleRoot` at its parent directory.” The same sequence remains inline in `HostModules::evaluate_clause_sets`. |

## Rust Review

The module-set API loads the installed roots under Quire's strict registry rules, locates the requested root, and extracts approval roles only from that module's loaded clause sets. The callers retain exact set identity and refuse duplicate owners. No panic path, unbounded recursion, unsafe code, wire conversion, gate narrowing, or applicable AssuranceProfile issue was found in the changed source. AP-201 governs measurement/reporting surfaces and AP-202 governs live Jev lens evaluation; neither applies here.

The only new test uses real module fixtures and the production CLI path. It proves the positive cross-vocabulary load path can fail if the full installed vocabulary is not used. It does not test malformed vocabulary, collisions, or policy ownership against a competing module, and its `FR-208-AC-1` trace does not match the assertions.

The new `module_root` helper duplicates the `locate_manifest` plus `ModuleRoot::open` sequence still present in `HostModules::evaluate_clause_sets`; consolidate the existing call site through the new helper so root resolution has one implementation.

## Validation

- PR: `agent-ix/quoin#984`, frozen at `b67fa55b016ea4d2bc2bc79cf821633d62d15325`; only the diff from `origin/main` was reviewed.
- `quoin matrix --repo <review-worktree> --json`: completed with Quoin 0.36.5 / Quire 0.51.0 / engine 0.62.2. The new test binds to FR-208-AC-1, but its assertions do not exercise that criterion. FR-181-AC-11 remains bound to existing profile tests.
- GitHub CI: internal CLA and gate passed; ordinary CLA check skipped.
- `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` via `locked-build.sh`: passed.
- Focused `tc_208_005_record_discharge_reads_approval_metadata_with_the_full_module_set` via `locked-build.sh`: 1 passed. The first attempt found the documented `node_modules` schema build input missing; after `pnpm install --frozen-lockfile`, the focused test passed.
- `cargo clippy --manifest-path rust/Cargo.toml --locked -p quoin-quire -p quoin-core -p quoin-cli --all-targets --all-features -- -D warnings` via `locked-build.sh`: passed.
- `cargo deny check` from `rust/` via `locked-build.sh`: passed (`advisories ok, bans ok, licenses ok, sources ok`); it emitted the existing yanked `yoke-derive 0.8.3` warning.
