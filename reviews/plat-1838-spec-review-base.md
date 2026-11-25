---
id: SR-967175026302197768900474181941935084761
title: "Base checklist review of Quoin PLAT-1838 dossier requirements"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-191-dossier-command.md, FR-229 through FR-232, spec/usecase/US-024-hand-a-reviewer-dossier-documents-generated-from-records.md, spec/spec.md, spec/functional/index.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-191"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-229"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-230"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-231"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-232"
    type: "reviews"
---

## Summary

The review examined the changed dossier requirements, user-story example, and indexes. The four FRs preserve the human decision boundary and the PLAT-1816 producer/source identity boundary, but the new source-record inputs lack typed contracts, and the software-description missing-field behavior names no permitted gap kind.

## Verdict

Spec validation passed for all eight changed documents. IDs and links are valid; verification cells name methods only. The source-record shapes and gap mapping need clarification before implementation. The new criteria currently have no trace-tagged tests in the matrix because implementation is pending; close that coverage in the code phase.

Active profiles AP-201 and AP-202 were inspected. AP-201 is scoped to finding-quality measurement changes and AP-202 to live evaluation of Jev lenses; neither applies to dossier generation. Both say `review_policy.mode: recommend`, and no applicable profile requires an enforced review run.

## Scope

- `FR-191`: changed M3 document-kind, selector, order, source-identity, gap-kind, gap-subject, and AC-32 statements.
- `FR-229` through `FR-232`: each Description, Inputs, Outputs, Behavior, and Acceptance Criteria section.
- `US-024-EX-5` and its Downstream dependencies.
- `StR-010` was read as traceability context.
- Changed index entries in `spec/spec.md` and `spec/functional/index.md`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The new software, documentation-level-decision, and device source records have no installed object types or spec-defined schemas. | FR-229, FR-230, FR-232 | missing-requirement |
| FND-002 | medium | Missing optional software-description fields require a gap, but the closed gap vocabulary defines no gap for those fields. | FR-229-AC-2, FR-191 | wrong-requirement |

## Evidence

- `quoin catalog list --json` has no `software_description`, `documentation_level_decision`, or `device_description` object types; `quoin catalog show` reports each type not found.
- FR-229 through FR-232 name fields and behavior but do not reference a source type schema defining field types, requiredness, validation, or identity uniqueness. FR-191 treats invalid selected records as source failures, so the validity boundary is also undefined for these types.
- FR-229 Inputs make `purpose`, `functions`, `operating_environment`, and `interfaces` optional, and Outputs require missing values to become gaps. FR-229-AC-2 requires a corresponding gap. FR-191's closed list has `no_software_description_source` but no software-description missing-field gap.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-233 now defines the three source-record kinds, member schemas, requiredness, validation, and identity uniqueness. After excerpt: `These JSON records SHALL use the closed record contract below; their record_type values are dossier input-record kinds, not Quire catalog object types.` |
| FND-002 | fixed | FR-191 now includes `software_description_field_unstated`, and FR-229 requires that gap for each missing/empty optional field. After excerpt: `software_description_field_unstated` |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | FR-233 requires absent or empty optional arrays to be reported as unstated, but the closed gap contract does not define a gap mapping for `architecture_part_ids` or `architecture_decision_ids`. | FR-233, FR-231, FR-191 |

Evidence: FR-233 Inputs and the following shared paragraph require missing/empty optional arrays to be reported as unstated. The architecture ID arrays are optional on `software_description`; FR-231 does not define their missing/empty gap behavior, and FR-191's closed gap kinds define no architecture-field-specific gap or subject. The `no_software_design_source` gap covers absence of a selected design source at document level, but does not say how a missing list on one item is identified when other software items have design sources.

Round-1 recheck included FR-233, updated FR-191, FR-229–FR-232, StR-010, US-024, and changed indexes. All ten changed spec documents validated; no hash/digest/pin/provenance antipattern was added. The new criteria remain untagged in the matrix pending implementation.
