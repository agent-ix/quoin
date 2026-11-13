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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | The missing-id fix rejects valid non-control documents. records::read marks every document without id as malformed regardless of checked_types or its declared archetype. master-requirements allows no id; adding a valid spec.md to a healthy control corpus makes facts-from-mappings refuse it. Preserve the missing-control-id refusal while validating each document against its actual contract. | rust/crates/quoin-quire/src/records.rs:137 |

## Dispositions

### Round 1

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Missing id/type controls now enter Corpus::malformed and are refused; a new integration test checks both omissions. The separate overly broad application is FND-004. |
| FND-002 | fixed | The duplicate reader is gone; mapping proposals and discharge use discharge::json_input. File and stdin reads stop at 8 MiB plus one byte and refuse over-limit input. |
| FND-003 | fixed | The tests generate a local synthetic module and install a synthetic pack under their temporary IX_HOME. No external security checkout is read. |
| FND-004 | still-open | New regression: the missing-id rule applies to every loaded document, including valid master-requirements documents whose schema does not require id. A valid spec.md therefore makes facts-from-mappings refuse a healthy control corpus. Scope identity validation to the declared archetype contract and retain malformed-control refusals. |

### Round 1 evidence

Candidate: ba003bbed8000adb44f47e7539a844f27dea2b80. Current method verdict: FAIL. The PR remains blocked by open findings. The original Findings section is unchanged.

Owner scope ruling: this code PR remains within existing FR-173; the broader disposition contract is handled by separate specification work. No behavior from that expansion is required in this disposition pass.

Independent execution: the shared JSON-reader ceiling test passed (1/1) at this head with SPEC_OBJECTS_SECURITY_DIR and IX_FILAMENT_MODULES_PATH unset. The mapping suite, valid-master runtime probe, quoin-quire tests, formatting, targeted Clippy and standalone count-helper probe could not start because all six mandatory locked-build lanes were occupied. The code lead directed static completion; all three queued wrapper sessions were cancelled (exit 130). No aggregate gate ran. Coder-reported passes were not substituted for independent execution.

Static confirmation of FND-004: the installed master-requirements-frontmatter.schema.json requires only type, name, org and component_type; no id is required. records.rs:137-147 marks any document missing id malformed regardless of archetype. mapping_facts.rs:181-186 refuses the first malformed path. Consequently a valid id-less spec.md is rejected; no runtime result is claimed. Quoin's own spec/spec.md is an example of this permitted document shape.

Static confirmation of the remaining gap FND-002: the assertion at tc_173_facts_from_mappings.rs:409-420 accepts all lines without the sentinel or percent sign whose compliant-bearing lines begin with |. The concrete input | Metric | Value | followed by | Compliant | 1 | satisfies every condition, while reporting a prohibited count. This is the unresolved count portion of the original separate gap finding, not a duplicate new finding.

Trace queries for FR-173, FR-174 and NFR-038 recognize all 18 criteria; every verification claim is structural. FR-174-AC-4 now has the installed-pack test. Citations were not treated as evidence; quoin matrix was not used. Plan completion: not assessed. Spec-review sub-analyses and other language lanes remain inapplicable; optional semantic gap-analysis was not requested.

GitHub reports head ba003bbed8000adb44f47e7539a844f27dea2b80, OPEN, MERGEABLE (no git conflict), and mergeStateStatus BLOCKED. Review disposition remains not mergeable because code FND-004 and gap FND-002 are still open.

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "        let has_id = frontmatter\n            .get(\"id\")\n   \
    \         .and_then(Value::as_str)\n            .is_some_and(|id| !id.trim().is_empty());\n\
    \        let has_type = frontmatter\n            .get(\"type\")\n            .and_then(Value::as_str)\n\
    \            .is_some_and(|kind| !kind.trim().is_empty());\n        (!has_id ||\
    \ !has_type).then(|| document.path.clone())"
- fnd: FND-002
  outcome: fixed
  after_excerpt: "        std::fs::File::open(source)\n            .and_then(|file|\
    \ file.take(limit).read_to_end(&mut bytes))\n            .map_err(|error| format!(\"\
    cannot read {label} {source}: {error}\"))?;\n    }\n    if bytes.len() > MAX_JSON_INPUT_BYTES\
    \ {\n        return Err(format!(\n            \"{label} exceeds the {MAX_JSON_INPUT_BYTES}-byte\
    \ input limit\"\n        ));\n    }\n    serde_json::from_slice(&bytes).map_err(|error|\
    \ format!(\"{label} is not JSON: {error}\"))"
- fnd: FND-003
  outcome: fixed
  after_excerpt: "fn synthetic_types_module(parent: &Path) -> PathBuf {\n    let module\
    \ = parent.join(\"synthetic-security-types\");\n    std::fs::create_dir_all(&module).unwrap();\n\
    \    std::fs::write(\n        module.join(\"manifest.yaml\"),\n        format!(\n\
    \            \"manifest_version: 1.0.0\\nname: synthetic-security-types\\n\\\n\
    \             version: 0.1.0\\n{}\",\n            synthetic_object_types()\n \
    \       ),\n    )\n    .unwrap();\n    module\n}"
- fnd: FND-004
  outcome: still-open
  reason: 'New regression: the missing-id rule applies to every loaded document, including
    valid master-requirements documents whose schema does not require id. A valid
    spec.md therefore makes facts-from-mappings refuse a healthy control corpus. Scope
    identity validation to the declared archetype contract and retain malformed-control
    refusals.'
```

### Round 2

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | Missing-id validation now checks the declared contract only for checked record types. The new valid-idless-master test asserts the effective control still produces C1; malformed control id/type cases remain checked. |
| FND-005 | still-open | The handoff identifies this as a signed head, but the commit has no signature header: git reports N and GitHub reports verified:false with reason unsigned. The code lead must supply the intended signed candidate before the signature discrepancy can close. |

### Round 2 evidence

Candidate: 2e72eb4639bd56627ba73ec5c2d1db18f09c8dfc. Current method verdict: CONDITIONAL. All original implementation and gap findings are now fixed. The separately reported signature discrepancy remains open.

Source disposition: code FND-004 is fixed by invalid_identity_paths: id-less documents are rejected only when their checked archetype contract rejects them. Valid id-less master-requirements documents remain outside that control-only identity check, while missing control id/type cases still reach refusal. The new FR-173-AC-1 test explicitly asserts that the valid master document leaves one C1 direct fact. Original code FND-001/002/003 were rechecked and remain fixed: malformed-control refusal, one bounded shared JSON reader, and generated local test modules with no external security checkout.

Gap FND-002 is fixed: is_expected_rmm_compliance_row recognizes the exact C1/C3 result shapes and rejects the reported summary-row counterexample explicitly. The sentinel and intermediate-file checks remain. Gap FND-001's installed-pack test still installs and reads the synthetic pack through temporary IX_HOME.

Independent quire trace lookups for FR-173-AC-1, FR-173-AC-5, FR-174-AC-3 and FR-174-AC-4 resolve the relevant tests with structural confidence. Citations are not counted as verification; quoin matrix was not used. No spec or plan requirement changed, so spec-review sub-analyses remain inapplicable. Optional semantic gap-analysis was not requested. Plan completion: not assessed.

Runtime verification is unavailable for this round: the focused mapping/reader/quoin-quire tests, formatting and targeted Clippy batch remained queued behind all six occupied locked-build lanes. The code lead directed completion without indefinite waiting; the queued wrapper was cancelled with exit 130 before any check started. Coder-reported passes are not presented as independent results. No aggregate gate ran. The attempted batch used the mandatory wrapper, the worktree target, debug=0, at most four jobs, and unset both external security-module environment variables.

Signature evidence was independently checked: a second raw-object audit with git --no-replace-objects finds only tree, parent, author and committer headers, with neither gpgsig nor gpgsig-sha256. GitHub also reports signature_present:false. There are no signature bytes for cryptographic verification; this is distinct from missing trusted-signer configuration. Additional corroboration: git log reports signature status N; git cat-file shows no signature header; GitHub's commit API reports verified:false, reason:unsigned. GitHub reports this exact head OPEN, MERGEABLE (no git conflict), and mergeStateStatus BLOCKED. The implementation and gap findings are resolved, but the signature finding and outstanding lead gate prevent an unqualified merge-ready claim. The frozen branch was not edited; these updated SRs are staged in the reviewer scratchpad for the lead's artifact commit.

```yaml
dispositions:
- fnd: FND-004
  outcome: fixed
  after_excerpt: "        if !has_id\n            && checked_types.contains(&kind)\n\
    \            && matches!(check(registry, document, kind)?, Contract::Violated(_))\n\
    \        {\n            malformed.push(document.path.clone());\n        }"
- fnd: FND-005
  outcome: still-open
  reason: 'The handoff identifies this as a signed head, but the commit has no signature
    header: git reports N and GitHub reports verified:false with reason unsigned.
    The code lead must supply the intended signed candidate before the signature discrepancy
    can close.'
```

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The candidate does not carry the signature claimed by the signed-head handoff. Commit 2e72eb4639bd56627ba73ec5c2d1db18f09c8dfc has no signature header; git reports signature status N, and GitHub commit verification is false with reason unsigned. This provenance discrepancy remains a merge blocker until the intended signed candidate is provided. | git:2e72eb4639bd56627ba73ec5c2d1db18f09c8dfc |
