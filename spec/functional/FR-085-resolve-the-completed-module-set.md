---
id: FR-085
title: "Resolve the completed module set and its contract surfaces"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-070"
    type: "depends_on"
---

# FR-085: Resolve the completed module set and its contract surfaces

## Description

The corpus measurement SHALL resolve every module of a declared required module set at the exact
revision declared for it, recording each module's resolved commit, manifest, declared types and
contract-surface digests, and SHALL refuse to publish a rate when a required module is unresolved.

## Rationale

The catalog pins in `default-modules.yaml` are the previous release of each module, not the completed
contract of this wave: `spec-artifacts-iso`'s mappings declaration does not exist at the pinned tag
`v0.18.0` at all. Measuring the catalog pin would publish a number about a contract nobody is
proposing to promote. Naming the required set, and refusing without it, is what stops a run that
resolved one module from satisfying every criterion.

## Inputs

- A declared required module set: for each module, a repository path, a revision and the module's
  data subdirectory.
- The `default-modules.yaml` catalog declaration, read for comparison only.

## Outputs

- A module record per module: name, repository origin, requested revision, resolved commit, manifest
  version, declared artifact types, declared object types, and a digest per contract surface.
- A `toolchain` record naming the measuring engine's version and source revision, the CLI's version
  and source revision, and each module's declared `semantic_core` version.
- A refusal record for any declared module the measurement could not resolve.

## Behavior

- The measurement SHALL read each module's files at the declared revision from the module
  repository's Git object store rather than from its working tree, so that a dirty checkout cannot
  change what was measured. This is also what keeps the measurement independent of
  agent-ix/quoin#347, under which no artifact-type module can be installed at all.
- The measurement SHALL record the resolved commit of every declared revision, including when the
  declared revision is a tag or another mutable ref.
- If a required module's declared revision does not resolve, then the measurement SHALL exit non-zero
  before reading any corpus document.
- The measurement SHALL record, for each module, the digest of its manifest, of each declared JSON
  Schema and of its mappings declaration when it publishes one.
- When a module's manifest declares a `data_schema` digest for a type, the measurement SHALL compare
  that declared digest with the digest of the schema file it read.
- If a declared `data_schema` digest differs from the digest of the schema file, then the
  measurement SHALL record a module finding naming both digests.
- Where a module publishes no Markdown mappings declaration, the measurement SHALL record
  `mappings: absent` against it.
- The measurement SHALL record, for each module, whether the revision measured equals the revision
  pinned for that module in `default-modules.yaml`.
- The measurement SHALL record every diagnostic the engine emits while loading the module set as a
  module finding rather than as a failure of any corpus document.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-085-CON-1 | Module resolution SHALL NOT check out, fetch or otherwise mutate a module repository. | Safety | Test |
| FR-085-CON-2 | An unresolved required module SHALL NOT be treated as a module declaring no types. | Interface | Test |
| FR-085-CON-3 | The required module set SHALL be a declared input naming every module whose schema this campaign completed. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-085-AC-1 | Each declared module resolves to a commit and records its manifest version, declared artifact types and declared object types. | Test (TC-1506) |
| FR-085-AC-2 | Each contract surface of each module carries a SHA-256 digest in the module record. | Test (TC-1507) |
| FR-085-AC-3 | Content read for a module is byte-identical to that module's content at the declared revision when the repository working tree carries an unrelated uncommitted edit, and no Git ref, index or object of that repository changes during the run. | Test (TC-1508) |
| FR-085-AC-4 | An unresolvable required revision exits non-zero before any corpus document is read, naming the module and the revision. | Test (TC-1509) |
| FR-085-AC-5 | A module publishing no mappings declaration records `mappings: absent` and still contributes its declared object types. | Test (TC-1510) |
| FR-085-AC-6 | Each module record states whether the measured resolved commit equals the commit its `default-modules.yaml` ref resolves to, and the report names every module where it does not. | Test (TC-1511) |
| FR-085-AC-7 | A `data_schema` digest that disagrees with the schema file read yields a module finding naming both digests. | Test (TC-1568) |
| FR-085-AC-8 | The toolchain record names the engine version and source revision, the CLI version and source revision, and each module's `semantic_core` version. | Test (TC-1569) |
| FR-085-AC-9 | A module-load diagnostic, such as one name declared as both an archetype and an artifact type, is recorded as a module finding and fails no corpus document. | Test (TC-1570) |
| FR-085-AC-10 | A required module set missing one of the campaign's completed modules is refused before any corpus document is read. | Test (TC-1571) |

## Dependencies

- **Upstream**: [FR-070](./FR-070-semantic-module-manifest-extension.md) declares the manifest surfaces this reads.
- **Downstream**: [FR-087](./FR-087-measure-structural-conformance-through-the-engine.md), [FR-088](./FR-088-measure-the-l3-semantic-dimension.md), [FR-090](./FR-090-publish-rates-with-unit-population-and-method.md)
