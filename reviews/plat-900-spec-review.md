---
id: SR-20261103118171
title: "Spec review of quoin PR 801 (FR-070 drops schema-ownership delegation)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@fdfb889; spec/functional/FR-070-semantic-module-manifest-extension.md, spec/matrix.md, spec/reviews/module-cookiecutter-dependency.md, plan/PLAN-007-semantic-module-contract"
review_set: subset
---

# Spec review of quoin PR 801

## Summary

Ticket: PLAT-900. Scoped to `git diff origin/main...HEAD` at fdfb889. The PR deletes
FR-070's two schema-delegation bullets and FR-070-CON-2 with its CR note, rewrites the
stale `import_types` conditional bullet, adds FR-070-AC-14 and TC-2140, and drops the
withdrawn TC-1343 row. Tools: quoin 0.30.0, quire 0.40.0.

Checks run: `quire validate` on FR-070 passes; on the changed set only the pre-existing
TC-1943/TC-1944 tombstone rows in spec/matrix.md fail (untouched by this PR). The pinned
`@agent-ix/semantic-schema` 0.5.0 was installed from the lockfile and read: its
`module-manifest.schema.json` root has `additionalProperties: false` and declares all
eight registry keys; its inline `semantic` block admits exactly the ten FR-070 keys,
including `import_types`. So the old conditional is moot and can simply go. quoin
compiles only `/properties/semantic` from that schema (`SemanticValidators::load`,
rust/crates/quoin-semantic/src/manifest.rs:193-211); it never validates the manifest root.

## Verdict

**PASS WITH FINDINGS (two medium).** The delegation bullets and CON-2 are gone, and no
FR-070 text names filament-core-service any more. Two problems: the rewritten bullet brings
the deleted "take the schema from the published package" rule back in new words, and
FR-070-AC-14 has no Behavior statement to verify. The review-file edit is reported in the
code-review artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new bullet "Quoin SHALL admit exactly the `semantic` keys the pinned `@agent-ix/semantic-schema` release defines and refuse any other" replaces the deleted delegation bullets, which the ticket says must go with no replacement wording. It also sets a second authority for the admitted key set next to the explicit ten-key "SHALL admit exactly these keys" bullet. If the two ever differ, the spec contradicts itself. The unknown-key rule is already stated by the explicit list and the "unknown key inside `semantic`" bullet. 0.5.0 defines `import_types`, so the old conditional can be deleted outright. Delete the bullet | spec/functional/FR-070-semantic-module-manifest-extension.md:48 |
| FND-002 | medium | FR-070-AC-14 (the eight root registry keys validate against the pinned schema, and an unknown root key is refused) has no parent Behavior statement. FR-070's subject is the `semantic` block, and quoin never applies the root schema, so the AC states a property of the upstream file with no quoin actor. Add a Behavior bullet that declares the eight root keys as admitted manifest keys, with no schema-sourcing wording, or drop AC-14 and TC-2140 and rely on the upstream declaration | spec/functional/FR-070-semantic-module-manifest-extension.md:82, spec/matrix.md:1369 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-070, path: spec/functional/FR-070-semantic-module-manifest-extension.md, role: examined, excerpt: "Behavior bullets; delegation bullets and CON-2 deleted"}
- {id: FR-070-AC-3, path: spec/functional/FR-070-semantic-module-manifest-extension.md, role: examined, excerpt: "A semantic block with an unknown key foo, or with the retired key semantic_core, is rejected naming that key; every key in the admitted list is accepted."}
- {id: FR-070-AC-14, path: spec/functional/FR-070-semantic-module-manifest-extension.md, role: examined, excerpt: "A module manifest carrying all eight registry keys ... validates against the pinned @agent-ix/semantic-schema module-manifest schema, and the same manifest with an unknown root key is refused."}
- {id: FR-070-CON-1, path: spec/functional/FR-070-semantic-module-manifest-extension.md, role: examined, excerpt: "The semantic block (including import_types) SHALL add no required key to the manifest root, ObjectTypeEntry, ArchetypeEntry, or ArtifactTypeEntry."}
- {id: TC-2140, path: spec/matrix.md, role: examined, excerpt: "A manifest carrying all eight registry keys ... validates against the pinned module-manifest schema; an unknown root key is refused"}
- {id: PLAN-007, path: plan/PLAN-007-semantic-module-contract, role: examined, excerpt: "TASK-039 scope reworded to consume published packages; filament-core-service#21 references removed"}
```
