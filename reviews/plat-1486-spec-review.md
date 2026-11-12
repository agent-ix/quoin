---
id: SR-20261112024907
title: "Spec review of quoin PR 830 (FR-137..139 locally installed licensed packs)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3157f4ba; spec/functional/FR-137-detect-and-place-licensed-packs.md, spec/functional/FR-138-never-publish-an-installed-pack.md, spec/functional/FR-139-pack-writes-nothing-into-a-repository.md, spec/functional/index.md, spec/spec.md"
review_set: subset
---

# Spec review of quoin PR 830

## Summary

Ticket: PLAT-1486. Scoped to `git diff origin/main...HEAD` at 3157f4ba. Tools: quoin 0.30.1,
quire 0.42.0 (engine 0.58.0). `quire validate` on the five changed files reports no errors and
three `ears:non-singular` warnings (see the EARS artifact). This base review covers integration
feasibility and index consistency; AC strength, EARS and failure domains have their own artifacts.

Feasibility: `quire_rs::clauses::check_publishable(&LoadedModule)` exists on quire-rs main
(src/clauses.rs:720, added in quire-rs#658, crate version 0.58.0) and `quire_rs::load_modules`
yields `LoadedModule`, so the decision can be obtained in-process without restating the rule.
But quoin's linked edge, `quoin-quire`, pins quire-rs `=0.56.0` at a rev that predates it, and
`quoin-modules` (install, list, ensure-defaults) has no engine dependency at all.

## Verdict

**PASS WITH FINDINGS (one high, two low).** The derived-pack design is sound and needs no second
copy of the rights rule, but the spec does not name the integration route, and the route does not
exist at the current pin.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The integration route is unnamed and cannot link today. `check_publishable` is not in quire-rs at quoin-quire's pin (`=0.56.0` @73454c7, rust/crates/quoin-quire/Cargo.toml:42), and `quoin-modules` has no dependency on `quoin-quire`, so "the module crate" guard (FR-138) cannot call it. Name the route: a `quoin-quire` function over `quire_rs::load_modules` + `clauses::check_publishable`, a new `publish_check` capability token with its witness in engine.rs, the quire-rs bump to a rev containing quire-rs#658 as a stated upstream dependency, and which crate hosts the guard | spec/functional/FR-137-detect-and-place-licensed-packs.md:17-18, spec/functional/FR-138-never-publish-an-installed-pack.md:36 |
| FND-002 | low | The FR-137..139 rows in spec.md sit between FR-124 and FR-130, out of id order. Move them after FR-135 | spec/spec.md:228-230 |
| FND-003 | low | Cross-repo relationship targets use `ix://agent-ix/quire-rs/spec/functional/FR-083` and `.../quire-cli/spec/functional/FR-029`; every other quire-rs reference in this spec (39) uses `ix://agent-ix/quire-rs/FR-NNN`. Use the short form | spec/functional/FR-137-detect-and-place-licensed-packs.md:8, spec/functional/FR-138-never-publish-an-installed-pack.md:8-10 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-137, path: spec/functional/FR-137-detect-and-place-licensed-packs.md, role: examined, excerpt: "Quoin SHALL derive pack status by calling the engine's check_publishable over the loaded module; a non-empty result makes the module a pack."}
- {id: FR-138, path: spec/functional/FR-138-never-publish-an-installed-pack.md, role: examined, excerpt: "Quoin SHALL expose one guard in the module crate that takes a set of installed module names and returns the engine's restricted sets per module"}
- {id: FR-139, path: spec/functional/FR-139-pack-writes-nothing-into-a-repository.md, role: examined, excerpt: "Installing, listing, inspecting, or removing a pack SHALL write only under IX_HOME"}
- {id: spec/functional/index.md, path: spec/functional/index.md, role: examined, excerpt: "three rows FR-137..139 appended after FR-136, in order"}
- {id: spec/spec.md, path: spec/spec.md, role: examined, excerpt: "three rows inserted after FR-124, before FR-130"}
```
