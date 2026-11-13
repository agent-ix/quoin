---
id: SR-20261113181010
title: Gap analysis of typed mapping discharge facts
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quoin; rust/crates/quoin-cli/src/evidence.rs, rust/crates/quoin-cli/src/evidence/mapping_facts.rs,
  rust/crates/quoin-cli/src/tests.rs, rust/crates/quoin-cli/tests/tc_173_facts_from_mappings.rs,
  rust/crates/quoin-quire/src/records.rs, spec/functional/FR-173-facts-from-mappings.md,
  spec/functional/FR-174-per-clause-compliance-from-mappings.md, spec/non-functional/NFR-038-mapping-facts-state-and-never-decide.md
review_set: subset
relationships:
- target: ix://agent-ix/quoin/FR-173
  type: references
- target: ix://agent-ix/quoin/FR-174
  type: references
---

## Summary

Ticket: PLAT-1525. Reviewed the frozen mapping-to-discharge implementation and its verification claims. The installed-pack criterion is untagged and the content-exclusion criterion is only partially checked.

## Verdict

**FAIL** — one acceptance criterion has no tagged test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The installed synthetic-pack chain has no verification test. quire trace --scope . --id FR-174-AC-4 returned resolved:false, Claims (0), Citations (0). World creates an empty temporary IX_HOME and supplies a prebuilt binding JSON plus IX_FILAMENT_MODULES_PATH; it never installs a synthetic pack. Add the required installed-pack flow and bind its test to FR-174-AC-4. | rust/crates/quoin-cli/tests/tc_173_facts_from_mappings.rs:94 |
| FND-002 | medium | The sole test claiming FR-174-AC-3 checks expected row substrings and absence of %. Its synthetic binding contains no clause-text sentinel; it does not inspect intermediate files or reject a compliant-row count. A leak of clause text into stored facts, or a numeric summary without %, would pass. Exercise the sentinel-bearing input and assert all three promised outputs/prohibitions. | rust/crates/quoin-cli/tests/tc_173_facts_from_mappings.rs:483 |

## Coverage

The review is limited to the five files changed by origin/main...HEAD, at commit 1ffe671abf9d78c3221b8482e8eeeb417d74176d. All five were inspected; requirements FR-173, FR-174 and NFR-038 supply 18 examined criteria. FR-133 and FR-046 were context only. The new command, flags, corpus projection, fact construction, read-only admission and stale reporting all have governing requirements; no additional unowned behavior or source stub was found. The no-op writes in DryRun implement the explicitly required read-only proposal, rather than standing in for production persistence.

Reconciliation: quire trace --scope . --id FR-173 --prefix --json; equivalent FR-174 and NFR-038 queries, plus exact FR-174-AC-4 lookup. All returned verification claims have structural confidence. 17 of 18 criteria have verification claims; one is untagged. Citations were not counted as verification. quoin matrix was not used, as directed because of PLAT-1672. No stored execution evidence was reconciled.

Plan completion: not assessed

Optional semantic gap-analysis: skipped (not requested); ordinary code-review checks of assertions against their claimed criteria ran.

No applicable AssuranceProfile: AP-201 governs finding-quality measurement and comparison; AP-202 governs live semantic-lens evaluation. Neither applies to this mapping-to-discharge adapter. No spec or plan file changed, so spec-review and its sub-analyses were skipped. Python, React and Tokio lanes do not apply.

Prior concerns checked: control ids in evidenceRefs, delegated-only scope exclusions, and unused mappings to another set match the existing FR-173 contract. Absence of Mappings is accepted by both the updated module validator and the command, so this is not a newly demonstrated Quoin validation bypass. Ambient module discovery follows the implemented command contract; an explicit --module option is not specified. These are distinct from the reproduced missing-id/type refusal defect.

Ticket reconciliation remains necessary: PLAT-1525 asks for every mapping to resolve against a loaded set, cited control evidence, and delegated/accepted_risk/temporary_exception dispositions with owner and approval. FR-173 deliberately limits validation to the supplied binding set, emits control ids as evidence, and derives only delegated dispositions from Responsible plus a command-level approval. The implementation follows that narrower existing spec; this review does not silently treat the broader ticket acceptance as fulfilled or authorize changing human risk decisions.

## Validation

Checks: the 13 new CLI integration tests passed with SPEC_OBJECTS_SECURITY_DIR=/home/peter/dev/worktrees/spec-objects-security/spec_objects_security (upstream checkout 99f418a). quoin-quire tests: 48 passed. cargo fmt --all --check passed. Targeted Clippy for quoin-cli and quoin-quire, all targets/features with warnings denied, passed. cargo deny check passed after authorized access to the Cargo advisory database lock. Initial integration run against /home/peter/dev/spec-objects-security/spec_objects_security failed 13/13 because scope_exclusion.md was absent, then the corrected dependency path passed. Targeted missing-id/type probes ran against the newly built candidate; the healthy control passed both tools, and each malformed control passed Quoin but failed Quire. Removing Mappings passed both.

All build/test/lint commands used /home/peter/dev/worktrees/locked-build.sh, /home/peter/dev/worktrees/plat-1525-quoin/rust/target, CARGO_PROFILE_DEV_DEBUG=0 and four or fewer jobs. The aggregate gate was not run, per the reviewer brief. Tool identities: installed quoin 0.30.1; quire 0.44.0 (engine 0.59.1); candidate links quire-rs 0.59.0. SpecReview schema/template: spec-artifacts-process@v0.31.0.

## Examined Units

```yaml
scope:
- id: FR-173-AC-1
  path: spec/functional/FR-173-facts-from-mappings.md
  role: examined
  excerpt: Over a synthetic set `example.invalid/std@1` with two binding clauses and
    a corpus where a control with `## Effectiveness` `effective` maps clause C1, stdout
    holds one `direct` fact for C1 with that control's id in `evidenceRefs` and an
    `attestation` equal to the supplied object.
- id: FR-173-AC-2
  path: spec/functional/FR-173-facts-from-mappings.md
  role: examined
  excerpt: A clause mapped only by a control that is `not_assessed`, `ineffective`,
    `partially_effective` or has no `## Effectiveness` section yields no fact and
    an `open` line naming the control ids.
- id: FR-173-AC-3
  path: spec/functional/FR-173-facts-from-mappings.md
  role: examined
  excerpt: A mapping to a non-binding clause and one to another set yield no fact
    and an `unused` line each.
- id: FR-173-AC-4
  path: spec/functional/FR-173-facts-from-mappings.md
  role: examined
  excerpt: 'A binding clause with a `scope_exclusion` and `--approval-ref X` yields
    a `delegated` disposition fact with `approvalRef` X and a `rationale` of the form
    `<rationale> (responsible: <responsible>)`; without `--approval-ref` it yields
    none and a `needs-approval` line.'
- id: FR-173-AC-5
  path: spec/functional/FR-173-facts-from-mappings.md
  role: examined
  excerpt: Each refusal case above, namely a missing attestation, an attestation with
    an extra field, an invalid corpus, an unknown clause in the binding report's set,
    a duplicate row in one control, a duplicate exclusion, a mapped-and-excluded clause
    and an identifier that folds onto another, exits non-zero with empty stdout and
    a message naming the control or exclusion, set and clause.
- id: FR-173-AC-6
  path: spec/functional/FR-173-facts-from-mappings.md
  role: examined
  excerpt: Two runs over identical inputs, with the corpus files in different directory
    order, produce byte-identical stdout and stderr.
- id: FR-173-AC-7
  path: spec/functional/FR-173-facts-from-mappings.md
  role: examined
  excerpt: The stdout of a successful run is accepted by `quoin evidence record-discharge
    --facts -` against the same binding report.
- id: FR-173-AC-8
  path: spec/functional/FR-173-facts-from-mappings.md
  role: examined
  excerpt: 'A run writes no file: a listing of the working directory, `--spec` and
    `--repo` is identical before and after.'
- id: FR-173-AC-9
  path: spec/functional/FR-173-facts-from-mappings.md
  role: examined
  excerpt: With `--repo` holding a stored `direct` fact for clause C1 and a corpus
    in which C1's only control is now `ineffective`, stdout has no fact for C1 and
    stderr has `stale C1`.
- id: FR-174-AC-1
  path: spec/functional/FR-174-per-clause-compliance-from-mappings.md
  role: examined
  excerpt: Over a synthetic set with three binding clauses, one answered by an effective
    control, one excluded with a recorded approval, and one answered only by an `ineffective`
    control, `quoin discharge --repo --rmm` shows `Compliant` with the control id,
    `Tailored`, and `Not compliant - undischarged` respectively.
- id: FR-174-AC-2
  path: spec/functional/FR-174-per-clause-compliance-from-mappings.md
  role: examined
  excerpt: After the expiry instant given as `--as-of`, the first clause renders `Not
    compliant - attestation not current`.
- id: FR-174-AC-3
  path: spec/functional/FR-174-per-clause-compliance-from-mappings.md
  role: examined
  excerpt: The rendered matrix and the chain's intermediate files contain none of
    the set's clause-text sentinel and no percentage or count of compliant rows.
- id: FR-174-AC-4
  path: spec/functional/FR-174-per-clause-compliance-from-mappings.md
  role: examined
  excerpt: The same chain against a synthetic pack installed in a temp `IX_HOME` renders
    the same states by clause id.
- id: FR-174-AC-5
  path: spec/functional/FR-174-per-clause-compliance-from-mappings.md
  role: examined
  excerpt: After the control of a recorded `Compliant` clause is changed to `ineffective`
    and the chain is re-run, the clause still renders `Compliant` and FR-173 reports
    `stale` for it; after its fact file is removed it renders `Not compliant - undischarged`.
- id: NFR-038-AC-1
  path: spec/non-functional/NFR-038-mapping-facts-state-and-never-decide.md
  role: examined
  excerpt: The `attestation` of every emitted fact equals the supplied object, and
    a missing or extended one is refused (FR-173-AC-1, FR-173-AC-5).
- id: NFR-038-AC-2
  path: spec/non-functional/NFR-038-mapping-facts-state-and-never-decide.md
  role: examined
  excerpt: No fact is emitted for a clause whose controls are not `effective` (FR-173-AC-2).
- id: NFR-038-AC-3
  path: spec/non-functional/NFR-038-mapping-facts-state-and-never-decide.md
  role: examined
  excerpt: The output holds no `effectiveness` key, and each control's `effective`
    comes from its own `## Effectiveness` section (FR-173-AC-1, FR-173-AC-2).
- id: NFR-038-AC-4
  path: spec/non-functional/NFR-038-mapping-facts-state-and-never-decide.md
  role: examined
  excerpt: A run writes no file (FR-173-AC-8).
- id: FR-133
  path: spec/functional/FR-133-discharge-facts-in-the-evidence-store.md
  role: context_only
  excerpt: Discharge facts in the evidence store and inward-completeness report
- id: FR-046
  path: spec/functional/FR-046-clause-discharge.md
  role: context_only
  excerpt: Explicit clause discharge accounting
```
