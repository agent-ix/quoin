<!-- reviewer repo=agent-ix/quoin visibility=public quoin=0.30.1 module=spec-artifacts-process@v0.31.0 id=SR-20261113203419 method=spec-review/base lang=spec pr=quoin#888 model=gpt-6.1-sol run=2a67d7d7-f270-4a7d-8a5d-5db8116c4e48 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | other | FR-185 is missing from the maintained functional-requirements index. |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Quoin SHALL export its retained evidence as an in-toto Statement v1 with the Test Result v0.1 predicate, so consumers can attest the statement using the subject supplied by the caller."}
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
findings:
  - fnd: FND-001
    severity: low
    confidence: high
    method: spec-review/base
    check: coverage
    artifact_id: FR-185
    related_ids: []
    path: spec/functional/index.md
    lines: "166"
    excerpt: |-
      Quoin SHALL export its retained evidence as an in-toto Statement v1 with the Test Result v0.1 predicate, so consumers can attest the statement using the subject supplied by the caller.
    finding: "FR-185 is absent from `spec/functional/index.md`, whose purpose is to index the repository's functional requirements; readers using the index cannot discover this new requirement."
bindings: []
```

+++
