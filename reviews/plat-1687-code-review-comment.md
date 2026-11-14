<!-- reviewer repo=agent-ix/quoin visibility=public quoin=0.30.1 module=spec-artifacts-process@v0.31.0 id=SR-20261113203417 method=code-review lang=rust pr=quoin#888 model=gpt-6.1-sol run=2a67d7d7-f270-4a7d-8a5d-5db8116c4e48 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | code-bug | A duplicate run entry for one bound symbol can hide a later fail and export the obligation as PASSED. |
| FND-002 | medium | code-bug | An unreadable `runs/` directory is silently treated as empty, so the command exits 0 and exports FAILED instead of `evidence_store_invalid` (exit 3). |
| FND-003 | medium | code-bug | A directory-fsync error occurs after the publisher renames the output into place, so the command exits 4 while the destination has been created or replaced. |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-185-AC-1, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "A fixture with a passing obligation, a failing symbol obligation and an obligation bound to a suite without a run exports `result: FAILED`; only the passing obligation is in `passedTests`, and the other two are in `failedTests`."}
  - {id: FR-185-AC-2, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "A fixture whose only non-pass outcome is `skip` exports `WARNED` with that obligation in `warnedTests`; an all-pass fixture exports `PASSED` with empty `warnedTests` and `failedTests`."}
  - {id: FR-185-AC-3, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The output subject equals the supplied array after canonicalization; a non-hex digest, missing `sha256`, empty array or non-array subject exits 2 and writes no output file."}
  - {id: FR-185-AC-4, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Repeating an export over the same store and subject yields byte-identical output, and reordering the stored runs or bindings does not change the bytes."}
  - {id: FR-185-AC-5, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The predicate has exactly the five specified members, deserializes through its closed type, and rejects a predicate with an additional member."}
  - {id: FR-185-AC-6, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "An unparsable run record or missing `bindings.json` exits 3 with `evidence_store_invalid` and the relevant path, and leaves no output file."}
  - {id: FR-185-AC-7, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "A static boundary check establishes that the export path calls no Git, network or process-spawning API."}
  - {id: FR-185-AC-8, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The consumer-side negative check from Engineering Assurance FR-069-AC-2 is reproducible from Quoin: an export cannot produce `result: PASSED` with a non-empty `failedTests`."}
  - {id: FR-185-CON-1, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The export SHALL not sign the statement, compute the subject, read Git state, access the network, or spawn a process."}
  - {id: FR-185-CON-2, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Quoin SHALL NOT vendor the in-toto schema or protobuf files."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "On failure, no output file is created or replaced. The command SHALL exit 2 for usage errors or an invalid subject, exit 3 for an unreadable or invalid evidence store with diagnostic code `evidence_store_invalid` and the relevant path, and exit 4 when the output cannot be written."}
findings:
  - fnd: FND-001
    severity: high
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: FR-185-AC-1
    related_ids: [FR-185-AC-8]
    path: rust/crates/quoin-evidence/src/stale.rs
    lines: "13-22"
    excerpt: |-
      A fixture with a passing obligation, a failing symbol obligation and an obligation bound to a suite without a run exports `result: FAILED`; only the passing obligation is in `passedTests`, and the other two are in `failedTests`.
    finding: "`failed_bound_symbols` uses `find`, so it sees only the first entry for a bound symbol. With a validly deserializable run containing `test: pass` followed by `test: fail`, both this helper and the mapper select the pass and export the obligation in `passedTests` with result PASSED. I reproduced that output with the CLI. Reject duplicate symbol entries as an invalid run or classify all matching entries so any fail/error dominates."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: FR-185
    related_ids: [FR-185-AC-6]
    path: rust/crates/quoin-cli/src/evidence/test_result.rs
    lines: "81-88"
    excerpt: |-
      The command SHALL exit 2 for usage errors or an invalid subject, exit 3 for an unreadable or invalid evidence store with diagnostic code `evidence_store_invalid` and the relevant path, and exit 4 when the output cannot be written.
    finding: "The export calls `latest_runs` through `DiskEvidence::entries`, whose `read_dir` error path returns an empty vector for both a missing and an unreadable directory. With readable bindings and `evidence/runs` mode 000, the command exits 0 and emits a FAILED statement instead of returning `evidence_store_invalid` with exit 3. Distinguish a missing run directory from an unreadable one for this export path."
  - fnd: FND-003
    severity: medium
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: FR-185
    related_ids: []
    path: rust/crates/quoin-cli/src/evidence/test_result.rs
    lines: "105-109"
    excerpt: |-
      On failure, no output file is created or replaced.
    finding: "`publish` renames the temporary file to the destination and then fsyncs the parent directory. If that final fsync fails, `publish` returns Err after the output has already been created or replaced; this handler converts the Err to exit 4. That contradicts the stated no-output-on-error contract. Preserve the prior destination or otherwise ensure a reported write error cannot leave the new output published."
bindings: []
```

+++
