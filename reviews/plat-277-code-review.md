---
id: SR-20261102086812
title: "PLAT-277 process PR134 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/spec-artifacts-process PR134 @9878e24 over origin/main 9fc6a8b; spec_artifacts_process/manifest.yaml, tests/test_semantic_manifest.py, tests/test_manifest.py"
review_set: subset
---

## Summary

Ticket: PLAT-277. Code review of contract fixes F2 (drop the inline `data_schema`
on `object_types.standard`) and F4 (rename five `archetypes:` display names).

Measured with quire 0.38.4 (engine 0.54.0), built from quire-cli main with
quire-rs main 6789f7d. All 40 `filament-standards/spec/standards/*.md` validate
with exit 0 under the full installed module set plus the PR module. The only
diagnostic printed is `DuplicateInverseEdge`, which another module owns. On
base the same run prints five `DuplicateArchetype` lines, the
`semantic.inline-data-schema` warning and
`semantic.record-invalid ... "code" is a required property` on every Standard
(exit 3). So F2's premise holds. A Standard with `code:` removed is still
refused, by the `Standard` frontmatter schema and by the `standard` locator. An
integer `code` is refused by the frontmatter schema. F2 loses no validation.

F4 removes the diagnostics, but the rename has two side effects in consumers
(FND-002, FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `semantic` block's header comment still says `object_types.standard` "keeps its inline `data_schema`" and that its "inline schema describes the frontmatter projection". The PR removed that schema, so the comment now contradicts the new comment at line 600. Rewrite or drop the stale sentences. | spec_artifacts_process/manifest.yaml:1740-1749; spec_artifacts_process/manifest.yaml:600-604 |
| FND-002 | medium | filament-core-service `ModuleService.activate` upserts archetypes by `(tenant_id, kind, name)` and never prunes this module's rows that are absent from the new manifest. On re-activation each renamed entry inserts a new row, and the old `ADR`/`Plan`/`Review`/`SpecReview`/`Standard` rows stay with the same `module_id`. The archetype list then shows both names. `ArchetypeService.get_by_kind` returns `rows[0]` with no `ORDER BY` when both rows share a tenant, so new containers bind to whichever row the database returns first. Existing containers keep their old ids. Fix this in filament-core-service by deleting this module's archetype rows that are not in the manifest on activation, or avoid the rename (see FND-003). Before re-activating, any live deployment needs a force deactivate. | spec_artifacts_process/manifest.yaml:7-46; filament-core-service filament_core_service/services/module_service.py:250-268; filament-core-service filament_core_service/services/archetype_service.py:60-76 |
| FND-003 | medium | Quire compiles every `archetypes:` entry into the validation registry (`Manifest::all_archetypes`). An entry with no schema becomes a permissive type. After the rename, `type: Project Plan`, `type: Spec Review`, `type: Architecture Decision Record`, `type: Review Report` and `type: Standard Definition` all validate with exit 0 and no checks. Measured: a `type: Project Plan` document passes, and `type: NoSuchType` is refused. The cause is in quire-rs: Filament nav entries should not compile as validation types. Either skip `archetypes:` entries that declare no validation parts, or fold a same-module same-name `archetypes:` entry into its `artifact_types` entry. That fix needs no rename, which also avoids FND-002. Renaming `artifact_types` instead is worse, because every corpus document's `type:` would change. | spec_artifacts_process/manifest.yaml:7-46; quire-rs src/loader/manifest.rs:430-445 |
| FND-004 | low | The module docstring lists the rows the file covers ("rows TC-089, TC-093, TC-094, TC-096, TC-128, TC-135") and already omits the new TC-167. The list is a hand-kept inventory of what the `trace` marks already state. Delete it rather than extending it. | tests/test_semantic_manifest.py:3 |

## Verdict

**CHANGES REQUESTED.** F2 is correct and loses no validation. The rename in F4
is a workaround for an engine defect. In quire it leaves five permissive type
names (FND-003), and in filament-core-service it leaves duplicate archetype rows
(FND-002). FND-001 is a one-line comment fix.

## Examined scope

```yaml
scope:
- {id: object_types.standard, path: spec_artifacts_process/manifest.yaml, role: examined}
- {id: archetypes, path: spec_artifacts_process/manifest.yaml, role: examined}
- {id: semantic, path: spec_artifacts_process/manifest.yaml, role: examined}
- {id: TC-096, path: tests/test_semantic_manifest.py, role: examined}
- {id: TC-167, path: tests/test_semantic_manifest.py, role: examined}
- {id: test_spec_review_archetype_registered_with_findings_validation, path: tests/test_manifest.py, role: examined}
```
