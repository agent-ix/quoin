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

FR-186 specifies the Quoin repository's release-CI workflow and evidence
handoff only. It does not change Quoin product behavior: the Quoin library and
engine do not create, sign, or retain attestations or run records. For a pushed
signed `v*` tag, repository CI runs the declared suite, produces fresh
run-specific bindings and a run record as ephemeral workflow output, uploads
them only in that tag's `release-evidence` artifact, and passes the artifact to
the shared Engineering Assurance attestation workflow. The suite declaration
and binding registry remain maintained in source control; per-tag run records
and generated bindings are never committed. [FR-030](./FR-030-evidence-store.md)
keeps run facts separate from authored suite declarations and does not add a
commit field to a run record. The artifact provenance identifies the signed
tag, its resolved source commit, and the workflow run that produced the
evidence. The artifact also contains a version-2 DSSE execution-provenance
envelope, signed by the source release workflow, that binds that commit to the
complete computed Test Matrix and exact run-record bytes. The shared consumer
authenticates this envelope under
[Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069)
before passing it to Quoin's export. It refuses to write a Test Result when
the evidence is absent, invalid, failing, or stale.

## Inputs

- The signed `v*` release tag selected by the Quoin tag-attestation workflow.
- The declared `SUITE-001` release-verification test suite in
  [`spec/evidence/suites.md`](../evidence/suites.md).

## Outputs

- A workflow artifact named `release-evidence` containing the maintained suite
  declaration and binding registry, the complete computed Test Matrix for the
  tagged source revision, fresh run-specific bindings, the `SUITE-001` run
  record, and `execution-provenance.dsse.json` binding the revision, matrix,
  and run-record bytes.
- A call to the shared Engineering Assurance workflow with
  `evidence_artifact` equal to `release-evidence`.

## Behavior

- For each pushed `v*` tag, the Quoin tag-attestation workflow SHALL run
  `make evidence` at the tagged source revision for `SUITE-001`.
- After `SUITE-001` passes and its run record is written, the Quoin
  tag-attestation workflow SHALL upload the maintained suite declaration and
  binding registry, the complete computed Test Matrix, run-specific bindings,
  run record, and signed execution provenance as `release-evidence`.
  Run-specific bindings, matrix, provenance, and records SHALL remain only in
  that artifact and SHALL NOT be committed.
- The version-2 DSSE payload SHALL use payload type
  `application/vnd.agent-ix.execution-provenance.v2+json` and contain exactly
  `version`, `sourceRevision`, `testMatrix`, and `runs`. `version` SHALL equal
  `2`; `sourceRevision` SHALL be the full resolved commit for the signed tag;
  `testMatrix` SHALL be the complete canonical Quire Test Matrix computed at
  that commit; and `runs` SHALL be sorted by suite id and contain exactly
  `suite`, `recordPath`, and lowercase `sha256` of each run record's exact
  bytes. The source release workflow identity SHALL sign the payload through
  the configured OIDC trust path.
- The `release-evidence` artifact SHALL identify the signed tag name, the
  resolved commit checked out for that tag, and the producing workflow run;
  those values SHALL identify the source and run represented by the uploaded
  bindings and record without adding source identity fields to the FR-030 run
  record.
- The tag-attestation workflow SHALL compute the Test Matrix from the same
  resolved source commit used to run the declared suites, and SHALL create the
  signed provenance only after those runs and their retained records exist.
- The workflow SHALL include each run record's exact relative path and digest
  in the provenance. It SHALL refuse artifact upload if the matrix, record, or
  provenance is missing or if a computed digest does not match the bytes being
  uploaded.
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
| FR-186-AC-7 | A workflow test confirms that `release-evidence` contains the version-2 DSSE envelope with the exact payload type and fields, a complete matrix from the tagged commit, sorted run references, and digests matching each uploaded run record; an altered or omitted record is detected. | Test |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md), [FR-112](./FR-112-npm-distribution-from-release-assets.md), [FR-185](./FR-185-in-toto-test-result-export.md), and [Engineering Assurance FR-069](ix://agent-ix/engineering-assurance/FR-069) and [FR-070](ix://agent-ix/engineering-assurance/FR-070).
- **Downstream**: the shared release-attestation workflow defined by Engineering Assurance FR-070.
