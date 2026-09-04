---
id: legacy-001
title: "ShipmentInTheLegacyForm"
type: element
object: element
expect: semantic.legacy-form
because: "the pre-contract free-text Properties form is accepted at warning until the sweep report exists and a human promotes the module"
---
# [legacy-001] ShipmentInTheLegacyForm

## Properties

- **shipment_id**: UUID (required) — the shipment's identity
- **tracking_code**: String (required)
- **destination**: String (required)
- **weight_grams**: Integer (required)
- **dispatched_at**: Timestamp (optional)
