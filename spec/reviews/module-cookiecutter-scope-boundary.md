---
id: SR-134
title: "Scope and boundary analysis of the semantic-module cookiecutter"
type: SpecReview
analysis: scope-boundary
scope: "StR-008; US-021; FR-076..FR-083; NFR-018; NFR-019"
review_set: all
---

# SR-134: Scope and boundary analysis of the semantic-module cookiecutter

## Summary

The reviewed slice (StR-008, US-021, FR-076 through FR-083, NFR-018, NFR-019)
specifies a single cookiecutter template, hosted in Quoin, that renders a
Quire semantic-module repository already conforming to the semantic-module
contract. The neighbouring systems are named correctly in most places: Quire
is invoked by name for `extract_semantic` and `quire validate`
(`agent-ix/quire-rs#392`, `agent-ix/quire-rs#388`), the `filament-core-data`
target registry is the named source for `generated_targets`, and the
template is explicitly barred from copying the schema emitter, the Quire
runtime, or the semantic-core grammar. The explicit safety gate against
bulk-recreating or normalizing existing module repositories is honored in
the requirement text itself.

Two boundary issues stand out. First, the emit-command pipeline that
absolutizes `$id`/`$ref` values, discards imported-library schemas, computes
digests, and rewrites the manifest textually is described as identical
logic that two hand migrations converged on separately, yet FR-077 renders
it as per-repository generated output rather than a versioned dependency —
the same treatment FR-076-CON-1 correctly gives the emitter, runtime, and
grammar. Second, FR-083's fleet-wide drift check reads the actual surfaces
of the maintained semantic-module repositories from inside Quoin's own gate
without specifying how, at what revision, or under what trust boundary,
which pulls a fleet-auditing responsibility into a gate otherwise scoped to
verifying the template's own render. A related, narrower gap concerns the
vendored module-manifest schema this slice depends on for FR-078/FR-083
conformance checks, whose synchronization with its owner, filament-core-service,
is unresolved work recorded in an earlier review and not revisited here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-077 describes the emit-command pipeline, absolutize refs, discard imported schemas, compute digests, rewrite the manifest, as the same logic both hand migrations converged on separately, yet the template renders it as per-repository output rather than reaching it as a versioned dependency the way FR-076-CON-1 requires for the emitter, runtime, and grammar, reproducing the fleet maintenance drift StR-008 exists to end. | FR-076; FR-077; StR-008 |
| FND-002 | medium | FR-083 requires Quoin's own render-and-test gate to compare the conformance contract against the surfaces the maintained semantic-module repositories carry, but names no access mechanism, pinned revision, or hermeticity boundary for reading those external repositories, coupling a template-only gate to live fleet state. | FR-083; StR-008-VC-3 |
| FND-003 | medium | FR-078-AC-1 and FR-083 assert conformance against Quoin's vendored module-manifest schema, whose ownership sits with filament-core-service under FR-070, out of this slice; an earlier review already recorded three divergent copies of that schema today, so this slice inherits an unresolved synchronization question rather than establishing the vendored copy it checks against is authoritative. | FR-078; FR-083; FR-070 |
| FND-004 | low | FR-082 requires the rendered spec tree to carry functional requirements describing the rendered module's own contract, but StR-008 and US-021 state the template does not generate a module's types, and neither requirement says what those rendered functional requirements contain or who authors them. | FR-082; StR-008; US-021 |
| FND-005 | low | The boundary against bulk-recreating or normalizing existing module repositories is honored explicitly: US-021 Constraints state no existing repository is recreated or normalized, StR-008 places each module's schema-completion work out of scope, and FR-083's comparison against maintained repositories is read-only, failing Quoin's own gate rather than changing those repositories. | US-021; StR-008; FR-083 |
