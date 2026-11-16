---
id: SR-20261116075206
title: "Gap analysis of PLAT-1622 risk management dossier"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; rust/crates/quoin-dossier; PLAT-1622; PR #951"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-199"
    type: references
  - target: "ix://agent-ix/quoin/FR-191"
    type: references
---

## Summary

Ticket: PLAT-1622. Audited the FR-199 criteria against the new generator, its tests, and structural trace claims. Production behavior is owned by FR-199; registry and gap ordering are owned by FR-191. No additional reverse code-to-spec gap or live stub was found. Plan completion: not assessed. The optional semantic review was skipped; test intent findings are recorded separately in the code-review artifact.

## Verdict

**FAIL** — FR-199-AC-8 has no test claim, and the new tests never exercise the source-failure path through the generator/engine.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test is bound to FR-199-AC-8. The new tests call private `render` with constructed `Summary` and `Advice`; none invokes `RiskManagementGenerator::generate` with an unreadable source or checks exit 3 and that no output file is created. | rust/crates/quoin-dossier/src/risk_management.rs:30-72,430-556 |

## Coverage

- FR-199-AC-1..7 each have a structural `quire trace` claim. AC-8 has no claim. The only citation in the trace result is the generator doc comment; citations are not verification evidence.
- Manual reverse-gap check: the new rendering and gap behaviors are specified by FR-199, and registry ordering/gap vocabulary are specified by FR-191. No unowned behavior or unfinished stub was found.
- Trace command: `quire trace --id FR-199 --prefix --scope . --module /home/peter/.ix/filament/modules/spec-artifacts-process --json`; exit 0, `resolved: true`, 7 structural verifying claims, zero for AC-8, one non-evidence citation. The trace report had 4,101 bytes of diagnostics on stderr from the installed model and existing symbol-index collisions; claims were structural.
- Matrix commands were attempted but not relied on: `quoin matrix --repo . --json` (Quoin 0.35.0) exited 0 with empty stderr and listed all eight FR-199 ACs as untagged; `quire matrix --scope . --format json` (Quire 0.45.0, engine 0.60.0) exited 0, listed all eight as untagged and emitted 482,029 bytes of warnings. The parent lead identified this as the known PLAT-1767 matrix limitation; no finding or workaround was based on those outputs.
- Installed module for trace was `spec-artifacts-process@v0.31.0`; `spec-objects-safety` was v0.8.0. No applicable AssuranceProfile was found.
- The reviewer independently ran `cargo fmt --all --check` and focused dossier Clippy via `/home/peter/dev/worktrees/locked-build.sh`; both passed. A reviewer test rerun did not produce a final result before the review handoff. The coder reported focused dossier tests passed (88 unit, 14 integration; four existing export tests ignored).

## Reviewer data

```yaml
clean: false
scope:
  - {id: FR-199, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "The risk management document renders quoin risk-summary rows and findings without assigning a new risk value or decision."}
  - {id: FR-199-AC-1, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "A complete hazard, situation, harm, pre/post estimates, control, requirement, verification and decided residual risk renders each source value and no gap."}
  - {id: FR-199-AC-2, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "Removing a control verification yields control_unverified naming that control; no source row is lost."}
  - {id: FR-199-AC-3, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "Removing a residual-risk decider yields residual_risk_undecided, a not stated decider cell and no invented person or accepted decision."}
  - {id: FR-199-AC-4, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "A hazard with no situation remains listed with its FR-170 finding and risk_chain_incomplete; dedicated control and residual-risk findings yield only their dedicated gap kind."}
  - {id: FR-199-AC-5, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "Every rendered policy region equals quoin risk advise --json for the same estimate; an unplaced estimate gets no region."}
  - {id: FR-199-AC-6, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "A benefit-risk entry from Advice.decision_queue appears under Undecided decisions; policy-not-found yields no_acceptability_policy and no unexposed record is invented."}
  - {id: FR-199-AC-7, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "Two runs produce byte-identical Markdown, and no generated field computes score, rank or acceptability."}
  - {id: FR-199-AC-8, path: spec/functional/FR-199-dossier-risk-management-content.md, role: examined, excerpt: "An unreadable safety record causes exit 3 with the source error and leaves no selected output file."}
  - {id: FR-191, path: spec/functional/FR-191-dossier-command.md, role: examined, excerpt: "The registry/report order is fixed; M2 includes DOS-VR, DOS-SOUP, DOS-ANOM, DOS-RISK, DOS-RMP, DOS-REL."}
  - {id: rust/crates/quoin-dossier/src/risk_management.rs, path: rust/crates/quoin-dossier/src/risk_management.rs, role: examined, excerpt: "The new generator reads summary and advice; its three tests call the private render helper with constructed inputs."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-199-AC-8
    related_ids: []
    path: rust/crates/quoin-dossier/src/risk_management.rs
    lines: "30-72,430-556"
    excerpt: |-
      fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
          ...
          if !unreadable.is_empty() {
              return Err(SourceFailure { ... });
          }
      }
      #[cfg(test)]
      mod tests {
          // tests call render(&summary, &advice)
      }
    finding: "No test claim or test exercises the unreadable-source path. The new tests construct Summary and Advice and call render directly; they do not call generate or the engine, so exit 3 and no selected output file on an unreadable safety record remain unverified."
bindings:
  - {test_id: complete_chain_renders_source_values_deterministically_without_derived_values, ac_id: FR-199-AC-1, trace: correct}
  - {test_id: complete_chain_renders_source_values_deterministically_without_derived_values, ac_id: FR-199-AC-7, trace: correct}
  - {test_id: row_findings_map_to_typed_gaps_and_keep_missing_decider_explicit, ac_id: FR-199-AC-2, trace: correct}
  - {test_id: row_findings_map_to_typed_gaps_and_keep_missing_decider_explicit, ac_id: FR-199-AC-3, trace: correct}
  - {test_id: row_findings_map_to_typed_gaps_and_keep_missing_decider_explicit, ac_id: FR-199-AC-4, trace: correct}
  - {test_id: policy_findings_and_undecided_entries_are_rendered_from_advice, ac_id: FR-199-AC-5, trace: correct}
  - {test_id: policy_findings_and_undecided_entries_are_rendered_from_advice, ac_id: FR-199-AC-6, trace: correct}
```
