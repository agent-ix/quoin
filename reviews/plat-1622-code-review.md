---
id: SR-20261116075205
title: "Code review of PLAT-1622 risk management dossier"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-dossier; PLAT-1622; PR #951"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-199"
    type: references
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
---

## Summary

Ticket: PLAT-1622. Reviewed the changed Rust source, registration, gap vocabulary, dependency, and all FR-199 acceptance criteria. The Rust review folded into this artifact. No applicable AssuranceProfile was found: AP-201 and AP-202 scopes do not cover this document generator.

## Verdict

**FAIL** — shipped registry order violates FR-191, and several tests do not assert the complete behavior their criteria require.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The shipped generator list still puts `risk-management-plan` before `verification-report`, SOUP, anomalies and `risk-management`, contrary to FR-191's fixed order. Registry selection and reports preserve registration order, and the new registry test asserts the wrong order, so an unfiltered dossier is misordered. | rust/crates/quoin-dossier/src/lib.rs:79-85,138-150 |
| FND-002 | medium | The AC-1 test does not assert that the generated document contains the harm or post-control estimate. Removing either rendering path would leave the test green despite AC-1 requiring every source value. | rust/crates/quoin-dossier/src/risk_management.rs:432-450 |
| FND-003 | medium | The AC-4 test pairs `situation: Some("SIT-1")` with a manually supplied `HazardWithoutSituation` finding and does not assert the hazard heading. It cannot show that a source hazard with no situation remains listed with its finding. | rust/crates/quoin-dossier/src/risk_management.rs:461-474 |
| FND-004 | medium | The AC-5 test sets both `Summary.policy_region` and `Advice.PolicyCell.region` to `None`, then checks `not stated`; it never checks that a rendered region equals the advice source region for the same estimate. | rust/crates/quoin-dossier/src/risk_management.rs:507-545 |

## Coverage

- Changed files reviewed: `rust/Cargo.lock`, `rust/crates/quoin-dossier/Cargo.toml`, `rust/crates/quoin-dossier/src/gap.rs`, `rust/crates/quoin-dossier/src/lib.rs`, and `rust/crates/quoin-dossier/src/risk_management.rs`.
- No CI workflow or gate changes were present. The added dependency is the workspace's `quoin-risk-advice` crate; no source or specification was copied.
- `quire trace --id FR-199 --prefix --scope . --module /home/peter/.ix/filament/modules/spec-artifacts-process --json`: exit 0, `resolved: true`; seven structural `claims.verifies` for AC-1 through AC-7, no claim for AC-8, and one citation (not verification evidence). The claims identify the three new unit tests in `risk_management.rs`.
- `quoin matrix --repo . --json` and `quire matrix --scope . --format json` both reported FR-199 criteria as untagged. The installed matrix behavior is the known PLAT-1767 limitation; these outputs were not used as review evidence. The structural `quire trace` result above was used instead.
- The changed production code has no unsafe code, process invocation, async work, or persistence/wire format. `cell`/`line` reuse the dossier's shared Markdown escaping helpers; gap kinds are inserted in FR-191 order.
- The reviewer independently ran `cargo fmt --all --check` and focused dossier Clippy via `locked-build.sh`; both passed. A reviewer test rerun did not produce a final result before the review handoff. The coder reported `cargo test --locked -p quoin-dossier` passed (88 unit and 14 integration tests; 4 existing export tests ignored).

## Reviewer data

```yaml
clean: false
scope:
  - {id: FR-199, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "The risk management document renders the rows and findings of quoin risk-summary without assigning a new risk value or decision."}
  - {id: FR-199-AC-1, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "A complete hazard, situation, harm, pre/post estimates, control, requirement, verification and decided residual risk renders each source value and no gap."}
  - {id: FR-199-AC-2, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "Removing a control verification yields control_unverified naming that control; no source row is lost."}
  - {id: FR-199-AC-3, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "Removing a residual-risk decider yields residual_risk_undecided, a not stated decider cell and no invented person or accepted decision."}
  - {id: FR-199-AC-4, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "A hazard with no situation remains listed with its FR-170 finding and risk_chain_incomplete; dedicated control and residual-risk findings yield only their dedicated gap kind."}
  - {id: FR-199-AC-5, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "Every rendered policy region equals quoin risk advise --json for the same estimate; an unplaced estimate gets no region."}
  - {id: FR-199-AC-6, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "A benefit-risk entry from Advice.decision_queue appears under Undecided decisions; policy-not-found yields no_acceptability_policy and no unexposed record is invented."}
  - {id: FR-199-AC-7, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "Two runs produce byte-identical Markdown, and no generated field computes score, rank or acceptability."}
  - {id: FR-199-AC-8, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "An unreadable safety record causes exit 3 with the source error and leaves no selected output file."}
  - {id: FR-191, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "Registry order is the fixed bundle order and the order of every report; the M2 sequence includes DOS-VR, DOS-SOUP, DOS-ANOM, DOS-RISK, DOS-RMP, DOS-REL."}
  - {id: rust/crates/quoin-dossier/src/risk_management.rs, path: rust/crates/quoin-dossier/src/risk_management.rs, role: examined, excerpt: "The new generator reads summary and advice, renders source values and findings, and declares three FR-199-tagged unit tests."}
  - {id: rust/crates/quoin-dossier/src/lib.rs, path: rust/crates/quoin-dossier/src/lib.rs, role: examined, excerpt: "The new risk-management generator is registered after anomalies; the shipped-registry test expects risk-management-plan before verification-report."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: FR-191
    related_ids: [FR-199]
    path: rust/crates/quoin-dossier/src/lib.rs
    lines: "79-85,138-150"
    excerpt: |-
      Box::new(risk_plan::RiskPlanGenerator(sources.policy)),
      Box::new(verification_report::VerificationReportGenerator::new(
          sources.matrix,
      )),
      Box::new(soup::SoupGenerator(sources.soup)),
      Box::new(anomalies::AnomaliesGenerator),
      Box::new(risk_management::RiskManagementGenerator),
    finding: "FR-191 fixes registry/report order as DOS-VR, DOS-SOUP, DOS-ANOM, DOS-RISK, DOS-RMP. Registry selection preserves registration order, but this list places risk-management-plan before all four. The updated registry test asserts that same wrong sequence, so unfiltered generation and reports violate the fixed order."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: code-review
    check: test-intent
    artifact_id: FR-199-AC-1
    related_ids: []
    path: rust/crates/quoin-dossier/src/risk_management.rs
    lines: "432-450"
    test_id: complete_chain_renders_source_values_deterministically_without_derived_values
    excerpt: |-
      assert!(first.body.contains("## HAZ-1"));
      assert!(first.body.contains("| EST-PRE | pre_control | serious | likely | high |  |"));
      assert!(first.body.contains("| CTRL-1 | FR-9 | TC-9 |"));
      assert!(first.body.contains("| RR-1 | accepted | A. Person |"));
    finding: "The fixture contains HARM-1 and EST-POST, but the assertions omit both. Removing harm or post-control-estimate rendering would not fail this test, although AC-1 requires every source value."
  - fnd: FND-003
    severity: medium
    confidence: high
    method: code-review
    check: test-intent
    artifact_id: FR-199-AC-4
    related_ids: []
    path: rust/crates/quoin-dossier/src/risk_management.rs
    lines: "461-474"
    test_id: row_findings_map_to_typed_gaps_and_keep_missing_decider_explicit
    excerpt: |-
      let mut row = complete_row();
      row.findings = vec![RowFinding {
          finding: RowToken::HazardWithoutSituation,
          names: vec!["HAZ-1".to_owned()],
      }];
    finding: "complete_row() supplies situation Some(\"SIT-1\"), so the manually attached HazardWithoutSituation token is inconsistent with the source scenario in AC-4. The test also never asserts the output contains the hazard heading; omitting the hazard would still leave the gap assertion passing."
  - fnd: FND-004
    severity: medium
    confidence: high
    method: code-review
    check: test-intent
    artifact_id: FR-199-AC-5
    related_ids: []
    path: rust/crates/quoin-dossier/src/risk_management.rs
    lines: "507-545"
    test_id: policy_findings_and_undecided_entries_are_rendered_from_advice
    excerpt: |-
      row.pre_control[0].policy_region = None;
      ...
      region: None,
      ...
      assert!(rendered.body.contains("| EST-PRE | pre_control | serious | likely | not stated | policy-not-found |"));
    finding: "The test checks the missing-region display, not the required equality between a placed estimate's Summary.policy_region and Advice.policy_cells region. A mismatch between those source values would not be detected."
bindings:
  - {test_id: complete_chain_renders_source_values_deterministically_without_derived_values, ac_id: FR-199-AC-1, trace: correct}
  - {test_id: complete_chain_renders_source_values_deterministically_without_derived_values, ac_id: FR-199-AC-7, trace: correct}
  - {test_id: row_findings_map_to_typed_gaps_and_keep_missing_decider_explicit, ac_id: FR-199-AC-2, trace: correct}
  - {test_id: row_findings_map_to_typed_gaps_and_keep_missing_decider_explicit, ac_id: FR-199-AC-3, trace: correct}
  - {test_id: row_findings_map_to_typed_gaps_and_keep_missing_decider_explicit, ac_id: FR-199-AC-4, trace: correct}
  - {test_id: policy_findings_and_undecided_entries_are_rendered_from_advice, ac_id: FR-199-AC-5, trace: correct}
  - {test_id: policy_findings_and_undecided_entries_are_rendered_from_advice, ac_id: FR-199-AC-6, trace: correct}
```
