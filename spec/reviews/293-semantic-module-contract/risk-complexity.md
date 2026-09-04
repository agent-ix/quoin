---
id: SR-123
title: "Risk and complexity review of issue 293 semantic module contract requirements"
type: SpecReview
analysis: risk-complexity
scope: "US-020, FR-070..FR-075, NFR-017, spec/matrix.md TC-1336..TC-1382"
review_set: all
---

# Risk and complexity review of issue 293 semantic module contract requirements

## Summary

The bundle is a contract specification whose executable half lives in another
repository: quire-rs (`agent-ix/quire-rs#388`) implements the extraction, the
manifest loader, the digest check, and the legacy-form validator, while quoin
owns the `semantic` block schema, the authoring-pack projection, the golden
fixtures, the derived package manifest and lock entries, and the legacy-form
diagnostics text. Technical risk is moderate and concentrated in four places:
two manifest loaders that today both ignore a `semantic` key, a table-to-fence
equivalence claim over unequal grammars, a lock and install surface that does
not yet exist, and offline digest and `$ref` resolution against a private,
unpublished semantic-core. Volatility is high wherever a requirement leans on an
artifact another open ticket has not yet defined (#287 lock, #291 sweep record,
semantic-core publication). Nothing here blocks specification; five items should
be sliced or spiked before `spec-to-plan` tasks them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-131 | medium | The manifest-rejection rules have no owner today: quoin's `src/catalog.ts` parses `manifest.yaml` as untyped YAML and reads only `name`, `version`, `artifact_types`, `object_types`; quire-rs `src/loader/manifest.rs` deserialises with `#[serde(default)]` and no `deny_unknown_fields`, so a `semantic` block is silently dropped — the "degrade to an empty model" the requirement forbids is the shipped behaviour until #388 lands, and quoin cannot test AC-3..AC-6 without its own loader or a quire that carries them. | FR-070-AC-3..AC-6; TC-1338..TC-1341 |
| FND-132 | medium | The `semantic` key vocabulary is split across three requirements: FR-070 enumerates `contract_version`, `semantic_core`, `package`, `exports`, `targets`, `mappings`; FR-074 adds `legacy_forms`; FR-075 adds a compatibility posture with no key name. Under `additionalProperties: false` the schema must enumerate all of them or FR-074/FR-075 blocks fail FR-070-AC-3. | FR-070-CON-2; FR-074; FR-075 |
| FND-133 | medium | Table-to-fence byte-identity is claimed over unequal grammars: the fence subset includes `:> <Type>` (a `RelationDecl` target) and `item` composite-owned fields, neither of which the four-column table can express, so the equivalence property in TC-1345 needs a stated domain (fence content restricted to what the table can say) or it is unfalsifiable on the interesting cases. | FR-071; TC-1345; US-020-EX-2 |
| FND-134 | medium | The mapping targets fields semantic-core 0.1.0 does not have: `ClauseRef.json` closes over `language`, `clauseId`, `sourceSpan` (`unevaluatedProperties: not {}`), so the external `clause: <path>` form has no landing field, and `SourceLocus.json` is line/column, not the byte span FR-072 names. Either the grammar changes (filament-core-data volatility) or the mapping does. | FR-072-AC-1; FR-072-AC-6 |
| FND-135 | high | FR-075 rests on surfaces that do not exist: quoin has no catalog lock (ts-plugin-kit `registry.json` only; `agent-ix/quoin#287` open), no `quoin install` command (the command is `quoin module install`), and `package-manifest.schema.json` requires `sourceRoots`, `profiles`, `imports[].versionConstraint`/`capabilities`, `extensions`, which the manifest as specified cannot supply. Highest slip risk in the bundle. | FR-075-AC-1..AC-3; TC-1372..TC-1374 |
| FND-136 | medium | Offline digest and `$ref` resolution is under-constrained: SHA-256 over file bytes is line-ending sensitive across checkouts, every referenced schema `$id` lives under `https://schemas.agent-ix.org/...` and must resolve from a vendored bundle inside each module, and `@agent-ix/semantic-core` is `private: true` at 0.1.0 with no registry to pin against, so `semantic.semantic_core` is a version string with nothing to resolve it. | FR-070; FR-073-AC-1..AC-3; FR-073-CON-1 |
| FND-137 | medium | The promotion guard depends on a "recorded advisory sweep report" from `agent-ix/quoin#291`, which is open and defines neither where the record lives nor its shape; the negative half of TC-1369 cannot be written until it does. | FR-074-AC-3; NFR-017 |
| FND-138 | low | Two resolution rules are looser than their tests: `Type` resolves by object title as well as `id`, which is a bundle-wide pass that titles can break; and `ClauseLanguage` admits any namespaced `<ns>:<name>`, so `tla` fails but `x:tla` passes with no checker behind it. | FR-071-AC-4; FR-072-AC-2 |
| FND-139 | low | Verification is concentrated in one planned file and gated on a quire binary that does not exist yet: 25 of the 47 test cases exercise extraction, loading, digest, or legacy detection that only quire-rs#388 implements, quoin runs quire-backed tests only under `make test-with-quire`, and the installed `quire 0.31.0` carries none of it. | TC-1344..TC-1350; TC-1353..TC-1358; TC-1360..TC-1364; TC-1366..TC-1369; TC-1371; TC-1379; TC-1380 |

## Risk register

| Req | Tech Risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| US-020 | Medium | High | Story spans three repositories and four open tickets (#388, #287, #290, #291); two authoring forms | Land FR-070 and FR-071 fixtures first; keep FR-075 and the promotion path as separate slices |
| FR-070 | Medium | Medium | Two loaders, neither enforcing the block today (FND-131); key vocabulary split (FND-132); exact semantic-core version with no registry (FND-136) | Publish the `semantic` JSON Schema from quoin as the single source both loaders validate against; enumerate all keys in one table; quoin tests the schema and the pack, quire tests the rejections |
| FR-071 | High | Medium | Two grammars claimed equivalent (FND-133); title-based resolution (FND-138); extraction lives in quire | Golden fixtures with a stated fence domain; property test over table-expressible content only; fixtures pinned by digest so quire-rs#388 consumes them unchanged |
| FR-072 | Medium | High | Grammar lacks the external-reference field and byte spans (FND-134); namespaced languages unchecked | Resolve against filament-core-data before tasking: add the field or drop the `clause:` form; state the span unit |
| FR-073 | High | Medium | Digest determinism, offline `$ref` bundle, private semantic-core (FND-136); path-escape guard | Spike: emit one module's `Entity.json`, vendor the semantic-core bundle, load under quire with `$id` mapping; normalise bytes before hashing and say so |
| FR-074 | Low | High | Guard depends on an undefined #291 record (FND-137); detection rules are simple | Ship detection and the warning first; keep `legacy_forms: error` and the guard as a slice after #291 defines its record |
| FR-075 | High | High | No lock, no `quoin install`, schema needs fields the manifest lacks (FND-135); coordinates only | Split: (a) derived manifest with the missing fields defaulted and named, (b) lock digests after #287, (c) import failure after #287; property test digest-changes-with-schema |
| NFR-017 | Low | Low | Measurement is a diff of `required` arrays, a default-module load, and a changed-path gate | Keep as quality gate; AC-2 sweep needs the #291 fixture set, otherwise runs on quoin's own fixtures |

## Top hazards

1. FR-075 — lock, install command, and package-manifest fields do not exist; slice behind #287 or it slips the whole bundle (FND-135).
2. FR-070 — the block is silently ignored by both loaders today, so a module can ship a `semantic` block that nothing reads; decide which loader owns each rejection before tasking (FND-131, FND-132).
3. FR-071 — the byte-identity property is over unequal grammars; state the fence domain or the golden fixtures prove less than the requirement says (FND-133).
4. FR-073 — offline digest and `$ref` resolution against an unpublished semantic-core; spike one module end to end before the mapping is called done (FND-136).
5. FR-072 — semantic-core 0.1.0 cannot hold the external clause reference or a byte span; this is a filament-core-data change or a mapping change, not a quoin task (FND-134).

## Quoin-owned versus quire-only verification

| Surface | Owner | Test cases |
| --- | --- | --- |
| `semantic` block JSON Schema, `required` diff, `additionalProperties` | quoin | TC-1342, TC-1343, TC-1382 |
| Authoring pack (`quoin write`) projection and migration example | quoin | TC-1337, TC-1370 |
| Golden fixtures with provenance | quoin | TC-1352 |
| Derived package manifest, lock digests, import failure, identity parity, URL rejection | quoin | TC-1372..TC-1378 |
| Static boundaries (no clause evaluation, no network, no publication, no corpus write) | quoin | TC-1351, TC-1359, TC-1365, TC-1377, TC-1381 |
| Default-module load without and with `semantic` | quoin via quire | TC-1336, TC-1366, TC-1379 |
| Manifest rejections (unknown key, export, version, duplicate package) | quire (or a quoin loader that does not exist) | TC-1338..TC-1341 |
| Table, fence, type, multiplicity, constraint, clause, operation extraction | quire only | TC-1344..TC-1350, TC-1353..TC-1358 |
| `data_schema` by path and digest, mismatch, version drift, inline warning, path escape | quire only | TC-1360..TC-1364 |
| Legacy-form detection, promotion, fixture-suite invariance, warning-only sweep | quire only | TC-1367..TC-1369, TC-1371, TC-1380 |

## Failure-domain gaps

The failure-domain analysis for this bundle (SR-119..SR-125 in this directory)
is pending; the base review (SR-118) records no open failure-domain gap. The
identity and purity hazards this register raises independently are the
duplicate `semantic.package` across modules (FR-070-AC-6), the digest as the
only identity of an emitted schema (FR-073), and the two-authority risk between
table and fence (FR-071).
