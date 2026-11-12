---
id: SR-20261112013803
title: "Gap analysis of quoin PR 819 (PLAT-1452 NFR-007/NFR-001 cleanup)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@7ee1809f; spec/non-functional/NFR-001, NFR-007, spec/assurance/MP-252, spec/matrix.md NFR-001/NFR-007 rows, rust/crates/quoin-modules/tests/install_reconcile.rs"
review_set: subset
---

# Gap analysis of quoin PR 819

## Summary

Ticket: PLAT-1452. Plan completion: not assessed. `quoin matrix --json` (quoin 0.30.0) was run
on the PR head 7ee1809f.

- NFR-001-M-1: `tagged`, binder `install_reconcile.rs:286`
  `tc_381_283_a_settled_lazy_reconcile_performs_no_network_io`. NFR-001-M-2: `untagged`.
  NFR-007-M-1: `tagged` (TC-3004). NFR-007-M-2: absent (0 occurrences in the matrix JSON).
- No `NFR-007-M-2` reference remains anywhere in the repo, reviews/ included (grep of the tree,
  excluding target and fixtures). It does not appear in skills or plans.
- Merges cleanly with origin/main (97e5f932, which touches only `default-modules.yaml`).

**NFR-001-M-1 binding is real.** `CountingResolver` counts every `GitResolver::resolve` call.
`ModuleInstaller` has one network/git seam, `self.resolver.resolve` (install.rs:192). The
first `ensure_defaults` asserts `calls() == 1`, so the counter is live. The second asserts
`calls() == 1` and `!touched_network()`. `quoin catalog` and `quoin write` run
`modules.ensure_defaults` in `lazy` mode before each read (catalog.rs:61-74, write.rs:38-39).
So the test observes the reconcile that catalog access runs. It does not drive it through
`quoin catalog` itself. "Partly" is honest: M-2 (store mutations) is not tagged.

**Deleting NFR-007-M-2 is justified, but not because it was vacuous.** Production
`Command::new` executables are exactly `git`, `ix-flow` (`runner_program()`) and `$EDITOR`.
TC-203 enforces this. quire is never a subprocess: `quoin-quire` links `quire-rs =0.56.0`.
But read literally, the metric "quire validation runs executed by quoin rather than emitted =
0" was false, not true by construction. The workflow provider runs quire validation in process
through `quoin_quire::validate::run` (flow/provider.rs:479). The obligation the metric
expressed, "quire is emitted, not run", no longer describes the product. Removing it loses
nothing anyone relies on. The PR description's "true by construction" is wrong, but the PR's
shipped text does not repeat it.

## Verdict

Clean for the PR diff. No untested new claim. The rewritten NFR-007 Statement claims an
executable set that TC-203 enforces, and an ix-flow failure surfacing that TC-3004 enforces.
Spec-consistency fallout outside the diff is reported in the spec-review artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
