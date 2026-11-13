---
id: SR-20261113203415
title: "Gap analysis — PLAT-1579 per-class evidence floors, FR-160 (quoin PR 880)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin; spec/functional/FR-160-per-class-evidence-floors.md, rust/crates/quoin-auditor/src/audit/{class.rs,ladder.rs,scores.rs,mod.rs}, rust/crates/quoin-auditor/tests/tc_160_class_floors.rs, rust/crates/quoin-assurance/tests/tc_1080_matrix.rs, rust/crates/quoin-cli/tests/tc_160_class_policy_command.rs, rust/crates/quoin-core/src/ops/quire/tests.rs"
review_set: subset
---

## Summary

Ticket: PLAT-1579. PR head `46d853e4`. Plan completion: not assessed. The ticket's four ACs (policy
as data with no built-in floor; a finding naming class, floor and shortfall; a class column in
audit and matrix output; tests for each floor kind and for an unclassified obligation) are all met
by the code and the tests.

`quoin matrix --json` (quoin 0.30.1) reports every FR-160 criterion `untagged`, even though the tests
carry `#[trace("FR-160-AC-n")]`. The same happens across the repository: 1656 untagged and 11 tagged,
and FR-039 and FR-094 are also all untagged. So this is a matrix harvest limit with this repo's
attribute form, not something this PR caused. The AC-to-test map below was built by reading the
tags.

| AC | Tests | Result |
| --- | --- | --- |
| FR-160-AC-1 | without_a_policy_...; tc_160_300; tc_160_matrix_001 | covered |
| FR-160-AC-2 | a_required_method_needs_...; an_unbound_obligation_...; tc_160_301 | covered for runs; scan join untested (FND-001) |
| FR-160-AC-3 | a_class_mutation_floor_...; another_classs_mutation_floor_... | covered |
| FR-160-AC-4 | class_independence_needs_...; the_class_assessment_record_... | covered; shortfall text weak (FND-002, FND-003) |
| FR-160-AC-5 | criticality_and_class_floors_... | covered; tie untested (FND-004) |
| FR-160-AC-6 | an_unclassified_obligation_...; tc_160_301; tc_160_matrix_002 | covered (exit 0 via `ok()`) |
| FR-160-AC-7 | a_class_without_a_policy_entry_... | covered |
| FR-160-AC-8 | a_policy_that_cannot_be_applied_...; a_report_without_the_vocabulary_...; tc_160_303 | covered for the AC's causes; new spec causes untested (FND-005) |
| FR-160-AC-9 / CON-1 | the_class_is_read_only_from_the_declared_carried_name; criticality_is_never_read_as_a_class | covered |
| FR-160-AC-10 | profile_and_class_independence_are_assessed_separately_... | covered |
| FR-160-AC-11 | the_class_report_carries_...; tc_160_301; tc_160_302; tc_160_matrix_002 | covered |
| FR-160-AC-12 | two_audits_of_the_same_input_are_identical; tc_160_304 | covered |
| FR-160-AC-13 | inspection | holds: class values and the field name appear only in tests |

A targeted `cargo mutants --in-place -p quoin-auditor` run over `independence_failed`, `floor_for`,
`shortfall`, `evidence_findings` and `mutation_floor` tested 31 mutants: 25 caught, 3 unviable,
3 missed. The ladder early return after an independence failure (`independence_failed -> false`)
**is caught**. The three missed are FND-002, FND-003 and FND-004.

## Verdict

FR-160 is implemented and its ACs are backed by tests that can fail. Two medium test gaps should be
closed in this PR: the scan-record join that the spec amendment adds, and the shortfall text naming
a dimension that is not shared. The rest is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec amendment widens the required-method join to "run or finding-shaped scan record", but no test binds a scan-only suite. Every `scene()` supplies `runs` only, and the CLI test binds no evidence. If `scan` were dropped from `[run, scan]` in `evidence_findings`, every test would still pass, and a scanner-only method's floor could never be met again. Fix: add a case in tc_160_class_floors where the obligation's only bound suite has a `FindingRecord` (in `scans`) whose `evidence_kind` is the required method's kind, and assert no `class-method-missing`. | rust/crates/quoin-auditor/src/audit/class.rs:256-268; rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:107-146 |
| FND-002 | medium | The class independence finding can name a dimension that is not shared, and no test fails. Mutant `class.rs:389 replace < with <=` survives. Under it, the AC-4 shared-actor case also reports `technique (shared)` although the two lines differ on technique. The test checks only that `actor (shared)` is present. FR-160 requires the finding to name "the dimensions still shared or missing", so a wrong dimension is a wrong result. Fix: in `class_independence_needs_...`, also assert the summary does not mention `technique`. | rust/crates/quoin-auditor/src/audit/class.rs:389; rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:323-335 |
| FND-003 | low | This is the survived early-return mutant. `shortfall`'s `if lines < 2` early return is untested: `class.rs:380 replace < with >` survives. The one-suite case in AC-4 asserts only that one finding exists, so its shortfall text could become "No single pair of lines differs on every dimension at once" (a pair claim with one line) and nothing would notice. It does need a test, because the text is the shortfall FR-160 says the finding names. A one-line assertion on the existing `alone` report (summary contains `Fewer than two evidence lines`) is enough. | rust/crates/quoin-auditor/src/audit/class.rs:380-382; rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:347-353 |
| FND-004 | low | The tie between a class floor and a criticality floor is untested: mutant `scores.rs:252 replace > with >=` survives, which flips the source a tie names from criticality to class. Fix: add a tie case (both 0.7) to `criticality_and_class_floors_...` asserting `criticality high`, alongside the FR-160 wording in SR-20261113203416 FND-002. | rust/crates/quoin-auditor/src/audit/scores.rs:252; rust/crates/quoin-auditor/tests/tc_160_class_floors.rs:400-438 |
| FND-005 | low | Two refusal causes the amendment adds are untested. One is a required method the catalog declares with no (or an empty) `evidence_kind`, handled by `declared_kind`'s `.filter`. The other is an unknown key in the policy file (`deny_unknown_fields`); its refusal path is serde's, not `QAU-CLASS-POLICY-REFUSED`, and no test shows it is refused with no output. Fix: add a `method("bare", "")` or a `None`-kind case to the refusal table, and one CLI or unit case with a misspelt key such as `mutation_flor`. | rust/crates/quoin-auditor/src/audit/class.rs:365-375; rust/crates/quoin-auditor/src/audit/class.rs:47,60 |
| FND-006 | low | FR-160 says quoin builds a class independence assessment record "for each obligation of the class". The code builds one only for obligations the ladder reaches (`Pass::class_independence` says "that the ladder reached"). A class-3 obligation that is undischarged, stale or rests on a vacuous scan has no record in `classIndependence`. Its other finding is still raised, so nothing passes wrongly, but the record set is narrower than stated. Fix: either state the narrower set in FR-160 or build the record before the ladder's early returns. | rust/crates/quoin-auditor/src/audit/ladder.rs:101-109; rust/crates/quoin-auditor/src/audit/mod.rs:131-133 |
| FND-007 | low | FR-160 says the profile assessments "carry `source: profile`". Under a class policy only the profile *finding* gains `source`. The profile `IndependenceAssessment` records in the report's `independence` member do not. Fix: say in FR-160 that `source` is on findings, or add it to the profile records under a policy. | rust/crates/quoin-auditor/src/audit/ladder.rs:191-205; spec/functional/FR-160-per-class-evidence-floors.md:85-86 |

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-160-AC-1 .. FR-160-AC-12, FR-160-CON-1 | examined | Mapped above |
| FR-160-AC-13, FR-160-CON-2 | examined | Inspection: hold |
| PLAT-1579 ticket ACs (4) | examined | Met |
| `ClassContext` (class.rs) | examined | FND-001, FND-002, FND-003, FND-005 |
| ladder `independence_failed` | examined | Early-return mutant caught; FND-006, FND-007 |
| `floor_for` | examined | FND-004 |


## Disposition Round 1 Evidence

Replacement reviewer: `gpt-6-astra`; run `9118bc4b-9e8a-4c1f-9f5b-4d09820fcc63`; Codex session
`01a1d6e9-d820-7821-83d5-1b05bd42a929`. Sole reviewer for this disposition.
Ticket: PLAT-1579; PR: agent-ix/quoin#880. Reviewed pushed head
`fb4c9ab7ab65bdbcc5bcf0b97150b99d600d0a1b` and fixes `f2f627dd` and `fb4c9ab7`.
Final diff: `git diff origin/main...HEAD`, merge base `63acd34f8a571f4f51541433df86757360670b30`;
verified origin/main `2975c419cfefdf38745386e3b72c3bb63c3c0613` against the remote.
All 27 changed files and the three committed review artifacts were inspected.

Current outcome: **6 fixed; FND-007 still-open. PR not mergeable.** No new findings. Original findings and prior review text above
are retained verbatim; the following rows are this round's current dispositions.

Verification: source, test-assertion, spec and pushed-commit inspection; `git diff --check
origin/main...HEAD` passed. No Cargo build/test, mutation run, computed matrix rerun or aggregate
gate ran in this pass. Earlier test/mutation claims above remain historical, not results for
this head. The aggregate gate belongs to the code lead. Tools: quoin `0.30.1`, quire `0.44.0`
(engine `0.59.1`), spec-artifacts-process `v0.31.0`. Validation uses this artifact's explicit
scratch path with `quire validate --scope .`.

## Dispositions

### Round 1

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | f2f627dd adds a_bound_scan_record_of_the_kind_satisfies_a_required_method: its only binding names scanner, runs is empty, and only scans supplies evidence. Matching mutation-run clears the finding; test-run restores it. |
| FND-002 | fixed | f2f627dd checks the exact shortfall suffix and rejects technique (shared) for two lines whose techniques differ. The previously surviving < to <= change would fail these assertions. |
| FND-003 | fixed | f2f627dd asserts the exact one-line shortfall. Removing or inverting the lines < 2 return would now fail this test. |
| FND-004 | fixed | f2f627dd adds equal 0.7 class and criticality floors and asserts criticality high. FR-160 mutation wording and AC-5 now state the same tie rule. |
| FND-005 | fixed | f2f627dd adds both absent and empty evidence_kind catalog entries (lines 133-137) and exercises them in the refusal table (lines 570-583), checking ClassPolicyRefused and the method name. This separate deserialization test rejects mutation_flor; the production core read fails before invoking audit and producing an audit payload. |
| FND-006 | fixed | f2f627dd computes the class assessment before ladder::run and appends it to class_independence after the call, independent of the ladder early-return path. The new undischarged-obligation test checks one insufficient record with the correct obligation. Stale and vacuous-scan early returns share this same enclosing path. |
| FND-007 | still-open | f2f627dd adds source: profile to the library report at rust/crates/quoin-auditor/src/audit/mod.rs:264-270, but rust/crates/quoin-core/src/ops/auditor/mod.rs:74-89 removes independence and deserializes it as Vec<IndependenceAssessment>. That type (rust/crates/quoin-evidence/src/types/independence.rs:120-139) has neither a source field nor a flattened remainder, so serde discards source. CLI JSON uses that reserialized boundary member at rust/crates/quoin-cli/src/evidence/audit.rs:175. The new test at tc_160_class_floors.rs:394 calls only the library and misses this loss. Preserve the source through the typed core/CLI payload and add a boundary assertion with and without a class policy, or make the explicitly permitted spec correction. The documented profile record source is still absent from user-visible audit output. |

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  reason: 'f2f627dd adds a_bound_scan_record_of_the_kind_satisfies_a_required_method: its only binding names scanner,
    runs is empty, and only scans supplies evidence. Matching mutation-run clears the finding; test-run restores
    it.'
  after_path: rust/crates/quoin-auditor/tests/tc_160_class_floors.rs
  after_lines: 788-791
  after_excerpt: |2-
        input.scans = Some(vec![scan("mutation-run")]);
        assert!(of_kind(&report(&input), "class-method-missing").is_empty());
        input.scans = Some(vec![scan("test-run")]);
        assert_eq!(of_kind(&report(&input), "class-method-missing").len(), 1);
- fnd: FND-002
  outcome: fixed
  reason: f2f627dd checks the exact shortfall suffix and rejects technique (shared) for two lines whose techniques
    differ. The previously surviving < to <= change would fail these assertions.
  after_path: rust/crates/quoin-auditor/tests/tc_160_class_floors.rs
  after_lines: 336-344
  after_excerpt: |2-
        // Only the dimension the two lines truly share is named.
        assert!(
            found[0]
                .summary
                .ends_with("Still shared or missing: actor (shared)."),
            "{}",
            found[0].summary
        );
        assert!(!found[0].summary.contains("technique (shared)"));
- fnd: FND-003
  outcome: fixed
  reason: f2f627dd asserts the exact one-line shortfall. Removing or inverting the lines < 2 return would now fail
    this test.
  after_path: rust/crates/quoin-auditor/tests/tc_160_class_floors.rs
  after_lines: 363-371
  after_excerpt: |2-
        let found = of_kind(&alone, "insufficient-independence");
        assert_eq!(found.len(), 1);
        assert!(
            found[0]
                .summary
                .ends_with("Fewer than two evidence lines are bound."),
            "{}",
            found[0].summary
        );
- fnd: FND-004
  outcome: fixed
  reason: f2f627dd adds equal 0.7 class and criticality floors and asserts criticality high. FR-160 mutation wording
    and AC-5 now state the same tie rule.
  after_path: rust/crates/quoin-auditor/tests/tc_160_class_floors.rs
  after_lines: 473-480
  after_excerpt: |2-
        let tie = crit_and_class(Some("high"), Some("class-2"), 0.7, 0.7);
        let found = of_kind(&tie, "insufficient-mutation-score");
        assert_eq!(found.len(), 1);
        assert!(
            found[0].summary.contains("criticality high"),
            "a tie names criticality: {}",
            found[0].summary
        );
- fnd: FND-005
  outcome: fixed
  reason: f2f627dd adds both absent and empty evidence_kind catalog entries (lines 133-137) and exercises them in
    the refusal table (lines 570-583), checking ClassPolicyRefused and the method name. This separate deserialization
    test rejects mutation_flor; the production core read fails before invoking audit and producing an audit payload.
  after_path: rust/crates/quoin-auditor/tests/tc_160_class_floors.rs
  after_lines: 746-754
  after_excerpt: |-
    fn an_unknown_policy_key_is_refused() {
        let misspelt = json!({
            "class_field": "class",
            "classes": { "class-2": { "mutation_flor": 0.8, "rationale": RATIONALE } }
        });
        let error = serde_json::from_value::<quoin_auditor::audit::ClassPolicy>(misspelt)
            .expect_err("an unknown key is refused");
        assert!(error.to_string().contains("mutation_flor"), "{error}");
    }
- fnd: FND-006
  outcome: fixed
  reason: f2f627dd computes the class assessment before ladder::run and appends it to class_independence after the
    call, independent of the ladder early-return path. The new undischarged-obligation test checks one insufficient
    record with the correct obligation. Stale and vacuous-scan early returns share this same enclosing path.
  after_path: rust/crates/quoin-auditor/src/audit/mod.rs
  after_lines: 184-194
  after_excerpt: |2-
            let class_assessment = indexed
                .class
                .as_ref()
                .and_then(|class| class.independence(obligation, &bindings));
            let mut pass = ladder::run(
                input,
                &indexed,
                obligation,
                &bindings,
                class_assessment.as_ref(),
            )?;
- fnd: FND-007
  outcome: still-open
  reason: 'f2f627dd adds source: profile to the library report at rust/crates/quoin-auditor/src/audit/mod.rs:264-270,
    but rust/crates/quoin-core/src/ops/auditor/mod.rs:74-89 removes independence and deserializes it as Vec<IndependenceAssessment>.
    That type (rust/crates/quoin-evidence/src/types/independence.rs:120-139) has neither a source field nor a flattened
    remainder, so serde discards source. CLI JSON uses that reserialized boundary member at rust/crates/quoin-cli/src/evidence/audit.rs:175.
    The new test at tc_160_class_floors.rs:394 calls only the library and misses this loss. Preserve the source
    through the typed core/CLI payload and add a boundary assertion with and without a class policy, or make the
    explicitly permitted spec correction. The documented profile record source is still absent from user-visible
    audit output.'
```


### Round 2

Reviewer: `gpt-6-astra`; run `54480e2e-e540-493d-8253-ad85c3112839`. PLAT-1579 / quoin#880.
Inspected pushed head `2b55708a85edb3c98b7209cf72089ff0a9d17990`, fixes `6526e53a` and `2b55708a`,
and the complete delta from round-1 head `fb4c9ab7`. The three committed SR files matched
these scratch copies before this append; original findings and round-1 rows remain intact.

Current outcome: **FND-007 fixed; all seven findings now fixed.** No new findings. Across all three artifacts every finding's
latest disposition is now fixed. Review dispositions permit merge after the lead's remaining
required checks and commitment of the latest scratch artifacts; this is not a gate result.

Inspection traced library source insertion through `quoin-core/src/ops/auditor/mod.rs`, the
new typed wrapper in `wire.rs`, and CLI JSON forwarding at `quoin-cli/src/evidence/audit.rs:175`.
The wrapper reuses `IndependenceAssessment` with flattening and keeps the source optional, so
ordinary assessment fields and output without a class policy retain their prior shape.
The new boundary test reaches production dispatch and serialized output. No unrelated source
or spec changes landed in this fix round. Existing closed findings require no repeat rows.

Verification in this pass: code, diff and test-assertion inspection only. No builds, tests or
gates were run. The user's handoff reports fmt, focused quoin-core clippy and four boundary
tests passing through locked-build.sh; these are supplied results, not fresh reviewer runs.
Tools: quoin 0.30.1; quire 0.44.0 (engine 0.59.1); spec-artifacts-process v0.31.0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-007 | fixed | 6526e53a changes the core conversion and AuditPayload to ProfileIndependenceAssessment, which flattens the existing assessment and declares optional source: IndependenceSource. The Profile variant serializes as profile; absent source is omitted. Thus the library-added marker survives the typed core boundary and the CLI forwards it unchanged. The new tc_501_601_profile_assessment_source_survives_audit_boundary exercises production dispatch and serialized output with and without a class policy. 2b55708a formats that fix. No new defect found. |

```yaml
dispositions:
- fnd: FND-007
  outcome: fixed
  reason: '6526e53a changes the core conversion and AuditPayload to ProfileIndependenceAssessment, which flattens
    the existing assessment and declares optional source: IndependenceSource. The Profile variant serializes as
    profile; absent source is omitted. Thus the library-added marker survives the typed core boundary and the CLI
    forwards it unchanged. The new tc_501_601_profile_assessment_source_survives_audit_boundary exercises production
    dispatch and serialized output with and without a class policy. 2b55708a formats that fix. No new defect found.'
  after_path: rust/crates/quoin-core/src/ops/auditor/mod.rs
  after_lines: 72-76
  after_excerpt: |2-
        let independence = report
            .other
            .remove("independence")
            .map(serde_json::from_value::<Vec<ProfileIndependenceAssessment>>)
            .transpose()
```
