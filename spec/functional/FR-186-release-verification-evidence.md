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
---

# FR-186: Carry Quoin release verification evidence with the tag attestation

## Description

Quoin SHALL run its declared release-verification suite at the signed tag and
carry the resulting evidence store to the shared release-attestation workflow
as the `release-evidence` artifact. The suite registry and binding registry
are committed; the run record is produced for that tag build and retained only
in its evidence artifact. [FR-030](./FR-030-evidence-store.md) keeps run facts
separate from authored suite declarations. The shared consumer
uses the retained store under [Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069)
and refuses to write a Test Result when evidence is absent or unusable.

## Inputs

- The signed `v*` release tag selected by the Quoin tag-attestation workflow.
- The declared `SUITE-001` release-verification test suite in
  [`spec/evidence/suites.md`](../evidence/suites.md).

## Outputs

- A workflow artifact named `release-evidence` containing the committed suite
  and binding registries and the fresh `SUITE-001` run record.
- A call to the shared Engineering Assurance workflow with
  `evidence_artifact` equal to `release-evidence`.

## Behavior

- For each pushed `v*` tag, the Quoin tag-attestation workflow SHALL run
  `make evidence` at the tagged source revision for `SUITE-001`.
- After `SUITE-001` passes and its run record is written, the Quoin
  tag-attestation workflow SHALL upload the committed suite and binding
  registries and fresh run record as `release-evidence`.
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
| FR-186-AC-1 | A workflow test confirms that the Quoin `v*` tag-attestation build runs `make evidence` for the declared `SUITE-001` at the tagged source and uploads `spec/evidence/` as the artifact `release-evidence` only after the suite succeeds. | Test |
| FR-186-AC-2 | A workflow test confirms that the shared Engineering Assurance call receives `evidence_artifact: release-evidence` and cannot start when suite execution or evidence upload fails. | Test |
| FR-186-AC-3 | Static inspection confirms that the OIDC-enabled attestation job consumes the evidence artifact and runs no checked-out Quoin code. | Inspection |
| FR-186-AC-4 | After a tag-attestation run, the uploaded artifact contains the committed suite and binding registries and the fresh run record; the run record is not committed to the repository. | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md), [FR-112](./FR-112-npm-distribution-from-release-assets.md), [FR-185](./FR-185-in-toto-test-result-export.md), and [Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069).
- **Downstream**: the shared release-attestation workflow defined by Engineering Assurance FR-070.
