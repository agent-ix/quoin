---
id: SR-2026112016223475846355074
title: SACM exporter code review
type: SpecReview
analysis: code-review
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

Reviewed PR #1062 at e13e15728433593ee66f1dac3d38ce84a441ee6b against refreshed main e3ad38e22131b9fb4ffff088d82dae4aec5a6424. PLAT-1647 and PLAT-1648 cover the argument and run artifact paths of the same exporter.

## Verdict

FAIL. The derived command loses non-supporting evidence before export, and the strategy counter-claim output needs an agreed normative mapping.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Derived SACM loses stale, suspect, and undischarged evidence before serialization. The case builder marks these obligation leaves open, and the GSN builder turns an open solution into an undeveloped goal and clears its LeafEvidence. SACM then has neither the selected run nor a solution from which to emit AssertedEvidence. The tagged test starts with synthetic solution nodes, so it bypasses this production boundary. Resolve the FR-221 and FR-228 contract and verify the result through the derived exporter. | rust/crates/quoin-assurance/src/gsn.rs:481 |
| FND-002 | high | A strategy-targeted challenge yields a counter AssertedInference ending at ArgumentReasoning. FR-221 and WF-8 allow that GSN target, while FR-226 requires the challenged Claim. OMG SACM 2.3 §11.14 describes inference endpoints as Assertions, and ArgumentReasoning is outside that subtype. The machine-readable superclass accepts ArgumentAsset, so the current metamodel checker would allow the mismatch. Specify a faithful mapping for strategy targets and cover goal, strategy, and assumption challenges. | rust/crates/quoin-assurance/src/sacm.rs:299 |
| FND-003 | medium | SACM treats a resolved authored evidence reference as undischarged whenever its GSN node carries a reference instead of LeafEvidence. The authored GSN path intentionally supplies only references, including supported ones, and the SACM exporter consequently changes the linked supported Claim to needsSupport. Make the contract distinguish a resolved authored reference without a run selection from a selected run that is missing, and verify both through the authored entry point. | rust/crates/quoin-assurance/src/sacm/artifact.rs:58 |
| FND-004 | medium | The retained run path is stored in Resource.name, with a grouping reference to the Artifact. An importer following SACM 2.3 §12.10 has no Resource.location to read. The official machine-readable XML lacks the location property described in the normative PDF, so the current passing XML-metaclass check does not settle the location contract. Document and verify the agreed representation before claiming normative location interoperability. | rust/crates/quoin-assurance/src/sacm/artifact.rs:204 |
| FND-005 | medium | The exporter reports a strategy-to-strategy support edge as unmapped, then traverses the same edge to emit a direct inference from the inner strategy's child to the outer goal with only the inner reasoning. An authored chain G1 → S1 → S2 → G2 loses S1 from the emitted inference. Define the nested reasoning behavior and test that an edge reported as unmapped has no approximated stand-in. | rust/crates/quoin-assurance/src/sacm.rs:329 |

## Coverage

- The candidate builds on FR-226’s approved acceptance placement correction. Its PR description accurately discloses the missing signed-acceptance upstream record. STD-723 blocks PLAT-1871. Full FR-226 and M2 conformance still require that work, or the gate and delivery sequence need explicit approval.
- The fixture validates one root and package containment, closed reference ids and namespaces, open claim declarations, literal SHA-256 of retained UTF-8 bytes, content deduplication, and stable ordering. Authored CLI tests cover usage error 3, stderr omission diagnostics, report bytes, and XMI withholding on report failure. The source assembles XMI in memory and publishes the requested report before it returns a response carrying XMI.
- The shipped dependency change pins quick-xml 0.42.0. The downloaded crate’s SHA-256 matches Cargo.lock and its manifest declares MIT. thiserror and tempfile were already in the workspace. The diff adds no copied SACM metamodel and does not lower an existing test gate.
- The author’s candidate log /tmp/quoin-plat-1647-1648-rebased-focused.log records exit 0 for eight SACM tests, including the external metamodel check. /tmp/quoin-plat-1647-1648-gsn-regression.log shows one passing GSN regression. /tmp/quoin-plat-1647-1648-unset-metamodel.log records the expected directly invoked test failure. Those checks substantiate the fixtures they execute and do not exercise evidence in the normal derived entry point or strategy-targeted counter-claims.
- Reviewer checks passed for changed Rust formatting on the pinned workspace toolchain (format.log, exit 0), git diff --check (git-diff-check.log, exit 0), and structure of the external OMG UML file (normative-metamodel-inspection.log, exit 0). The unset make target rejected the invocation (metamodel-unset.log, exit 2). The candidate binaries in the author logs were no longer present at the recorded target. Aggregate, mutation, and long test runs were excluded by the review brief.
- OMG SACM 2.3 §§11.13, 11.14 and 12.10 govern endpoint and Resource semantics. The external model documents broad base reference types, and the normative PDF documents narrower inference semantics and a location association absent from the XML. [OMG SACM 2.3 PDF](https://www.omg.org/spec/SACM/2.3/PDF)

## Reviewed criteria

| Id | Path | Role | Excerpt |
| --- | --- | --- | --- |
| FR-221-AC-2 | spec/functional/FR-221-gsn-model-export.md | examined | A derived fixture exports goal, strategy, and solution nodes; `supported_by` edges preserve case order; every node's `source` equals its Quoin source id; and solution evidence matches the fixture's `LeafEvidence`. |
| FR-221-AC-3 | spec/functional/FR-221-gsn-model-export.md | examined | An open derived obligation exports as an undeveloped open goal with the auditor reason verbatim; a goal without children exports as undeveloped; and a requirement with no reachable claim is listed in `unreachable`, not as a node. |
| FR-221-AC-5 | spec/functional/FR-221-gsn-model-export.md | examined | An authored fixture exports its top claim, reasoning, criteria, evidence, assumptions, participants, and authority using the stated mappings; an undecided criterion is an undeveloped goal, and a missing or unresolved top-claim evidence reference keeps the root open with its reason. |
| FR-221-AC-6 | spec/functional/FR-221-gsn-model-export.md | examined | A currently open authored challenge exports as a `counter_claim` node and one `challenges` edge to its goal, strategy, or assumption target; it is absent from `context` nodes and has no `in_context_of` or `supported_by` challenge edge; its source id, statement, owner, current reason, declared status, resolution references, and expiry are preserved. |
| FR-222-AC-1 | spec/functional/FR-222-gsn-model-well-formedness.md | examined | A model with a `supported_by` edge from a solution to a goal is refused under WF-1, names both endpoint ids, writes no model to stdout, and exits with failure. |
| FR-222-AC-2 | spec/functional/FR-222-gsn-model-well-formedness.md | examined | A model with an `in_context_of` edge from an assumption to a goal is refused under WF-2, names both endpoint ids, writes no model to stdout, and exits with failure. |
| FR-222-AC-5 | spec/functional/FR-222-gsn-model-well-formedness.md | examined | A goal or strategy with no `supported_by` child and no `undeveloped: true` is refused under WF-5 and names that node id. |
| FR-222-AC-6 | spec/functional/FR-222-gsn-model-well-formedness.md | examined | A model with duplicate node ids or an edge endpoint absent from `nodes` is refused under WF-6, names the duplicate or dangling ids, and writes no model to stdout. |
| FR-222-AC-8 | spec/functional/FR-222-gsn-model-well-formedness.md | examined | A counter-claim with no `challenges` edge, more than one such edge, or an edge to a solution is refused under WF-8 and names the counter-claim and invalid target ids. |
| FR-222-AC-9 | spec/functional/FR-222-gsn-model-well-formedness.md | examined | A model with multiple independent violations reports each applicable rule id and all affected node ids in one failed export, with no partial model on stdout. |
| FR-223-AC-3 | spec/functional/FR-223-assurance-case-context-and-counter-claims.md | examined | An authored argument with two assumptions, one resolved challenge, one open challenge, and participants produces two assumptions, one justification, one counter-claim, and context for the participants with all declared source fields preserved. |
| FR-223-AC-4 | spec/functional/FR-223-assurance-case-context-and-counter-claims.md | examined | An open challenge is represented only as a counter-claim; it is never a Context node or a supporting node, and its top claim remains open. |
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
