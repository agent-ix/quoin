<!-- reviewer repo=agent-ix/quoin visibility=public quoin=0.30.1 module=spec-artifacts-process@v0.31.0 id=SR-20261113203418 method=gap-analysis lang=rust pr=quoin#888 model=gpt-6.1-sol run=2a67d7d7-f270-4a7d-8a5d-5db8116c4e48 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | The CLI command adapter implements FR-185 behavior but has no `#[implements("FR-185")]` trace link; the annotation on `build_statement` only links the pure mapper. |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "When a user runs `quoin evidence export-test-result`, Quoin SHALL read only the retained run records and `bindings.json` from the selected evidence directory and SHALL NOT read Git state, access the network, or spawn a process."}
  - {id: FR-185-AC-1, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "A fixture with a passing obligation, a failing symbol obligation and an obligation bound to a suite without a run exports `result: FAILED`; only the passing obligation is in `passedTests`, and the other two are in `failedTests`."}
  - {id: FR-185-AC-2, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "A fixture whose only non-pass outcome is `skip` exports `WARNED` with that obligation in `warnedTests`; an all-pass fixture exports `PASSED` with empty `warnedTests` and `failedTests`."}
  - {id: FR-185-AC-3, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The output subject equals the supplied array after canonicalization; a non-hex digest, missing `sha256`, empty array or non-array subject exits 2 and writes no output file."}
  - {id: FR-185-AC-4, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Repeating an export over the same store and subject yields byte-identical output, and reordering the stored runs or bindings does not change the bytes."}
  - {id: FR-185-AC-5, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The predicate has exactly the five specified members, deserializes through its closed type, and rejects a predicate with an additional member."}
  - {id: FR-185-AC-6, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "An unparsable run record or missing `bindings.json` exits 3 with `evidence_store_invalid` and the relevant path, and leaves no output file."}
  - {id: FR-185-AC-7, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "A static boundary check establishes that the export path calls no Git, network or process-spawning API."}
  - {id: FR-185-AC-8, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The consumer-side negative check from Engineering Assurance FR-069-AC-2 is reproducible from Quoin: an export cannot produce `result: PASSED` with a non-empty `failedTests`."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-185
    related_ids: []
    path: rust/crates/quoin-cli/src/evidence/test_result.rs
    lines: "34-37"
    excerpt: |-
      When a user runs `quoin evidence export-test-result`, Quoin SHALL read only the retained run records and `bindings.json` from the selected evidence directory and SHALL NOT read Git state, access the network, or spawn a process.
    finding: "The new `run` adapter implements the command's store access, input and output error mapping, and file publication, but it has no `#[implements("FR-185")]` attribute. The only implementation link is on `build_statement`, which does not cover this command path; add the implementation trace to the command adapter."
bindings:
  - {test_id: tc_185_cli_exports_pass_fail_and_missing_run_classifications, ac_id: FR-185-AC-1, trace: correct}
  - {test_id: tc_185_failures_and_missing_runs_never_pass, ac_id: FR-185-AC-1, trace: correct}
  - {test_id: tc_185_skip_warns_and_all_pass_passes, ac_id: FR-185-AC-2, trace: correct}
  - {test_id: tc_185_invalid_subject_exits_two_without_output, ac_id: FR-185-AC-3, trace: correct}
  - {test_id: tc_185_subject_is_preserved_and_bad_subjects_are_refused, ac_id: FR-185-AC-3, trace: correct}
  - {test_id: tc_185_reordered_store_records_produce_identical_bytes, ac_id: FR-185-AC-4, trace: correct}
  - {test_id: tc_185_lists_and_configuration_are_deterministic, ac_id: FR-185-AC-4, trace: correct}
  - {test_id: tc_185_predicate_type_is_closed_and_has_exact_members, ac_id: FR-185-AC-5, trace: correct}
  - {test_id: tc_185_invalid_store_names_path_and_leaves_no_output, ac_id: FR-185-AC-6, trace: correct}
  - {test_id: tc_185_cli_export_path_has_no_git_network_or_process_api, ac_id: FR-185-AC-7, trace: correct}
  - {test_id: tc_185_cli_exports_pass_fail_and_missing_run_classifications, ac_id: FR-185-AC-8, trace: correct}
  - {test_id: tc_185_failures_and_missing_runs_never_pass, ac_id: FR-185-AC-8, trace: correct}
```

+++
