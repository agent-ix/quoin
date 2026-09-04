---
id: element-001
title: "Shipment"
type: element
object: element
---
<!-- element authoring skeleton ({{ cookiecutter.repo_name }}). Fill every
     section with substantive content. Contract (the manifest's body_extraction
     asserts it, and `make test` proves it):
     - Frontmatter MUST carry id, title, type: element, object: element.
     - "## Properties" (H2, required): one typed row per attribute, header
       exactly `Field | Type | Multiplicity | Constraints`. At least one row
       carries the `identity` constraint, because Element.json requires one.
     - "## Invariants" (H2): one `### <clauseId>` per clause, each owning
       exactly one ```ocl``` fence. The fence text is carried verbatim and is
       never evaluated here.
     REPLACE the vocabulary. `Shipment` is a worked example of the SHAPE; your
     module's types, fields and clauses go here. -->
# [element-001] Shipment

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| shipment_id | UUID | 1..1 | identity |
| tracking_code | String | 1..1 | minLength: 6, maxLength: 40 |
| destination | String | 1..1 | minLength: 1 |
| weight_grams | Integer | 1..1 | min: 1 |
| dispatched_at | Timestamp | 0..1 | |

## Invariants

The clauses a Shipment declaration enforces. Each clause owns one `ocl` fence
under its own `### <clauseId>` heading.

### DispatchedShipmentHasTrackingCode

```ocl
context Shipment
inv DispatchedShipmentHasTrackingCode:
  self.dispatched_at->notEmpty() implies self.tracking_code->notEmpty()
```

### WeightIsPositive

```ocl
context Shipment
inv WeightIsPositive:
  self.weight_grams > 0
```
