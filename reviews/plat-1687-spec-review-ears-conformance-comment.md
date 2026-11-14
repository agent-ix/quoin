<!-- reviewer repo=agent-ix/quoin visibility=public quoin=0.30.1 module=spec-artifacts-process@v0.31.0 id=SR-20261113203420 method=spec-review/ears-conformance lang=spec pr=quoin#888 model=gpt-6.1-sol run=2a67d7d7-f270-4a7d-8a5d-5db8116c4e48 -->

+++ [reviewer data]

```yaml
clean: true
scope:
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Quoin SHALL export its retained evidence as an in-toto Statement v1 with the Test Result v0.1 predicate, so consumers can attest the statement using the subject supplied by the caller."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "When a user runs `quoin evidence export-test-result`, Quoin SHALL read only the retained run records and `bindings.json` from the selected evidence directory and SHALL NOT read Git state, access the network, or spawn a process."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The statement SHALL contain exactly `_type`, `subject`, `predicateType` and `predicate`, with `_type` equal to `https://in-toto.io/Statement/v1` and `predicateType` equal to `https://in-toto.io/attestation/test-result/v0.1`."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The predicate SHALL contain exactly `result`, `configuration`, `passedTests`, `warnedTests` and `failedTests`. Its closed deserialization type SHALL reject unknown members."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Quoin SHALL select the latest run for each suite by greatest lexical `timestamp`; when timestamps tie, Quoin SHALL select the record with the lexicographically greatest store path."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "For each suite used in the export, `configuration` SHALL contain one entry with `name` equal to the suite id and annotations `tool` and `timestamp` from the selected run for that suite. Entries SHALL be sorted by suite id."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Quoin SHALL reuse the stale-evidence conditions of `quoin-auditor::audit::stale::FindingKind::STALE_EVIDENCE`. A bound suite without a recorded run, or a latest bound-symbol outcome of `fail` or `error`, makes the obligation stale."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "If an obligation is stale under the `quoin-auditor::audit::stale::FindingKind::STALE_EVIDENCE` conditions, then Quoin SHALL put it in `failedTests`."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "When an obligation is not stale and any bound symbol has outcome `skip` or is absent from its run, Quoin SHALL put it in `warnedTests`."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "When an obligation is not stale and every bound symbol has outcome `pass`, Quoin SHALL put it in `passedTests`."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "This export SHALL NOT define a second staleness rule."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Quoin SHALL sort and deduplicate the three test lists. Quoin SHALL place each obligation in exactly one list."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "`result` SHALL be `FAILED` when `failedTests` is non-empty, `WARNED` when `failedTests` is empty and `warnedTests` is non-empty, and `PASSED` otherwise."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Quoin SHALL copy the caller's subject array without computing or adding any digest. Quoin SHALL validate that the array is non-empty and that every descriptor has a `digest.sha256` of exactly 64 lowercase hexadecimal characters."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Quoin SHALL write the canonical statement atomically by writing a temporary file and renaming it to the destination only after successful export."}
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The command SHALL exit 2 for usage errors or an invalid subject, exit 3 for an unreadable or invalid evidence store with diagnostic code `evidence_store_invalid` and the relevant path, and exit 4 when the output cannot be written."}
  - {id: FR-185-CON-1, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "The export SHALL not sign the statement, compute the subject, read Git state, access the network, or spawn a process."}
  - {id: FR-185-CON-2, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "Quoin SHALL NOT vendor the in-toto schema or protobuf files."}
findings: []
```

+++
