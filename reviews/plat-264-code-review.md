---
id: SR-17070
title: "Code review of quoin PR 782 (semantic-module template render tests and fixes)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@c73b0123; rust/crates/quoin-semantic/tests/tc_264_template_render.rs, rust/crates/quoin-semantic/Cargo.toml, templates/semantic-module/hooks/pre_gen_project.py, templates/semantic-module/conformance.yaml, templates/semantic-module/{{cookiecutter.repo_name}}/README.md, templates/semantic-module/{{cookiecutter.repo_name}}/docs/catalog-entry.md"
review_set: subset
---

# Code review of quoin PR 782

## Summary

Ticket: PLAT-264. Scoped to `git diff origin/main...HEAD` at c73b0123 (one commit on current
main). Includes the Rust lane (`rust-review`, with quoin's repo-level `rust-style` as the higher
authority). Tools: quoin 0.29.1, quire 0.36.2 (engine 0.50.2), cookiecutter 2.6.0.

Checks run: `cargo test -p quoin-semantic --test tc_264_template_render` (29 passed);
`cargo clippy -p quoin-semantic --all-targets --all-features -D warnings` (clean);
`cargo fmt --all --check` (clean). The private registry was not used: the embedded semantic
contract was populated locally from filament-core-data git at the commits that cut
`@agent-ix/semantic-schema` 0.3.0 (6156f9e0) and `@agent-ix/semantic-core` 0.3.2 (135a5f8a^).
With `cookiecutter` absent from PATH, 27 of 29 tests fail with the named install command and
none skip (the two that pass do not render). Hand renders: the default and three
`generated_targets` values, and `imported_modules=agent-ix/spec-artifacts-iso@01.2.3`.

## Verdict

**CHANGES REQUESTED (one high, three medium, two low).** The render harness is sound: it
really renders all three variants, refusals assert an empty output directory, and a missing
renderer fails rather than skips. The catalog-entry rewrite is correct against
`default-modules.yaml` (`git-subdir`, `url`, `path`, `defaultEnabled`) and the
`github:org/repo//pkg` source form `quoin-modules` parses. The exact-version fix matches the
published `module-semantic-block` schema (`imports` values are `common#/$defs/semver`,
described as "pinned to the exact version"), but it leaves the template's own rendered spec
and suite saying the opposite, and its regex admits a version the schema rejects. The README
fix records the declared targets correctly but points at a `spec/spec.md` list that does not
contain them. The ceremony deletion left a comment naming the deleted drift check.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The exact-version fix contradicts the template's own rendered spec and suite: rendered FR-001 still says imports map to "a version requirement" and FR-001-AC-8 says "The earlier clause that mandated an exact version is withdrawn: a version requirement is not a pin"; rendered `test_manifest_semantic.py` accepts `^1.2`, `>=1 <2`, `1.x`. The hook now refuses exactly what the rendered spec promises. quoin FR-075 and `package_manifest.rs` also say "any valid version requirement", so this needs a decision (exact everywhere, or a schema change upstream), not a one-sided hook edit | templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-001-declare-the-semantic-contract.md:30,42; templates/semantic-module/{{cookiecutter.repo_name}}/tests/test_manifest_semantic.py:62-73 |
| FND-002 | medium | `IMPORT_REF` uses `[0-9]+` per component, so `agent-ix/spec-artifacts-iso@01.2.3` passes the hook and renders `imports: {agent-ix/spec-artifacts-iso: 01.2.3}`, which the schema's semver pattern `(0\|[1-9][0-9]*)` rejects: the same hook-accepts/schema-rejects defect this PR fixes. It also rejects `1.2.3-rc.1`, which the schema admits | templates/semantic-module/hooks/pre_gen_project.py:61 |
| FND-003 | medium | The new README sentence says "`spec/spec.md` lists them as out of scope", but rendered `spec/spec.md` hard-codes `rust`, `typescript`, `python-pydantic-v2`, `python-dataclass`. For the default render (`json-schema,markdown`) the README names `markdown` and the cross-reference is false. It also calls `markdown` a "code-generation" emitter and reads "`markdown` have no emitter" | templates/semantic-module/{{cookiecutter.repo_name}}/README.md:41-47; templates/semantic-module/{{cookiecutter.repo_name}}/spec/spec.md:44 |
| FND-004 | medium | The conformance contract's header still says the file exists so "the drift check (FR-083-AC-5) has something to compare the maintained module repositories against". The drift check and FR-083-AC-5 do not exist, so the deletion of the ceremony is incomplete in the file this PR edits | templates/semantic-module/conformance.yaml:3-5 |
| FND-005 | low | `tc_264_044` pins the contract's exact top-level key set with the message "every block of the contract is read by the gate", but `contract_version` is read by nothing in the harness. It checks file shape, not behaviour, and its stated reason is false for one of the five keys | rust/crates/quoin-semantic/tests/tc_264_template_render.rs:836-856 |
| FND-006 | low | In `tc_264_033` the `uses` assertion carries the message "a job carries its own steps", which describes the opposite failure. A failure would report the wrong cause | rust/crates/quoin-semantic/tests/tc_264_template_render.rs:566-569 |

## Examined scope

```yaml
clean: false
scope:
- id: rust/crates/quoin-semantic/tests/tc_264_template_render.rs
  path: rust/crates/quoin-semantic/tests/tc_264_template_render.rs
  role: examined
  excerpt: "fn cookiecutter(output: &Path, args: &[&str]) -> std::process::Output { Command::new(\"cookiecutter\") ... .expect(\"`cookiecutter` is not installed; install it with `pipx install cookiecutter` ...\") }"
- id: rust/crates/quoin-semantic/Cargo.toml
  path: rust/crates/quoin-semantic/Cargo.toml
  role: examined
  excerpt: "regex = { workspace = true }"
- id: templates/semantic-module/hooks/pre_gen_project.py
  path: templates/semantic-module/hooks/pre_gen_project.py
  role: examined
  excerpt: "IMPORT_REF = re.compile(r\"^[a-z0-9][a-z0-9-]*/[a-z0-9][a-z0-9-]*@[0-9]+\\.[0-9]+\\.[0-9]+$\")"
- id: templates/semantic-module/conformance.yaml
  path: templates/semantic-module/conformance.yaml
  role: examined
  excerpt: "# FR-083-CON-2: a declared file rather than a list inside a test body, so a contract change is reviewable on its own and the drift check (FR-083-AC-5) has something to compare the maintained module repositories against."
- id: templates/semantic-module/README.md
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/README.md"
  role: examined
  excerpt: "**Only `json-schema` is emitted today.** ... Every other declared target is recorded as declared and not emitted: ... have no emitter today (the code-generation emitters belong to `agent-ix/filament-core-data#11`). `spec/spec.md` lists them as out of scope."
- id: templates/semantic-module/docs/catalog-entry.md
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/docs/catalog-entry.md"
  role: examined
  excerpt: "- name: {{ cookiecutter.repo_name }} defaultEnabled: true source: type: git-subdir url: {{ cookiecutter.org }}/{{ cookiecutter.repo_name }} path: {{ cookiecutter.package_name }} ... quoin module install github:{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}//{{ cookiecutter.package_name }}"
- id: templates/semantic-module/FR-001
  path: "templates/semantic-module/{{cookiecutter.repo_name}}/spec/functional/FR-001-declare-the-semantic-contract.md"
  role: context_only
  excerpt: "| FR-001-AC-8 | Every `semantic.imports` entry maps to a valid version requirement and carries no hash, digest, or revision. The earlier clause that mandated an exact version is withdrawn: a version requirement is not a pin. | Test |"
- id: default-modules.yaml
  path: default-modules.yaml
  role: context_only
  excerpt: "- name: spec-artifacts-iso defaultEnabled: true source: type: git-subdir url: agent-ix/spec-artifacts-iso path: spec_artifacts_iso"
```

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The doc comment on `DerivedImport::version_constraint` still says "the contract accepts any valid version requirement, and a plain version is one". That contradicts the exact-semver ruling, FR-075 as now edited, and the schema. No code change is needed, but the comment now misleads | rust/crates/quoin-semantic/src/package_manifest.rs:45-46 |
| FND-008 | low | The two new parametrised tests in the rendered suite (`test_a_range_or_malformed_version_is_not_exact`, `test_an_exact_version_is_accepted`) exercise only the test file's own `EXACT_SEMVER` constant. They can never observe the module's manifest, so they add rows tagged FR-001-AC-8 that test the oracle, not the module | templates/semantic-module/{{cookiecutter.repo_name}}/tests/test_manifest_semantic.py:80-89 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | dad84cf2: rendered FR-001 statement and FR-001-AC-8 now require an exact semantic version and refuse ranges; the rendered test checks the schema's semver pattern; FR-075 prose says exact; matches the lead's exact-everywhere ruling |
| FND-002 | fixed | dad84cf2: `IMPORT_REF` now embeds the schema's semver pattern (no leading zeros, prerelease/build allowed); `tc_264_007` refuses `@01.2.3` and renders `@1.2.3-rc.1` (35/35 pass) |
| FND-003 | fixed | dad84cf2: the `spec/spec.md` cross-reference and the "code-generation" wording are gone, and the verb agrees with the count. The default render reads "`markdown` has no emitter today" |
| FND-004 | fixed | dad84cf2: the conformance.yaml header no longer names the drift check, FR-083-AC-5 or the maintained repositories |
| FND-005 | fixed | dad84cf2: renamed to `..._with_exactly_these_blocks`; the message now says which blocks the gate reads and that `contract_version` versions the file |
| FND-006 | fixed | dad84cf2: message now reads "a job does not delegate to a shared reusable workflow" |
