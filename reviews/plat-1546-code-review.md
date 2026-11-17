---
id: SR-20261116075209
title: "Code review — quoin PLAT-1546 threat-model dossier generator"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/src/dossier.rs, rust/crates/quoin-dossier/src/{gap.rs,lib.rs,sources.rs,threat_model.rs}, rust/crates/quoin-quire/src/{flow_report.rs,lib.rs}, rust/crates/quoin-dossier/tests/"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-202", type: references }
---

## Summary

Reviewed PR #960 at `82b794821277dd6b9267f61ba1f20e3f2d7775bb` against `origin/main`, including the Rust implementation, its tests, FR-202, the changed dossier registration, and workspace conventions. The generator has a model-attribution defect, and its AC-14 trace tag overstates what its test exercises.

## Verdict

**CONDITIONAL** — two medium findings remain; the separate gap-analysis artifact records the matrix coverage failure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Treatment findings are attributed to models that do not consider the threat | rust/crates/quoin-dossier/src/threat_model.rs:326-330 |
| FND-002 | medium | The scope-order test is tagged for AC-14 without exercising AC-14's findings and global-gap ordering | rust/crates/quoin-dossier/src/threat_model.rs:942 |

## Coverage

- Changed crates: `quoin-quire`, `quoin-dossier`, and `quoin-cli`; no applicable AssuranceProfile was found under `spec/`.
- Gates: `cargo fmt --all --check` passed. The independent workspace Clippy run was stopped during cold compilation in `quoin-jev`, before reaching the changed crates; the chained deny and workspace test steps therefore did not run. The PR description records changed-crate Clippy and dossier tests passing through `locked-build.sh` (8 focused tests; 101 unit and 17 integration tests, 3 ignored).
- Spec check: `quire validate --scope . spec/functional/FR-202-dossier-threat-model-content.md` exited 0 with the existing `semantic.unused-import` warning.
- Trace matrix details are in the separate gap-analysis review SR-20261116075210.
