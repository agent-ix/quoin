---
id: FR-186
title: "Carry Quoin release verification evidence with the tag attestation"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-030"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-112"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-185"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-069"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: "requires"
---

# FR-186: Carry Quoin release verification evidence with the tag attestation

## Description

Quoin SHALL run its declared release-verification suite at the signed tag and
carry the resulting evidence store to the shared release-attestation workflow
as the `release-evidence` artifact. The suite declaration and binding registry
are maintained in source control. The tag build produces fresh run-specific
bindings and a run record, which are retained only in that tag's evidence
artifact. [FR-030](./FR-030-evidence-store.md) keeps run facts separate from
authored suite declarations and does not add a commit field to a run record.
The artifact provenance identifies the signed tag, its resolved source commit,
and the workflow run that produced the evidence. The shared consumer uses the
retained store under [Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069)
and refuses to write a Test Result when the evidence is absent, invalid,
failing, or stale.

## Inputs

- The signed `v*` release tag selected by the Quoin tag-attestation workflow.
- The declared `SUITE-001` release-verification test suite in
  [`spec/evidence/suites.md`](../evidence/suites.md).

## Outputs

- A workflow artifact named `release-evidence` containing the maintained suite
  declaration and binding registry, fresh run-specific bindings and the
  `SUITE-001` run record, plus provenance tying that evidence to the signed tag,
  resolved source commit, and producing workflow run.
- A call to the shared Engineering Assurance workflow with
  `evidence_artifact` equal to `release-evidence`.

## Behavior

- For each pushed `v*` tag, the Quoin tag-attestation workflow SHALL run
  `make evidence` at the tagged source revision for `SUITE-001`.
- After `SUITE-001` passes and its run record is written, the Quoin
  tag-attestation workflow SHALL upload the maintained suite declaration and
  binding registry together with the run-specific bindings, run record, and
  provenance as `release-evidence`. Run-specific bindings and records SHALL
  remain only in that artifact and SHALL NOT be committed.
- The `release-evidence` artifact SHALL identify the signed tag name, the
  resolved commit checked out for that tag, and the producing workflow run;
  those values SHALL identify the source and run represented by the uploaded
  bindings and record without adding source identity fields to the FR-030 run
  record.
- The Quoin tag-attestation workflow SHALL pass
  `evidence_artifact: release-evidence` to the shared Engineering Assurance
  workflow.
- The attestation job SHALL consume the uploaded artifact without checking out
  or running Quoin repository code, in accordance with NFR-011.
- If the suite fails, its run record is absent, or the evidence artifact cannot
  be uploaded, the workflow SHALL fail before the shared attestation call.
- The tag-attestation workflow SHALL NOT modify the committed suite or binding
  registries and SHALL NOT commit generated run records to the repository.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-186-AC-1 | The maintained suite declaration identifies `FR-186-AC-1` through `FR-186-AC-6` as required release bindings for `SUITE-001`; a workflow test confirms that the Quoin `v*` tag-attestation build runs `make evidence` at the tagged source and produces a passing binding for each listed criterion before upload. | Test |
| FR-186-AC-2 | A workflow test confirms that the shared Engineering Assurance call receives `evidence_artifact: release-evidence` only after suite execution and evidence upload succeed, and cannot start when either fails. | Test |
| FR-186-AC-3 | Static inspection confirms that the OIDC-enabled attestation job consumes the evidence artifact and runs no checked-out Quoin code. | Inspection |
| FR-186-AC-4 | After a tag-attestation run, the uploaded artifact contains the maintained suite declaration and binding registry, the fresh run-specific bindings and run record, and no generated run-specific bindings or records are committed to the repository. | Test |
| FR-186-AC-5 | A workflow test confirms that the artifact provenance identifies the signed tag, its resolved source commit, and the producing workflow run, and that each identifies the source and run represented by the uploaded bindings and run record. | Test |
| FR-186-AC-6 | A consumer integration test confirms that absent or invalid evidence, a missing required binding or run record, a failed required criterion, or evidence stale for the current signed-tag workflow produces no Test Result and no release attestation, and reports the applicable failure. | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md), [FR-112](./FR-112-npm-distribution-from-release-assets.md), [FR-185](./FR-185-in-toto-test-result-export.md), and [Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069) and [FR-070](ix://agent-ix/engineering-assurance/FR-070).
- **Downstream**: the shared release-attestation workflow defined by Engineering Assurance FR-070.
