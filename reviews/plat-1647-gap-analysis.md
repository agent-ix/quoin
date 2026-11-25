---
id: SR-2026112016223475846355073
title: SACM exporter gap analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quoin; PR 1062; PLAT-1647 and PLAT-1648; Makefile, rust/crates/quoin-assurance, rust/crates/quoin-cli, SACM requirements
review_set: subset
relationships:
- target: ix://agent-ix/quoin/FR-226
  type: references
- target: ix://agent-ix/quoin/FR-227
  type: references
- target: ix://agent-ix/quoin/FR-228
  type: references
- target: ix://agent-ix/quoin/FR-221
  type: references
- target: ix://agent-ix/quoin/FR-222
  type: references
- target: ix://agent-ix/quoin/FR-225
  type: references
- target: ix://agent-ix/quoin/NFR-045
  type: references
- target: ix://agent-ix/quoin/NFR-046
  type: references
---

## Summary

Audited the SACM scope in PR #1062 at e13e15728433593ee66f1dac3d38ce84a441ee6b from the computed matrix and the command's real source and test surface. Plan completion was not assessed.

## Verdict

FAIL. Signed acceptance and offline verification lack test binders, and the synthetic evidence test bypasses the production derived constructor.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The computed matrix has no binder for FR-226-AC-9 or any FR-225 criterion. GsnCase has no signed_acceptance field, and SACM cannot produce the specified acceptance Artifact. The PR discloses the boundary and Linear tracks the caller and verifier contract in STD-723, which blocks PLAT-1871. An explicit gate or scope decision can sequence this work after the base exporter. The current head cannot claim complete FR-226 or M2 conformance. | rust/crates/quoin-assurance/src/gsn.rs:14 |
| FND-002 | medium | The computed matrix has no binder for the zero-network runtime criterion. The focused logs exercise both exporters under ordinary process networking. The export code adds local file I/O only, but no test evidence verifies the full command under the offline condition prescribed by NFR-045. | spec/non-functional/NFR-045-assurance-exports-run-offline.md:29 |
| FND-003 | high | The only FR-228 binder constructs a GsnCase with preexisting solution nodes and cannot detect that the production derived GSN constructor removes solution evidence on open leaves. The only new CLI success fixture is an authored argument without evidence references. Add a production-path check that begins with the evidence store and confirms the selected artifact and non-supporting or missing evidence are visible on stdout. | rust/crates/quoin-assurance/tests/tc_sacm_export.rs:225 |

## Coverage

- Reconciliation used quoin matrix 0.36.8 and quire coverage 0.52.0 (engine 0.62.2), both exit 0. Logs are matrix.json, matrix.stderr, coverage.json, and coverage.stderr in this scratchpad. The coverage command emitted pre-existing symbol-id diagnostics on five unrelated conditional declarations.
- FR-225 through FR-228 and NFR-045/046 have 17 tagged criteria, one opt-in-only binder, seven untagged test criteria, and three inspection criteria without source binders. The computed matrix has no retained run evidence for these rows. The opt-in-only FR-226-AC-7 binder runs against the user-supplied normative file in the candidate log. FR-226-AC-8 and the two NFR-046 measures are inspection criteria, and this review inspected the PR scope and dependency diff.
- Repository-wide inventory has 1,682 tagged rows, six opt-in-only rows, 649 untagged rows, 169 method-without-symbol rows, and 36 retired rows. The findings in this review apply to the PR and its interface dependencies.
- Four new behaviors were inventoried: CLI SACM export, optional report publication, the library SACM export, and the explicit metamodel command. Each has an owning requirement. No source stubs or empty test bodies were found in the changed surface. The FR-228 production-entry gap is a tagged-test reachability defect.
- The offline namespace preflight was unavailable because unshare -n exited 1 with Operation not permitted. The authored and derived CLI paths were inspected for new runtime network I/O. The prescribed end-to-end offline check is still outstanding.
- Plan completion: not assessed
- Optional gap-analysis semantic review was skipped. The separate code-review pass examined requirement-to-code fidelity.

## Reviewed criteria

| Id | Path | Role | Excerpt |
| --- | --- | --- | --- |
| FR-225-AC-1 | spec/functional/FR-225-signed-acceptance-of-case-content.md | context_only | A verified EA approval over the exact case content identity appears in both GSN and SACM output with signer, authority, approval reference, and approval instant preserved. |
| FR-225-AC-2 | spec/functional/FR-225-signed-acceptance-of-case-content.md | context_only | An approval tag for different content, an unverifiable tag, or an expired approval emits no accepted record and reports the corresponding reason. |
| FR-225-AC-3 | spec/functional/FR-225-signed-acceptance-of-case-content.md | context_only | When EA approval verification is unavailable, export contains no positive acceptance assertion and still preserves the underlying case. |
| FR-225-AC-4 | spec/functional/FR-225-signed-acceptance-of-case-content.md | context_only | The content identity is stable for byte-identical canonical GSN models, changes when case content changes, and is identical in GSN and SACM output for the same case. |
| FR-225-AC-5 | spec/functional/FR-225-signed-acceptance-of-case-content.md | context_only | Export performs no Git subprocess and does not assign product-risk, safety, or compliance acceptability from the human acceptance. |
| FR-226-AC-1 | spec/functional/FR-226-sacm-export.md | examined | A fixture containing each GSN node kind produces the stated SACM elements and relationships under an AssuranceCasePackage root, and all references resolve to an existing XMI id. |
| FR-226-AC-2 | spec/functional/FR-226-sacm-export.md | examined | An open or undeveloped goal maps to a claim needing support; it is never asserted, including when the source is a counter-claim. |
| FR-226-AC-3 | spec/functional/FR-226-sacm-export.md | examined | A direct goal-to-goal edge and a strategy-mediated edge produce their distinct AssertedInference structures with stable source ids. |
| FR-226-AC-4 | spec/functional/FR-226-sacm-export.md | examined | Re-export of unchanged GSN input produces byte-identical UTF-8 XMI with stable element and relationship ordering. |
| FR-226-AC-5 | spec/functional/FR-226-sacm-export.md | examined | Combining `--sacm` with any incompatible output flag returns a usage error and writes no XMI. |
| FR-226-AC-6 | spec/functional/FR-226-sacm-export.md | examined | A well-formedness failure in the GSN input writes no XMI and preserves the named WF rule and node ids in the error report. |
| FR-226-AC-7 | spec/functional/FR-226-sacm-export.md | examined | Local expectations validate the XMI shape; the explicit metamodel-conformance check compares the output against the normative model at `QUOIN_SACM_METAMODEL` and rejects an unset path. |
| FR-226-AC-8 | spec/functional/FR-226-sacm-export.md | examined | No external interop application is named as verified until PLAT-1649 has successful fixture-import evidence with application name, version, and licence. |
| FR-226-AC-9 | spec/functional/FR-226-sacm-export.md | examined | A GSN case with verified `signed_acceptance` and an open claim yields exactly one labeled acceptance Artifact via the root's `artifactPackage` and its `artifactElement` containment. No argument element references it. Exporting the case with `signed_acceptance` omitted yields no acceptance Artifact and identical Claim declarations. |
| FR-227-AC-1 | spec/functional/FR-227-sacm-unmapped-report.md | examined | A fixture with one unreachable requirement and one unmapped source item writes both diagnostics to stderr and records both in the JSON report with their ids and reasons. |
| FR-227-AC-2 | spec/functional/FR-227-sacm-unmapped-report.md | examined | Unreachable and unmapped source items produce no SACM stand-in elements and are not silently dropped. |
| FR-227-AC-3 | spec/functional/FR-227-sacm-unmapped-report.md | examined | Repeating the report for unchanged input produces byte-identical JSON with deterministic item order. |
| FR-227-AC-4 | spec/functional/FR-227-sacm-unmapped-report.md | examined | An unwritable report path returns an error and does not report successful report retention. |
| FR-228-AC-1 | spec/functional/FR-228-sacm-evidence-artifact-content-identity.md | examined | Two solutions with identical run-record bytes share one Artifact; a third solution with different bytes gets a distinct Artifact even if its suite id matches. |
| FR-228-AC-2 | spec/functional/FR-228-sacm-evidence-artifact-content-identity.md | examined | Each Artifact exposes the selected content identity, suite id, verdict, relative run-record path, and timestamp as metadata only; its XMI id depends on content identity and not timestamp. |
| FR-228-AC-3 | spec/functional/FR-228-sacm-evidence-artifact-content-identity.md | examined | For stale, suspect, and undischarged evidence, the AssertedEvidence has `isCounter=false` and the linked Claim has `needsSupport=true`. |
| FR-228-AC-4 | spec/functional/FR-228-sacm-evidence-artifact-content-identity.md | examined | A solution with no run remains visible as undischarged evidence and has no invented Artifact. |
| FR-228-AC-5 | spec/functional/FR-228-sacm-evidence-artifact-content-identity.md | examined | The XMI contains no absolute host path or Git revision id, and it contains no digest other than the declared content identities. |
| FR-228-AC-6 | spec/functional/FR-228-sacm-evidence-artifact-content-identity.md | examined | The XMI does not embed any run-record bytes. |
| FR-228-AC-7 | spec/functional/FR-228-sacm-evidence-artifact-content-identity.md | examined | Re-export with unchanged evidence content produces byte-identical XMI and stable content-derived Artifact ids. |
| NFR-045-M-1 | spec/non-functional/NFR-045-assurance-exports-run-offline.md | examined | Runtime network requests made by either export \| 0 \| 0 \| Test |
| NFR-046-M-1 | spec/non-functional/NFR-046-no-copied-sacm-metamodel.md | examined | Copies of the SACM metamodel in the quoin repository \| 0 \| 0 \| Inspection |
| NFR-046-M-2 | spec/non-functional/NFR-046-no-copied-sacm-metamodel.md | examined | New runtime dependencies outside the workspace other than the selected XML writer \| 0 \| 0 \| Inspection |
