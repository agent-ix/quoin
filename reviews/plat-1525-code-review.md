---
id: SR-20261113181009
title: Code and Rust review of typed mapping discharge facts
type: SpecReview
analysis: code-review
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

Ticket: PLAT-1525. Reviewed the frozen mapping-to-discharge implementation and its verification claims. Corpus validation, duplicated input handling, and test dependency provisioning need correction.

## Verdict

**FAIL** — high-severity defects remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Invalid controls can bypass the promised corpus-validation refusal. records::read discards documents missing id or type before validation; read_corpus treats the resulting empty corpus as valid. Removing either field from the healthy effective-control fixture makes quire validate exit 3, but facts-from-mappings exits 0 with [] and empty stderr. Preserve and reject these validation failures before proposing facts. | rust/crates/quoin-cli/src/evidence/mapping_facts.rs:178 |
| FND-002 | high | The new read_json duplicates the existing discharge::json_input file-or-stdin JSON reader at rust/crates/quoin-cli/src/discharge.rs:302-314. Searching the CLI for JSON readers found both copies with the same read-to-end and parse behavior; input-size and error-handling changes now need two edits. Reuse one reader as required by code-review Duplication inside one repo. The new unbounded stdin accumulation also violates the repository Untrusted input ceiling rule. | rust/crates/quoin-cli/src/evidence/mapping_facts.rs:153 |
| FND-003 | high | All 13 new integration tests require an external spec-objects-security checkout containing scope_exclusion.md. The default resolves a sibling checkout and the only override is SPEC_OBJECTS_SECURITY_DIR; neither Makefile nor the Rust CI job provisions that source. A normal CI checkout therefore cannot run this suite. With the ordinary checkout the suite reproduced 13 failures at line 43; it passed only after using the specially supplied updated sibling. Make the required module available through the supported dependency/test lane without relying on an unrelated checkout. | rust/crates/quoin-cli/tests/tc_173_facts_from_mappings.rs:30 |

## Coverage

The review is limited to the five files changed by origin/main...HEAD, at commit 1ffe671abf9d78c3221b8482e8eeeb417d74176d. All five were inspected; requirements FR-173, FR-174 and NFR-038 supply 18 examined criteria. FR-133 and FR-046 were context only. The new command, flags, corpus projection, fact construction, read-only admission and stale reporting all have governing requirements; no additional unowned behavior or source stub was found. The no-op writes in DryRun implement the explicitly required read-only proposal, rather than standing in for production persistence.

Reconciliation: quire trace --scope . --id FR-173 --prefix --json; equivalent FR-174 and NFR-038 queries, plus exact FR-174-AC-4 lookup. All returned verification claims have structural confidence. 17 of 18 criteria have verification claims; one is untagged. Citations were not counted as verification. quoin matrix was not used, as directed because of PLAT-1672. No stored execution evidence was reconciled.

Plan completion: not assessed

Optional semantic gap-analysis: skipped (not requested); ordinary code-review checks of assertions against their claimed criteria ran.

No applicable AssuranceProfile: AP-201 governs finding-quality measurement and comparison; AP-202 governs live semantic-lens evaluation. Neither applies to this mapping-to-discharge adapter. No spec or plan file changed, so spec-review and its sub-analyses were skipped. Python, React and Tokio lanes do not apply.

Prior concerns checked: control ids in evidenceRefs, delegated-only scope exclusions, and unused mappings to another set match the existing FR-173 contract. Absence of Mappings is accepted by both the updated module validator and the command, so this is not a newly demonstrated Quoin validation bypass. Ambient module discovery follows the implemented command contract; an explicit --module option is not specified. These are distinct from the reproduced missing-id/type refusal defect.

Ticket reconciliation remains necessary: PLAT-1525 asks for every mapping to resolve against a loaded set, cited control evidence, and delegated/accepted_risk/temporary_exception dispositions with owner and approval. FR-173 deliberately limits validation to the supplied binding set, emits control ids as evidence, and derives only delegated dispositions from Responsible plus a command-level approval. The implementation follows that narrower existing spec; this review does not silently treat the broader ticket acceptance as fulfilled or authorize changing human risk decisions.

## Validation

Checks: the 13 new CLI integration tests passed with SPEC_OBJECTS_SECURITY_DIR=/home/peter/dev/worktrees/spec-objects-security/spec_objects_security (upstream checkout 99f418a). quoin-quire tests: 48 passed. cargo fmt --all --check passed. Targeted cargo clippy --locked -p quoin-cli -p quoin-quire --all-targets --all-features -- -D warnings passed. cargo deny check passed (advisories, bans, licenses and sources); the first sandboxed attempt could not lock the advisory database, and the authorized cache-access retry passed. Initial integration run against /home/peter/dev/spec-objects-security/spec_objects_security failed 13/13 because scope_exclusion.md was absent, then the corrected dependency path passed. Targeted missing-id/type probes ran against the newly built candidate; the healthy control passed both tools, and each malformed control passed Quoin but failed Quire. Removing Mappings passed both.

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
- id: rust/crates/quoin-cli/src/evidence/mapping_facts.rs#read_json
  path: rust/crates/quoin-cli/src/evidence/mapping_facts.rs
  role: examined
  excerpt: "fn read_json(source: &str, label: &str) -> Result<Value, String> {\n \
    \   let bytes = if source == \"-\" {\n        let mut bytes = Vec::new();\n  \
    \      std::io::Read::read_to_end(&mut std::io::stdin(), &mut bytes)\n       \
    \     .map_err(|error| format!(\"cannot read {label}: {error}\"))?;\n        bytes\n\
    \    } else {\n        std::fs::read(source).map_err(|error| format!(\"cannot\
    \ read {label} {source}: {error}\"))?\n    };\n    serde_json::from_slice(&bytes).map_err(|error|\
    \ format!(\"{label} is not JSON: {error}\"))\n}"
- id: rust/crates/quoin-cli/tests/tc_173_facts_from_mappings.rs#security_module
  path: rust/crates/quoin-cli/tests/tc_173_facts_from_mappings.rs
  role: examined
  excerpt: "    let root = std::env::var_os(\"SPEC_OBJECTS_SECURITY_DIR\").map_or_else(\n\
    \        || {\n            Path::new(env!(\"CARGO_MANIFEST_DIR\"))\n         \
    \       .join(\"../../../../spec-objects-security/spec_objects_security\")\n \
    \       },\n        PathBuf::from,\n    );\n    let root = std::fs::canonicalize(&root).unwrap_or_else(|error|\
    \ {\n        panic!(\n            \"spec-objects-security is not at {}: {error};\
    \ set SPEC_OBJECTS_SECURITY_DIR\",\n            root.display()\n        )\n  \
    \  });\n    assert!(root.join(\"skeletons/scope_exclusion.md\").is_file());"
```
