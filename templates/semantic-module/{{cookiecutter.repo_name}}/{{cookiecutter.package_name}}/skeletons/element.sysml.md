---
id: element-001
title: "Shipment"
type: element
object: element
---
<!-- element authoring skeleton, alternate Properties form. Declares exactly the
     same fields as element.md, authored as one ```sysml``` fence instead of the
     typed table. Both forms extract to the same declarations.
     One artifact carries ONE form; the alternate is a separate file, never a
     second block in the same document. A document carrying both is refused —
     see tests/fixtures/negative/element-both-property-forms.md. -->
# [element-001] Shipment

## Properties

```sysml
attribute shipment_id : UUID[1..1] { identity }
attribute tracking_code : String[1..1] { minLength: 6, maxLength: 40 }
attribute destination : String[1..1] { minLength: 1 }
attribute weight_grams : Integer[1..1] { min: 1 }
attribute dispatched_at : Timestamp[0..1]
```

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
