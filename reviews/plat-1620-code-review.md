---
id: SR-20261116075207
title: "Code review (Rust lane) — PLAT-1620 PR #952 release record"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1620 PR #952 at c12519246d2b53da0a9f688a7c54136370f0baa2; rust/crates/quoin-cli/src/{dossier,release,release/history,release/snapshot}.rs, rust/crates/quoin-cli/tests/{tc_1614_dossier_command,tc_3604_release_snapshot}.rs, rust/crates/quoin-dossier/src/{anomalies,architecture,engine,export,gap,lib,registry,release_record,release_snapshot,risk_management,risk_plan,rmm,soup,srs,stub,testing,trace_matrix,verification_report}.rs, rust/crates/quoin-dossier/tests/{tc_1437_generators,tc_1617_verification_report,tc_1618_soup,tc_1619_anomalies,tc_1622_risk_management,tc_1656_risk_management_plan}.rs; FR-201"
review_set: subset
relationships:
  - {target: "ix://agent-ix/quoin/FR-201", type: references}
---

## Summary

Reviewed PLAT-1620 PR #952 at `c12519246d2b53da0a9f688a7c54136370f0baa2`. The Rust lane was folded into this code review. The release record is registered, reads the shared closed snapshot DTO, uses the supplied EA result without executing git, renders snapshot history in order, and appends the EA result after user approvals for export. One malformed-input path remains: the EA `reasons` array is only checked as an array, so non-string reason values pass the source parser and are rendered instead of producing FR-191 source failure.

The authoritative EA FR-047 result contract defines `reasons` as reason strings. A concrete malformed result `{"protocol":"engineering-assurance.release-verify-result/v1","status":"not_released","tag":"v1","reasons":[{}]}` passes `read_verify` because it checks only `Value::as_array`; `render_decision` serializes the object as a reason. FR-201 requires a present malformed verify file to exit 3 before writing selected documents. Quoin matrix reports FR-201 criteria as untagged (the known PLAT-1767 limitation); `quire trace --scope . --id FR-201 --prefix --json` resolves all eight criteria to Rust trace attributes and the corresponding tests, so that matrix output is not treated as a PR finding.

## Verdict

Changes requested: one medium finding. The remaining reviewed release-record behavior and Rust conventions are sound in the examined scope. No workflow gate was changed. PR-provided evidence reports the dossier crate tests and all-target/all-feature Clippy clean; this review did not rerun builds.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `read_verify` accepts malformed EA reasons because it checks only that `reasons` is an array, not that each item is a string. For a `not_released` result with `reasons: [{}]`, generation accepts the source and renders `{}` as a reason instead of returning source failure exit 3; this violates FR-201's malformed-source behavior. Validate the EA reason element types (and add a malformed verify fixture asserting exit 3 and no selected output). | rust/crates/quoin-dossier/src/release_record.rs:78-123 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-201-AC-1 through FR-201-AC-5 | examined | Rendering, source order, and snapshot-only evidence reviewed; FND-001 applies to the source validation boundary. |
| FR-201-AC-6 | examined | Snapshot malformed case is tested; malformed EA result reason members are not rejected (FND-001). |
| FR-201-AC-7 and FR-201-AC-8 | examined | Export plumbing and no-git/repeatability behavior reviewed; no finding. |
| Shared snapshot model and CLI writer refactor | examined | CLI and dossier use the same closed DTO/parser; no second parser found. |
| Dossier registry, export handoff, and pre-existing generators updated for new Inputs | examined | Registration order, module forwarding and call-site updates reviewed; no finding. |
| PR CI workflow and crate lint policy | examined | No gate narrowing found. |
| EA FR-047 result shape | context_only | Used to establish that EA reasons are strings and the released/not-released output shape. |

## Rust Review Notes

No unsafe code, production panic path, unchecked narrowing, or unintended async/blocking lifecycle was introduced in the examined diff. Snapshot DTOs reject unknown members and validate the protocol. CLI/dossier snapshot serialization now shares the same public DTOs; the release-record parser is the remaining loose wire boundary covered by FND-001. Test assertions check the generated fields, history ordering, source failures, approval argument ordering, and no-git repeatability. The PR-provided test and lint evidence was inspected; this review did not rerun the gates.
