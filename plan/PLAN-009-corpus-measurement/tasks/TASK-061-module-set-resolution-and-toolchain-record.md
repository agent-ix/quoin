---
id: TASK-061
title: "Module set resolution, contract-surface digests and the toolchain record"
type: Task
status: todo
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-085"
    type: references
  - target: "ix://agent-ix/quoin/TC-1506"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1507"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1508"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1509"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1510"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1511"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1568"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1569"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1570"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1571"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1556"
    type: verifies
---

# TASK-061: Module set resolution, contract-surface digests and the toolchain record

## Scope

Resolve the declared required module set from each module repository's Git
object store — never its working tree — and record what was resolved. The required
set is the nine modules this wave completed plus `engineering-assurance`; a
missing or unresolvable member exits non-zero before a corpus document is read.
Each module records its resolved commit, manifest version, declared artifact and
object types, and a digest per contract surface, and each declared `data_schema`
digest is compared with the schema file actually read.

## Subtasks

- [ ] Read module files with `git show <rev>:<path>` against the module repository, so a dirty checkout cannot change what is measured.
- [ ] Resolve every declared ref to a commit and record both, so a moved tag is visible.
- [ ] Digest the manifest, every declared JSON Schema, and the mappings declaration where one exists.
- [ ] Compare each declared `data_schema` digest with the digest of the file read; a difference is a module finding.
- [ ] Record `mappings: absent` for a module that publishes none, and still take its object types.
- [ ] Compare each measured commit with the commit its `default-modules.yaml` ref resolves to and record the divergence.
- [ ] Build the toolchain record: engine version and source revision, CLI version and source revision, each module's `semantic_core`.
- [ ] Record every module-load diagnostic as a module finding against the module, never as a document failure.

## Deliverables

- A module record and a toolchain record that make every published rate attributable to exact contract revisions.

## Notes

- Reading the object store is also what keeps this independent of agent-ix/quoin#347, under which no artifact-type module can be installed at all.
- `spec-artifacts-iso` declares ADR, Plan, Review, SpecReview and Standard as both an archetype and an artifact type in `spec-artifacts-process`; the engine reports five `DuplicateArchetype` diagnostics at load. Those are module findings this task must carry, not noise to filter.
