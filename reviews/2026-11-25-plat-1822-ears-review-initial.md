---
id: SR-967175026302197768900474181941935084789
title: "spec-review/ears-conformance review of module config root"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; spec/functional/FR-004-resolve-config-root.md; rust/crates/quoin-cli/src/core_runtime.rs; rust/crates/quoin-cli/tests/tc_ix_config_root.rs"
review_set: subset
---

## Summary

Ticket: PLAT-1822. Reviewed PR #1081 at 4f71eaa3f0090a6c713a48308e5bb4f2edbc9b23. Tools: quoin 0.36.8, quire 0.52.0 (engine 0.62.2), spec-artifacts-process v0.36.0.

## Verdict

CONDITIONAL — Explicit findings require disposition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The changed precedence statement repeats SHALL four times, and quire validate reports ears:non-singular. Express the ordered selection as one SHALL obligation with an ordered list, preserving the explicit precedence. | spec/functional/FR-004-resolve-config-root.md:32-35 |

## Scope

```yaml
scope:
- id: FR-004-AC-1
  path: spec/functional/FR-004-resolve-config-root.md
  role: examined
  excerpt: '`--config-root` selects the config root for the run'
- id: FR-004-AC-2
  path: spec/functional/FR-004-resolve-config-root.md
  role: examined
  excerpt: With no `--config-root`, the root falls back to `IX_CONFIG_ROOT`, then
    `IX_HOME`, then to `~/.ix`
- id: FR-004-AC-3
  path: spec/functional/FR-004-resolve-config-root.md
  role: examined
  excerpt: '`--no-project-config` disables the project config root'
- id: FR-004-AC-4
  path: spec/functional/FR-004-resolve-config-root.md
  role: examined
  excerpt: '`IX_CONFIG_ROOT` selects the module home for `quoin module list` and `quoin
    module ensure-defaults`, ahead of `IX_HOME`'
- id: FR-004
  path: spec/functional/FR-004-resolve-config-root.md
  role: examined
  excerpt: 'The CLI SHALL resolve a config root from the `--config-root` flag or the

    `IX_CONFIG_ROOT` environment variable, defaulting to `~/.ix` (via the `IX_HOME`

    environment variable), export the resolved root to

    `IX_HOME`, and configure the shared `ix-cli-core` runtime context with the `ix`

    namespace and a project config root at `<cwd>/.ix`, treating project config as

    enabled unless `--no-project-config` is given.'
```

## Coverage

Plan completion: not assessed

AP-201 (finding scoring/measurement) and AP-202 (live Jev evaluation) were read and do not apply to module-home selection. No enforced review run is required. Full diff inspected; no dependencies or CI gates changed. settings resolves explicit config first, filters empty IX_CONFIG_ROOT, then delegates IX_HOME/default to HostCatalog, HostModules and HostSemantic. Invocation parsing and config::context were inspected as related routes. New test uses the real subprocess and registry fixtures; no source/test stubs or duplicated fixture implementation were found (the existing install_defaults invokes network installation and has different behavior). implements tags resolve for AC2/AC4.

Static reconciliation: quire matrix; no run evidence read. Whole-repository census: 1750 tagged, 636 untagged, 175 method-without-symbol, 36 retired, 13 tagged-by-ignored-test. FR-004 AC2/AC4 are tagged; AC1/AC3 remain untagged as already documented under PLAT-1045. These preexisting outside-diff gaps are context, not new PR findings. The PR-bounded reverse-gap inventory covers the environment selector and both module commands, owned by FR-004; no new unowned behavior or hollow implementation was found. This is not a whole-repository assurance PASS. Optional semantic gap audit was not requested and was skipped.

Changed-file rustfmt passes. Full cargo fmt exits 1 for untouched tc_1817_suite004_record.rs. Focused cargo test was submitted through locked-build.sh and is pending resource admission. Broad clippy, crate/workspace tests and cargo-deny were not run because PLAT-1822 explicitly requires focused tests only.
