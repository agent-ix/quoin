---
id: SR-038
title: "Failure-domain review of issue 289 semantic-module architecture requirements"
type: SpecReview
analysis: failure-domain
scope: "US-013, FR-046..FR-050, NFR-013..NFR-014"
review_set: all
---

# Failure-domain review of issue 289 semantic-module architecture requirements

## Summary

The review exercised authority collision, stale external decisions, unknown dynamic modules,
compile-time closed worlds, projection loss, and accidental implementation activation. Each failure
shape now has an explicit disposition or stop condition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-029 | low | No unhandled failure domain remains; conflicts stop promotion, unknown extensions follow a profile, external decisions carry identity, and behavior-changing paths are excluded. | FR-048-AC-6; FR-049-AC-3; FR-050-AC-6; NFR-014 |

## Failure inventory

| Failure shape | Required response |
| --- | --- |
| Markdown, generated code, and a database each claim authority | Stop promotion and classify projection versus concern split or require a successor ADR. |
| A static consumer receives an unknown dynamic extension | Preserve, reject, or surface it according to a named profile; never coerce it to a known type. |
| An external ADR changes after this record was written | Its recorded repository/path/status/revision or date exposes the drift for review. |
| TypeSpec evidence is mistaken for an accepted source decision | Keep the JSON Schema recommendation and provisional ADR status distinct until a human successor change. |
| Architecture language leaks into implementation | The changed-path guard and explicit ticket scope reject the change. |
