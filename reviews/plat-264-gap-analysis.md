---
id: SR-17071
title: "Gap analysis of quoin PR 782 (semantic-module template render tests)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@c73b0123; spec/functional/FR-076, FR-078, FR-081, FR-082, FR-083 against rust/crates/quoin-semantic/tests/tc_264_template_render.rs and templates/semantic-module"
review_set: subset
---

# Gap analysis of quoin PR 782

## Summary

Ticket: PLAT-264. Plan completion: not assessed. Planless audit of the criteria of the five
requirements the PR claims to verify (FR-076, FR-078, FR-081, FR-082, FR-083) against every
`/// Trace:` tag under `rust/`, plus the template the tests render. Tools: quoin 0.29.1,
quire 0.36.2 (engine 0.50.2), cookiecutter 2.6.0 and the system cookiecutter 1.7.3
(`/usr/bin/cookiecutter` on this host).

Covered by this PR and checked against the criterion text: FR-076-AC-1..13, FR-076-CON-2,
FR-078-AC-1/2/3/5/6/8, FR-081-AC-1/2/5/6/7, FR-082-AC-2/5, FR-083-AC-1..4, FR-083-CON-1/2.
Declared not covered in the PR body, accepted as stated (need a registry or toolchain):
FR-077, FR-080, FR-081-AC-3, FR-082-AC-1 and the `quire validate` behaviour of FR-083.

## Verdict

**GAPS FOUND (two medium, one low).** The tests that exist are real: each renders, and each
fails without the renderer. But several criteria a render alone can check are neither tested
nor named in the PR's "Not covered" list, two existing tests assert criteria they are not
tagged with, and FR-076-AC-6 passes only because cookiecutter 2.x refuses an off-list choice
itself. The template's own check is never reached.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Criteria a render alone can check stay unbacked and are missing from the PR's "Not covered" list: FR-078-AC-7 (no type name in two declarations), FR-081-AC-8 and FR-081-CON-1 (no local path reference or version upper bound), FR-082-AC-3 (no TC id in a rendered Verification cell), FR-082-AC-4 (every rendered test carries a trace tag naming an existing AC), FR-082-AC-6 and FR-082-CON-2 (Out of Scope wording, no copied requirement), FR-076-CON-1 (no emitter, runtime or grammar copy) | spec/functional/FR-078-generated-manifest-semantic-block.md:72; rust/crates/quoin-semantic/tests/tc_264_template_render.rs |
| FND-002 | low | `tc_264_020` asserts that the `semantic` block has exactly the nine admitted keys, which is FR-078-CON-1, and `tc_264_040` asserts that no rendered file carries a credential or private-registry host, which is FR-081-AC-4. Neither is tagged, so the computed matrix still reports both as untested | rust/crates/quoin-semantic/tests/tc_264_template_render.rs:382-385,741-743 |
| FND-003 | medium | FR-076-AC-6 is met by the renderer, not the template. Under cookiecutter 2.6.0, `module_kind=hybrid` is refused by cookiecutter's own choice validation before the hook runs. Under cookiecutter 1.7.3, the off-list value falls back to the default and an object module renders with exit 0 (reproduced; `tc_264_006` and `tc_264_014` fail). The hook's `MODULE_KIND not in MODULE_KINDS` branch is unreachable under 2.x, and no renderer floor is stated. Pre-existing, but the PR claims AC-6 as verified | templates/semantic-module/hooks/pre_gen_project.py:74-78; rust/crates/quoin-semantic/tests/tc_264_template_render.rs:253-257 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-076-AC-6, path: spec/functional/FR-076-semantic-module-template-variants.md, role: examined, excerpt: "`module_kind: hybrid` aborts rendering naming `hybrid`."}
- {id: FR-076-AC-7, path: spec/functional/FR-076-semantic-module-template-variants.md, role: examined, excerpt: "`imported_modules: [\"agent-ix/spec-objects-business\"]` and `[\"agent-ix/spec-objects-business@^1.2.0\"]` abort naming the entry; the exact `<org>/<repo>@<x.y.z>` form renders, because the module-manifest schema admits exact versions only."}
- {id: FR-076-AC-10, path: spec/functional/FR-076-semantic-module-template-variants.md, role: examined, excerpt: "A rendering that declares a target with no emitter today records that target as declared-not-emitted in the rendered README and in `semantic.targets`, and the two agree."}
- {id: FR-076-CON-1, path: spec/functional/FR-076-semantic-module-template-variants.md, role: examined, excerpt: "The template SHALL contain no copy of the schema emitter, the Quire runtime, or the semantic-core grammar; each is reached as a versioned dependency."}
- {id: FR-078-AC-5, path: spec/functional/FR-078-generated-manifest-semantic-block.md, role: examined, excerpt: "A mixed-variant rendering with two imported modules yields two `semantic.imports` entries carrying the declared exact versions; an object-variant rendering with none yields `imports: {}`."}
- {id: FR-078-AC-7, path: spec/functional/FR-078-generated-manifest-semantic-block.md, role: examined, excerpt: "No exported type name appears in more than one declaration of a rendered manifest, in any variant."}
- {id: FR-078-CON-1, path: spec/functional/FR-078-generated-manifest-semantic-block.md, role: examined, excerpt: "The rendered `semantic` block SHALL carry no key outside the set FR-070 admits."}
- {id: FR-081-AC-4, path: spec/functional/FR-081-generated-public-repository-baseline.md, role: examined, excerpt: "No rendered file matches a credential, token, or private-registry publication pattern."}
- {id: FR-081-AC-7, path: spec/functional/FR-081-generated-public-repository-baseline.md, role: examined, excerpt: "The rendered catalog document names the Quoin catalog file and the tracking project to add the module to."}
- {id: FR-081-AC-8, path: spec/functional/FR-081-generated-public-repository-baseline.md, role: examined, excerpt: "No rendered dependency declaration uses a local path reference or a version upper bound."}
- {id: FR-082-AC-3, path: spec/functional/FR-082-generated-governance-tree.md, role: examined, excerpt: "No rendered acceptance criterion's `Verification` cell carries a test-case id."}
- {id: FR-082-AC-4, path: spec/functional/FR-082-generated-governance-tree.md, role: examined, excerpt: "Every rendered test function carries a trace tag, and every id it names is an acceptance criterion that exists in the rendered spec."}
- {id: FR-082-AC-6, path: spec/functional/FR-082-generated-governance-tree.md, role: examined, excerpt: "The rendered master-requirements Out of Scope section states that the module's domain types are the maintainer's to specify, and no rendered requirement text is copied from a maintained module repository."}
- {id: FR-083-AC-1, path: spec/functional/FR-083-template-render-self-tests.md, role: examined, excerpt: "The gate renders all three variants and reports which variant and which surface failed when one does."}
- {id: FR-083-AC-2, path: spec/functional/FR-083-template-render-self-tests.md, role: examined, excerpt: "Every rendered directory is removed after the run, including after a failure."}
- {id: FR-083-AC-3, path: spec/functional/FR-083-template-render-self-tests.md, role: examined, excerpt: "An injected unresolved token, placeholder organization, absolute path, credential, or private-registry default in the template fails the gate naming the file."}
- {id: FR-083-AC-4, path: spec/functional/FR-083-template-render-self-tests.md, role: examined, excerpt: "Removing a surface the conformance contract names fails the gate naming that surface."}
- {id: FR-083-AC-6, path: spec/functional/FR-083-template-render-self-tests.md, role: context_only, excerpt: "With the schema toolchain absent, the gate fails naming the install command and reports no skipped emission check."}
- {id: NFR-020, path: spec/non-functional/NFR-020-declared-toolchain-floors.md, role: examined, excerpt: "When an external command the template or a rendered repository invokes is absent, the invoking command SHALL fail naming the command and how to install it."}
```

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | `tc_264_051` treats only a literal `<` as a version upper bound. Poetry caret requirements in the rendered dev dependencies (`^26.1.0`, which means `<27`) pass, so whether FR-081-AC-8 forbids an implicit caret cap is decided by the test's spelling rather than by the criterion | rust/crates/quoin-semantic/tests/tc_264_template_render.rs:908-937 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | dad84cf2 adds tc_264_050..055, tagged FR-078-AC-7, FR-081-AC-8/CON-1, FR-082-AC-3, FR-082-AC-4, FR-082-AC-6/CON-2 and FR-076-CON-1; each renders and passes (35/35) |
| FND-002 | fixed | dad84cf2 adds `/// Trace: FR-078-CON-1` to tc_264_020 and `/// Trace: FR-081-AC-4` to tc_264_040 |
| FND-003 | fixed | dad84cf2: FR-076 states the cookiecutter 2.x floor, and `assert_renderer_floor` fails every render below it. Re-run with `/usr/bin/cookiecutter` 1.7.3: tc_264_006 fails with "cookiecutter 1.x is below the 2.x floor (FR-076)" |
| FND-004 | fixed | d0cac2c1 settles the ambiguity in the criterion rather than the test: FR-081-AC-8 and FR-081-CON-1 now define an upper bound as an explicit `<` or `<=` and allow caret or tilde ranges as compatible-release floors. That matches the ecosystem's no-pins stance, and `tc_264_051` now checks exactly what the criterion says |
