---
id: FR-085
title: "Resolve the completed module set and its contract surfaces"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "traces_to"
---

# FR-085: Resolve the completed module set and its contract surfaces

## Description

The corpus measurement SHALL resolve every module named in its declared module set at the exact
revision declared for it, and SHALL record each module's manifest, declared types, JSON Schemas and
Markdown mappings by digest, so that the schemas a published rate was measured against are
identifiable afterwards.

## Rationale

The catalog pins in `default-modules.yaml` are the previous release of each module, not the completed
contract of this wave. Measuring the catalog pin would measure the wrong schemas and report a number
about a contract nobody is proposing to promote. Naming the revision per module is what keeps the
report and the promotion decision talking about the same thing.

## Inputs

- A declared module set: for each module, a repository path, a revision, and the module's data
  subdirectory.

## Outputs

- A module record per module: name, repository, requested revision, resolved commit, manifest
  version, declared artifact types, declared object types, and a digest per contract surface.
- A refusal record for any declared module the measurement could not resolve.

## Behavior

- The measurement SHALL read each module's files at the declared revision from the repository's
  object store, and SHALL NOT read the repository's working tree, so that a dirty checkout cannot
  change what was measured.
- If a declared revision does not resolve in the module repository, then the measurement SHALL record
  the module as `unresolved` with the failing revision and SHALL continue with the remaining modules.
- The measurement SHALL record, for each module, the digest of its manifest, of each declared JSON
  Schema, and of its mappings declaration when it publishes one.
- Where a module publishes no Markdown mappings, the measurement SHALL record `mappings: absent`
  against it and SHALL NOT treat its documents as unmeasurable for that reason alone.
- The measurement SHALL record, for each module, whether the revision measured equals the revision
  pinned for that module in `default-modules.yaml`.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-085-CON-1 | Module resolution SHALL NOT check out, fetch, or otherwise mutate a module repository. | Safety | Test |
| FR-085-CON-2 | An unresolved module SHALL NOT be silently treated as a module declaring no types. | Interface | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-085-AC-1 | Each declared module resolves to a commit and records its manifest version, declared artifact types and declared object types. | Test (TC-1506) |
| FR-085-AC-2 | Each contract surface of each module carries a SHA-256 digest in the module record. | Test (TC-1507) |
| FR-085-AC-3 | Content read for a module is byte-identical to that module's content at the declared revision when the repository working tree carries an unrelated uncommitted edit. | Test (TC-1508) |
| FR-085-AC-4 | An unresolvable revision yields one `unresolved` module record naming the revision, and the remaining modules are still resolved. | Test (TC-1509) |
| FR-085-AC-5 | A module publishing no mappings declaration records `mappings: absent` and still contributes its declared object types. | Test (TC-1510) |
| FR-085-AC-6 | Each module record states whether the measured revision equals its `default-modules.yaml` pin, and the report names every module where it does not. | Test (TC-1511) |

## Dependencies

- **Downstream**: [FR-087](./FR-087-evaluate-declared-markdown-mappings.md) evaluates the resolved mappings.
