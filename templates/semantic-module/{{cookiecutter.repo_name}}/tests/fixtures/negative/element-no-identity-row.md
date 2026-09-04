---
id: negative-001
title: "ShipmentWithoutIdentity"
type: element
object: element
expect: semantic.record-invalid.missing-identity-field
because: "Element.json requires at least one field carrying the identity constraint"
---
# [negative-001] ShipmentWithoutIdentity

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| tracking_code | String | 1..1 | minLength: 6 |
| destination | String | 1..1 | |
