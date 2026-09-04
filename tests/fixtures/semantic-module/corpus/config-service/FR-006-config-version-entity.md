---
id: FR-006
title: ConfigVersion Entity
object: entity
type: FR
traces:
  - US-002
status: IMPLEMENTED
---

# FR-006: ConfigVersion Entity

## Description
The service SHALL define a `ConfigVersion` SQLModel table with the following columns:

## Properties
| Column | Type | Constraints |
|--------|------|-------------|
| `id` | UUID | PK, default uuid4 |
| `overlay_id` | UUID | FK → config_overlays.id |
| `version_number` | int | required |
| `data` | Dict[str, Any] | JSONB column |
| `hash` | str | content hash |
| `parent_id` | UUID (optional) | nullable, previous version reference |
| `created_at` | datetime | default=utc_now |
| `created_by` | str | actor identifier |
## Relationships

- `overlay`: many-to-one → ConfigOverlay ([FR-005](./FR-005-config-overlay-entity.md)), lazy="noload"

## Dependencies

- [FR-005](./FR-005-config-overlay-entity.md) (ConfigOverlay) — parent entity

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-006-AC-1 | The `ConfigVersion` model is persisted in the `config_versions` table | Inspection |
| FR-006-AC-2 | Versions are immutable once created — no update operations are permitted on existing versions | Test |
| FR-006-AC-3 | The `data` column uses JSONB for efficient querying and storage | Inspection |

### FR-006-AC-1
The `ConfigVersion` model SHALL be persisted in the `config_versions` table.

### FR-006-AC-2
Versions SHALL be immutable once created — no update operations are permitted on existing versions.

### FR-006-AC-3
The `data` column SHALL use JSONB for efficient querying and storage.
