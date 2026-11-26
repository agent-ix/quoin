---
id: FRR-002
title: "Proposed Quoin features that can make false assurance claims"
type: FeatureRiskRating
scope: supplier
rating: high_process_risk
status: proposed
relationships:
  - target: ix://agent-ix/quoin/IU-001
    type: uses_intended_use
  - target: ix://agent-ix/quoin/FR-012
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-029
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-030
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-032
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-033
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-034
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-035
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-036
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-037
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-039
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-040
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-042
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-046
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-060
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-064
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-065
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-069
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-087
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-088
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-093
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-094
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-107
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-108
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-110
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-111
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-115
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-116
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-120
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-133
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-135
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-152
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-153
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-164
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-174
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-175
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-185
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-186
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-188
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-190
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-194
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-196
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-208
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-211
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-214
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-217
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-218
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-225
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-235
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-236
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-237
    type: rates_feature
---
# Proposed Quoin features that can make false assurance claims

## Rationale

These are proposed supplier-side ratings for IU-001, unsigned and unaccepted pending a human decision. Each retained high feature can itself falsely state that verification, traceability, discharge, or approval occurred. The feature-specific false-claim reason follows.

### Retained high false-claim reasons

- **FR-012** — `catalog validate` can report a catalog clean while duplicate type declarations remain.
- **FR-029** — the Quire adapter can accept an invalid engine payload as valid input to later verification output.
- **FR-030** — evidence binding can attach a passing result to an obligation when only a failing or unrelated symbol was observed.
- **FR-032** — the evidence auditor can label stale or vacuous evidence usable for an obligation.
- **FR-033** — an evidence adapter can turn an adverse producer result into a passing run entry.
- **FR-034** — the findings adapter can treat a scan with findings as clean verification evidence.
- **FR-035** — the configuration-coverage report can claim required combinations were exercised when they were not.
- **FR-036** — architecture conformance can report a violated boundary as passing.
- **FR-037** — vocabulary completeness can report a required concept present when it is absent.
- **FR-039** — the mutation gate can report the declared threshold met when the computed score is below it.
- **FR-040** — assurance-case construction can mark a claim supported from evidence that does not support it.
- **FR-042** — agent-evaluation evidence intake can count a non-passing scenario as a passed verification result.
- **FR-046** — clause-discharge accounting can place an unsupported clause in the discharged partition.
- **FR-060** — operational-evidence intake/reporting can mark a required control exercise discharged without a valid, in-window observation.
- **FR-064** — a proof attestation can bind a proof result to the wrong candidate, obligation, command, or retained output.
- **FR-065** — change-assurance verification can issue a valid receipt for an unproved or mismatched candidate.
- **FR-069** — a campaign adapter can map an adverse test result to a passing verification entry.
- **FR-087** — structural conformance measurement can report a document as passing when Quire reports a blocking diagnostic.
- **FR-088** — semantic conformance measurement can report a positive result when the measurement did not run or failed.
- **FR-093** — evidence-producer trust can accept a result outside the selected producer use or after an invalidating change.
- **FR-094** — evidence-independence assessment can report an obligation independent when a selected independence dimension is not met.
- **FR-107** — the ratchet report can claim a measurement held when it regressed from the governing bound.
- **FR-108** — the independent measurement verifier can accept a collection that violates the plan's decision rule.
- **FR-110** — a measurement verdict can pass despite a changed or unresolved protected apparatus.
- **FR-111** — change assurance can credit a candidate that changes protected measurement apparatus.
- **FR-115** — the evidence-backed matrix can show a criterion backed by a passing result that does not bind to it.
- **FR-116** — retained Quire evidence qualification can credit the wrong act, obligation, or method as positive evidence.
- **FR-120** — the test-result adapter can attach a criterion trace id not present on that test result.
- **FR-133** — discharge-fact intake/reporting can mark a binding clause complete without an eligible current fact.
- **FR-135** — the mapping matrix can label a clause compliant when its discharge source does not.
- **FR-152** — an analysis result can be bound to an obligation whose id the recorder did not state.
- **FR-153** — an unqualified or non-passing analysis result can count toward an obligation.
- **FR-164** — the anomaly release gate can pass while an in-scope blocking anomaly remains open.
- **FR-174** — the per-clause report can state compliance without a current discharge fact.
- **FR-175** — the anomaly checker can report an anomaly fixed without its required verifying test passing.
- **FR-185** — Test Result export can assert a passed obligation for a missing, failed, or untraced test.
- **FR-186** — a release tag attestation can carry verification evidence for a different release candidate.
- **FR-188** — formal-rule state can report an unsigned or incorrectly signed rule as approved.
- **FR-190** — formal-rule analysis can credit a draft as an approved claim.
- **FR-194** — the dossier trace matrix can show a criterion traced to evidence that does not bind to it.
- **FR-196** — the verification report can show a criterion backed by evidence when the matrix reports no such binding.
- **FR-208** — clause discharge can omit a required approval role and still report the fact eligible.
- **FR-211** — clause discharge can resolve an invalid or stale evidence reference or unverified approval as discharged.
- **FR-214** — decision attribution can report a signed decision by a principal different from the named decider.
- **FR-217** — the validation report can show a passing result or requirement coverage that the retained runs do not support.
- **FR-218** — sign-off verification can report acceptance for an invalid/unmapped signer or wrong principal/trailer.
- **FR-225** — assurance-case export can record human acceptance without a verified tag for the exact case content.
- **FR-235** — self-test execution can report a case passing when the installed command's observed result differs from the case oracle.
- **FR-236** — run-record retention can persist a different outcome, trace set, or version from the execution it records.
- **FR-237** — independent kit verification can report a release verified when its required tests or seeded-fault checks failed.

## Assurance activities

| Activity | Acceptance criterion |
|---|---|
| Exercise command parsing and module resolution with valid, missing, and conflicting inputs; inspect the selected command, contract, and output. | FR-012-AC-1 |
| Run evidence-store and change-assurance checks with repeated, changed, and invalid inputs; compare retained bytes and refusal status. | FR-030-AC-1; FR-030-AC-2; FR-064-AC-1; FR-065-AC-1 |
| Exercise traceability, audit, and measurement calculations against small enumerated fixtures and independently compare the resulting statuses and rates. | FR-032-AC-1; FR-035-AC-1; FR-037-AC-1; FR-107-AC-1; FR-115-AC-1; FR-153-AC-1 |
| Verify discharge and approval outcomes with complete and incomplete evidence and with accepted and rejected signer inputs. | FR-046-AC-1; FR-093-AC-1; FR-188-AC-1; FR-208-AC-1; FR-211-AC-1; FR-225-AC-1 |
| Compare generated dossier and assurance exports with their source records, including missing-source and malformed-model cases. | FR-040-AC-1; FR-194-AC-1; FR-196-AC-1 |
| Exercise the self-test runner and report with passing, failing, timed-out, and missing-case inputs; inspect every retained run field. | FR-217-AC-1; FR-218-AC-1; FR-235-AC-1; FR-236-AC-1 |
| Exercise independent kit verification with complete and incomplete runs, identity conflicts, and seeded mismatches; inspect the overall outcome and findings. | FR-237-AC-1; FR-237-AC-2; FR-237-AC-3; FR-237-AC-4; FR-237-AC-5; FR-237-AC-6 |
