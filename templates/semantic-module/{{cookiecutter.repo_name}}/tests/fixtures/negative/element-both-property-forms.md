---
id: negative-002
title: "ShipmentInTwoForms"
type: element
object: element
expect: semantic.properties.both-forms-present
because: "one artifact carries one Properties form; the alternate is a separate file"
---
# [negative-002] ShipmentInTwoForms

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| shipment_id | UUID | 1..1 | identity |

```sysml
attribute shipment_id : UUID[1..1] { identity }
```
