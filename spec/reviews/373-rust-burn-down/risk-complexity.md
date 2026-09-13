---
id: SR-156
title: "Risk and complexity review of the EPIC 373 Rust burn-down spec set"
type: SpecReview
analysis: risk-complexity
scope: "ADR-0003, StR-009, US-024, FR-096..FR-103, NFR-024..NFR-027, IT-003"
review_set: all
---

# Risk and complexity review of the EPIC 373 Rust burn-down spec set

## Summary

This set specifies a staged Rust port of a quoted 105,814-line first-party
surface across ten delivery stages on a ~46-week critical path, and it is
unusually well grounded: the boundary is the one this repository already
hardened in `src/quire/exec.ts`, the identity guarantees are stated as hard
gates rather than regressions, and coexistence is given its own metric class.
The risk is not that the set is vague — it is that four of its load-bearing
numbers and two of its identity claims do not survive measurement against the
tree at `30425e6`. The largest named block of debt, `skills/` at 23,164 lines,
is 22,575 lines of one triplicated vendored build artefact belonging to
`ix-spec-workflows`; the validator the port must match asserts no `format`
keyword at all because `ajv-formats` is not a dependency, while ten schema sites
declare one; the evidence store's record identity is sha256 over `node:crypto`,
a digest domain the "exactly one crate" clause never names, alongside a
hand-written strict JSON parser whose refusals are part of that identity; and
several gates are written over populations ("every reachable store in the
ecosystem") that nothing enumerates, which under the set's own
empty-population rule makes them unable to pass. Volatility is concentrated in
the three places the owner already knows about — the `filament-core-data`
publication gate, the oclif extension contract, and the `qa-corpus` scope
ruling — plus one the set does not flag: the package is published to public
npmjs today, which the set forbids. Nothing here blocks specification; five
items should be re-based or spiked before `spec-to-plan` decomposes stages.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-173 | high | The programme's largest named debt block is not Quoin logic: of `skills/` 23,164 lines, 22,575 are three byte-identical copies (md5 `ea3d2c74…`) of `workflow-assets/dist/index.js` (7,525 lines each) plus their `.d.ts`, a bundled build artefact of `ix-spec-workflows`; the only hand-written assertion logic is 139 lines of `scripts/invariants.js` that re-export `specInvariants` from it. Classifying `skills/**/workflow-assets/**` wholesale as first-party executable logic in scope for the burn-down inflates the baseline by ~21% and asserts ownership of another repository's build output — the same argument FR-103 uses to exclude the `corpus/` submodule. | FR-099; FR-101; FR-101-AC-9; StR-009 |
| FND-174 | high | The ajv-to-`jsonschema` parity claim rests on a validator that asserts no formats: `ajv-formats` is not a dependency anywhere in the repository, yet the schemas declare `"format": "date-time"` nine times and `"format": "uuid"` once. Under ajv those keywords are inert annotations; a Rust validator that asserts them changes the verdict on real documents, and the differential harness will read that as a Rust defect rather than as a deliberate semantic choice. The set never states whether format assertion is on. | FR-098-AC-2; FR-099-AC-6; FR-100 |
| FND-175 | high | The "exactly one crate" clause names canonical JSON, JCS and blake3, but the identity that matters most is none of those: `src/evidence/assurance-records.ts` computes `recordId` as `sha256:<hex>` via `node:crypto` `createHash`, names files `sha256-<hex>.json`, and validates every cross-record citation against that pattern; `src/quire/contract.ts:103` hashes schemas with sha256 too. blake3 is the change-assurance domain only. A `quoin-store` that owns "canonicalization and digest" as specified can satisfy FR-098-CON-2 and FR-100-CON-4 while the sha256 record-identity path is reimplemented somewhere else. | FR-098-CON-2; FR-100-CON-4; NFR-025 |
| FND-176 | high | Record identity includes a refusal boundary nothing in the set ports: `src/change-assurance/integrity.ts` hand-writes `StrictJsonParser`, rejecting a UTF-8 BOM, non-fatal UTF-8, and trailing content before canonicalization runs. `serde_json` accepts or rejects a different set (duplicate keys, number grammar, lone surrogates), so a document the retained path refuses can canonicalize and hash cleanly in Rust. FR-098-AC-4 tests JCS over adversarial input but tests the canonicalizer, not the parser that guards it. | FR-098-AC-4; FR-100; NFR-025-AC-2 |
| FND-177 | high | The normalized-diagnostic equality in FR-098-AC-2 is asserted over five ajv call sites that do not agree with each other: `src/quire/validate.ts:50`, `src/measurement/operational.ts:29` and `src/semantic/manifest.ts:71` use `strict: false`, while `src/semantic/manifest.ts:282` and `src/semantic/package-manifest.ts:100` use `strict: true`. ajv strict mode is schema-authoring analysis with no counterpart in the `jsonschema` crate, and `allErrors: true` makes the error *set* — not just its text — part of what is compared, which for `anyOf`/`oneOf` differs structurally between the two engines. As written the property is likely false on the first non-trivial schema. | FR-098-AC-2; FR-098 |
| FND-178 | medium | FR-096-AC-6 pins the payload test at exactly 67,108,864 bytes, which is exactly `QUIRE_MAX_BUFFER = 64 * 1024 * 1024` in `src/quire/exec.ts:24`. Node throws `ERR_CHILD_PROCESS_STDIO_MAXBUFFER` when output *outgrows* maxBuffer, so the criterion names the boundary at which the incident it descends from (agent-ix/quoin#164, ENOBUFS) recurs — any framing byte or trailing newline puts the run over. The AC needs a payload below the cap, or a cap above the AC. | FR-096-AC-6; FR-096 |
| FND-179 | medium | FR-099-AC-8 is partly tautological and partly miscounted. "The workspace declares no dependency on `@agent-ix/ix-cli-core`" is trivially true of a Cargo workspace, which cannot express an npm dependency — the assertion belongs over `package.json`. And the count is wrong: `src/` imports twelve distinct symbols (`BaseCommand`, `ConfigService`, `RunnerLoadOptions`, `loadConfig`, `run`, `maybeOfferUpdate`, `registerPluginSchema`, `runConfigDoctor`, `runConfigEdit`, `runConfigGet`, `runConfigSet`, `runSelfUpdate`), not nine, so a gate keyed to nine passes with three unported. Two of them, `runSelfUpdate` and `maybeOfferUpdate`, *are* the marketplace surface the same requirement says not to port. | FR-099-AC-8; FR-099 |
| FND-180 | medium | The publication constraint contradicts the repository's current published state rather than preserving it: `package.json` declares `publishConfig.registry = https://registry.npmjs.org/` with `access: public`, so `@agent-ix/quoin` is a public npmjs package today. FR-097, FR-102-AC-7 and FR-103 all forbid public npmjs. This is an owner ruling about an already-published artefact (unpublish, deprecate, or exempt), not a manifest edit a task can carry. | FR-097; FR-102-AC-7; FR-103 |
| FND-181 | medium | NFR-026's floor is real but its gates reach outside the repository. Measured: `quire-rs` and `filament-core-data` both pin 1.94.1; `engineering-assurance` pins 1.98.1. AC-4 therefore requires a test that reads another repository's `rust-toolchain.toml` with no offline source named, which is an external-contract test that will be skipped or stubbed; and AC-3 ("a second declaration of the channel anywhere in the repository fails the gate") collides with any CI action that names a toolchain version, so the CI lane must be written to install from the file only. | NFR-026-AC-3; NFR-026-AC-4 |
| FND-182 | medium | Three gates are written over a population nothing enumerates — "every digest in every reachable store", "every store reachable in the ecosystem" — while the same set declares that a run over an empty or unnamed population is inconclusive and cannot satisfy a cutover gate. As written the store-backed cutover can neither pass nor be scoped: the requirement must name the store roots (this repository's evidence root, and which others by what discovery rule) before FR-100 is tasked. | FR-098-AC-3; FR-100-AC-2; NFR-025-AC-1 |
| FND-183 | medium | NFR-024 decides the metric's opening value and two of its inputs do not exist: delivery-stage tickets 0 through 9 are prose in the epic body, and `.language-allowances.yaml` is absent, so on the day enforcement lands essentially the whole 105k-line surface classifies as violation rather than retention. The requirement also contradicts itself on the manifest's location — Verification names `quoin/.language-allowances.yaml` "checked in at the repository root", AC-6 names `.language-allowances.yaml` at the root. Create the stage issues and the manifest as stage 0, before the classifier. | NFR-024-AC-1; NFR-024-AC-6; NFR-024 |
| FND-184 | medium | The oclif extension contract is the set's only requirement deferred to a dated decision that falls ~46 weeks out, and it is wider than the set treats it: `@agent-ix/filament-plan-sync` 0.1.0 is a runtime `dependencies` entry as well as an `oclif.plugins` declaration, and `command_not_found` is a published hook whose consumers are outside this repository and untestable from it. Deciding it at stage 8 makes the decision cheap to defer and expensive to reverse; FR-102-AC-3 gates on the disposition existing but nothing dates it earlier. | FR-102-AC-3; FR-102; US-024 |
| FND-185 | low | Several artefacts are described in the present tense and do not exist at `30425e6`: ADR-0003 states `src/core/exec.ts` "is a copy-edit" of `src/quire/exec.ts` and FR-102-AC-6 asserts its deletion, but neither `src/core/` nor `rust/` nor `.language-allowances.yaml` is in the tree. Relatedly, FR-101-AC-9's fixed figures are revision-bound: the same measurement over `git ls-files` at HEAD yields 105,489 lines across 363 files against the quoted 105,814 across 364 at `e718d45`, so the AC must cite the baseline revision as the object of the assertion rather than assert the numbers at the revision under test. | FR-101-AC-9; FR-102-AC-6; FR-096 |
| FND-186 | low | The parity-oracle population is cited as 109 TypeScript test files; the tree carries 105 `*.test.ts` files under `tests/`. The `tests/props` count the port must mirror is right — seven `fast-check` suites — but the headline number the programme will burn down against should be restated with its measurement, since FR-101-AC-4 makes each retired test's criterion an auditable row. | FR-101-AC-4; FR-098-AC-6; US-024 |

## Risk register

| Req | Tech Risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| StR-009 | High | Medium | Ten stages, ~46 weeks, one language boundary and one schema source held simultaneously; VC-2's "no unclassified path" rests on a baseline that is 21% vendored bundle (FND-173) | Re-base the inventory before stage 0 tickets are cut; classify `workflow-assets/dist/**` as another repository's artefact and state its owner |
| US-024 | Medium | High | Story spans a 46-week critical path with three open owner rulings (extension contract, `qa-corpus` scope, toolchain disagreement); oracle population misquoted (FND-186) | Keep the story's reversibility and empty-population examples as standing gates; restate the oracle count with its measurement |
| FR-096 | Medium | Low | Boundary is already built and hardened; risk is the payload boundary condition (FND-178) and a type-surface check over a surface that does not exist yet (FND-185) | Spike stage 0 end to end on one operation; move AC-6's payload below `QUIRE_MAX_BUFFER` or raise the cap deliberately and say which |
| FR-097 | Medium | High | Gated on `filament-core-data` Phase B, which publishes nothing today; registry posture contradicts the shipped manifest (FND-180) | Land generation and provenance now, retirement behind Phase B; take the public-npmjs ruling to the owner before stage 0 closes |
| FR-098 | High | Low | ajv format posture unstated (FND-174); strict-mode split and `allErrors` set cardinality (FND-177); parser refusals outside the canonicalizer (FND-176); replay population unbounded (FND-182) | Spike the differential on `quoin-validators` first as ADR-0003 directs; state the format posture and the diagnostic-tuple domain as spec text; add a parser-refusal corpus beside the JCS corpus; name the store roots |
| FR-099 | Medium | Medium | Twelve `ix-cli-core` symbols not nine, one AC vacuous (FND-179); skills-root and flow-root resolution are two different lookups (`src/flows.ts` resolves `IX_SPEC_WORKFLOWS_ROOT` and a sibling checkout first); strict manifest parsing must be preserved exactly | Enumerate the twelve behaviours in the requirement; re-point AC-8's first clause at `package.json`; state which root FR-099's fallback covers |
| FR-100 | High | Low | sha256 record identity outside the named single-crate clause (FND-175); byte-identical re-serialization of every store; two dependency cycles broken in TypeScript before any crate exists; replay population unnamed (FND-182) | Name sha256 record identity in the single-crate clause; land both cycle breaks as behaviour-neutral refactors in stage 0; enumerate the store roots and report the covered population with every replay |
| FR-101 | Medium | Medium | Disposition completeness over a mis-sized inventory (FND-173, FND-185); the fixture-capture rule retires the oracle correctly but needs the capture to precede each cutover | Re-base the inventory first; make fixture capture an explicit step of each cutover ticket rather than a property of the deletion ticket |
| FR-102 | Medium | High | Published extension contract undecided for ~46 weeks and wider than stated (FND-184); registry posture (FND-180); 59-command snapshot must hold across every earlier stage | Date the extension disposition now and record it as an ADR amendment; keep the command-surface snapshot as a per-revision gate from stage 0, not from stage 8 |
| FR-103 | Low | High | Scope depends on an open owner ruling over `agent-ix/qa-corpus`; accounting must come from `engineering-assurance`, which may not supply it | Keep the Quoin-side consolidation as its own slice; file the `engineering-assurance` gap ticket early enough that stage 6 is not blocked on it |
| NFR-024 | Medium | High | Successor tickets and the allowance manifest do not exist, so the metric opens at near-total violation; internal path contradiction (FND-183) | Create stage issues 0–9 and `.language-allowances.yaml` as stage-0 deliverables, before the classifier runs; fix the manifest path to one spelling |
| NFR-025 | High | Low | Digest identity is unrecoverable if it moves; guarantee must hold through cutover, revert and deletion; depends on FND-175 and FND-176 being closed | Hard-gate the store-backed cutover on a named, non-empty replay; run the planted-single-byte probe in the same lane so the gate is proven to fail |
| NFR-026 | Low | Medium | Upstream repositories measured at 1.94.1 against a 1.98.1 floor; AC-4 reads other repositories, AC-3 forbids a second declaration (FND-181) | Declare the channel once in `rust/rust-toolchain.toml` and have CI install from it; make AC-4 a reported disagreement with a checked-in expected-pins record rather than a live cross-repository read |
| NFR-027 | Low | Low | Mechanical source and test checks over a workspace that does not exist yet; the only real risk is a check that scans nothing | Land the checks with the first crate so the scanned population is never zero; keep AC-9's inconclusive rule as the guard |
| IT-003 | Medium | High | Cannot run at all until `filament-core-data` Phase B closes; today all six of its crates declare `publish = false`; needs clean-environment registry credentials | Keep it recorded as spec-ahead-of-code beside IT-001/IT-002; do not make any stage-0..3 exit depend on it |

## Top hazards

1. FND-173 — the burn-down's headline number is 21% another repository's triplicated build artefact. Re-base the inventory before stage tickets are cut, or every velocity figure the programme publishes for 46 weeks is quoted against a population that is not Quoin's.
2. FND-175 with FND-176 — the identity domain the programme promises not to move is sha256 record ids plus a hand-written strict JSON parser, neither named by the "exactly one crate" clause. This is the one failure mode NFR-025 calls unrecoverable, and the current wording lets a conforming implementation miss it.
3. FND-174 with FND-177 — ajv asserts no formats and disagrees with itself on strict mode across five call sites, so FR-098-AC-2's normalized-tuple equality is probably false before a line of Rust is written. Settle the format posture and the diagnostic domain as spec text, on the `quoin-validators` leaf, before stage 2.
4. FND-182 with FND-183 — three cutover gates run over populations nothing enumerates, and the coexistence metric opens at near-total violation because its successor tickets and allowance manifest do not exist. Both are stage-0 bookkeeping that decides whether the programme's own evidence is readable.
5. FND-184 — the published oclif extension contract is deferred ~46 weeks and is wider than the set records. Date the disposition now; it is the only item here whose cost rises with every week it stays open.

## Failure-domain gaps

No `spec-failure-domain-analysis` deliverable exists for this batch. Four
findings in this review are failure-domain shaped and should be carried into one
when it is run: identity confusion across three digest domains (FND-175),
purity and refusal-boundary loss at the JSON parser (FND-176), the extension
topology of the published oclif contract (FND-184), and the empty-population
edge case that three gates share (FND-182).
