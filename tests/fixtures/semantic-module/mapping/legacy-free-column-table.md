---
id: FR-900
title: WidgetRevision Entity
object: entity
type: FR
traces:
  - US-900
status: IMPLEMENTED
---

# FR-900: WidgetRevision Entity

## Description
The service SHALL define a `WidgetRevision` table with the following columns:

## Properties
| Column | Type | Constraints |
|--------|------|-------------|
| `id` | UUID | PK, default uuid4 |
| `widget_id` | UUID | FK -> widgets.id |
| `revision_number` | int | required |
| `payload` | Dict[str, Any] | JSONB column |
| `checksum` | str | content hash |
| `previous_id` | UUID (optional) | nullable, previous revision reference |
| `created_at` | datetime | default=utc_now |
