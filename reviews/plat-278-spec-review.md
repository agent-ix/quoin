---
id: SR-17047
title: "Spec review of the FR-071 verification note in quoin PR 781"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin@f17b3c2d; spec/functional/FR-071-typed-properties-mapping.md"
review_set: subset
---

# Spec review of the FR-071 verification note

## Summary

Ticket: PLAT-278. The one spec change is a seven-line verification note under FR-071's
acceptance criteria. Tools: quoin 0.29.1.

The claimed mismatch was measured directly:

- `quire-fixtures` at ce172a7d, and on quire-rs main, records `TypeRef.target` as
  `{kind, value}` (for example `{kind: KernelScalar, value: UUID}`). It records constraints as
  `{kind: MinConstraint, value: {keyword: min, value: {kind: number, value: 1}}}`.
- semantic-core at the `semantic-core-v0.3.2` tag (quoin's `package.json` pin) declares
  `TypeRef.target` as `anyOf` [SemanticId string, KernelScalar string]. `MinConstraint.value` is
  a bare number or string. The two shapes therefore do not match under 0.3.2. The note is right
  on that point.
- filament-core-data main is different. Commit 7e6f209e ("IR-994: Migrate semantic kernel unions
  to adjacent wire forms (#310)", IR-994 Done) changed `TypeRef.target` to `$ref
  TypeRefTarget.json` and made `ConstraintDecl.json` a `oneOf` over `{kind, value}`. That is
  exactly the fixtures' shape. IR-994's acceptance says "old untagged values are refused". No
  semantic-core tag contains 7e6f209e yet.

## Verdict

**FAIL (one high, one medium, one low).** The note puts the AC-1 blocker in the wrong repository,
and it has no owner or expiry condition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The note says the goldens "belong to `quire-rs`; the mismatch is reported there". That makes regenerating the fixtures the fix. In fact the fixtures already use semantic-core's current adjacent `{kind, value}` wire form (filament-core-data IR-994 / #310 / 7e6f209e). quoin's pin is what lags: 0.3.2 in `package.json`, while the note says "0.3.0". The real blocker is a semantic-core release that contains IR-994, followed by a quoin pin bump. Regenerating the fixtures as the note directs would move them to the untagged form that IR-994 refuses | spec/functional/FR-071-typed-properties-mapping.md:70-75 |
| FND-002 | medium | The note names no owner ticket and no expiry condition. "Reported there" is not true: no quire-rs or filament-core-data ticket exists for this, and the PLAT-278 coder comment says ticket creation was denied. The note should cite a ticket and state its exit. For example: "When the `@agent-ix/semantic-core` pin includes IR-994, add `FieldDecl.json` validation for AC-1, add `TypeRef`/`ConstraintDecl` validation to the AC-4/AC-6 tests, and delete this note." | spec/functional/FR-071-typed-properties-mapping.md:70-75 |
| FND-003 | low | AC-1 says "Quoin's copy of the config-service FR-006 rows". FR-071-CON-2 says quoin holds no copy of the fixtures. The note sits directly under AC-1 and does not reconcile the two | spec/functional/FR-071-typed-properties-mapping.md:61 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-071-AC-1, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "Quoin's copy of the config-service FR-006 rows, rewritten as a typed table, has an expected `FieldDecl[]` fixture whose every element validates against the semantic-core `FieldDecl.json`."}
- {id: FR-071-CON-2, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "Quoin SHALL read the golden fixtures (table form, fence form, expected normalized `FieldDecl[]`, expected diagnostics) from the `quire-fixtures` crate in `agent-ix/quire-rs`, holding no copy of them."}
- {id: FR-071-verification-note, path: spec/functional/FR-071-typed-properties-mapping.md, role: examined, excerpt: "FR-071-AC-1's schema validation is not yet assertable: the goldens record `TypeRef.target` and constraints in a tagged `{kind, value}` shape that the semantic-core 0.3.0 schemas Quoin reads do not accept. The goldens belong to `quire-rs`; the mismatch is reported there rather than copied or weakened here."}
- {id: FR-071, path: spec/functional/FR-071-typed-properties-mapping.md, role: context_only, excerpt: "Quoin SHALL publish the mapping from a `## Properties` section ... to a semantic-core `FieldDecl[]`, as golden fixtures that Quire implements"}
```
