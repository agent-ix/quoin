---
id: SR-16402
title: scope-boundary review of FR-123 model-table mapping
type: SpecReview
analysis: scope-boundary
scope: agent-ix/quoin; spec/functional/FR-123-model-table-sections-mapping.md; spec/functional/FR-121-record-key-sections-mapping.md; spec/functional/FR-071-typed-properties-mapping.md; spec/matrix.md; spec/spec.md
review_set: subset
date: '2026-11-09'
---

# scope-boundary review of FR-123 model-table mapping

## Summary

Ticket: PLAT-240 (agent-ix/quoin PR #780, head 45b5004a).

Checked who owns each artifact FR-123 depends on: quire-rs FR-075/FR-076 and its extractor, quire-fixtures, and spec-objects-business. CON-2 (no change to the business module) holds; fixtures are referenced from quire-fixtures with no quoin copy; no vendoring, pins or ceremony. Three defects.

## Scope

- `FR-123-CON-1` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-CON-2` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#deps-upstream-quire-rs` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#deps-downstream-business` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#members` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123#vocabulary` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-123-AC-4` (examined) — spec/functional/FR-123-model-table-sections-mapping.md
- `FR-121#owned-keys` (examined) — spec/functional/FR-121-record-key-sections-mapping.md

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-123 needs `members[].target` as the SemanticId of the declaration the Member cell names, but quire-rs FR-075 resolves only Consumes/Emits items and the extractor carries the Member cell as a raw string (`src/semantic/model.rs` read_members: `target: target.to_string()`); the Upstream entry asks only for a change that 'copies model entries', which would copy `OrderLine` and fail the SemanticId pattern. | spec/functional/FR-123-model-table-sections-mapping.md:64 |
| FND-002 | low | AC-4 expects an empty `Description` to yield no `doc` and fail Term.json; quire-rs `read_vocabulary` carries it as `""`, which Term.json (`doc: string`) accepts, so the AC needs a quire-rs extractor change that the Dependencies do not name (FR-075's own text says an empty Description is absent, so it is a quire-rs code/spec mismatch). | spec/functional/FR-123-model-table-sections-mapping.md:65 |
| FND-003 | low | CON-1's 'holding no copy follows the FR-071-CON-2 rule' relies on PR #777's FR-071-CON-2 text; on origin/main FR-071-CON-2 still says Quoin publishes the goldens under tests/fixtures/semantic-module/, the opposite. | spec/functional/FR-123-model-table-sections-mapping.md:81 |

## Details

- **FND-001** (FR-123#members): The Member cell is a name (`OrderLine`, `Money`), while FR-104/FR-076 RelationDecl targets are id-only (FR-076-AC-17: a title Target is `target-not-id`), so the same record would carry two target-resolution rules. Fix: name the quire-rs FR-075 change (Member cell resolved by 'Declaration references' with admission from construct `references.members: [aggregate-member]`), or require an id in the Member cell as FR-104 does, and list it in Upstream, quire-rs.

- **FND-002** (FR-123#vocabulary): Fix: add the change to Upstream, quire-rs, or cite the quire-rs ticket.

- **FND-003** (FR-123-CON-1): Merge order: land #777 first or inline the rule. FR-121-CON-2 (merged) has the same dependency.

## Verdict

One high: the members target resolution is a quire-rs FR-075 change that is not named.
