---
id: SR-122
title: "Evidence-method review of the semantic module contract"
type: SpecReview
analysis: evidence
scope: "US-020, FR-070..FR-075, NFR-017, spec/matrix.md TC-1336..TC-1382, spec/evidence/suites.md"
review_set: all
---

# Evidence-method review of the semantic module contract

## Summary

The deterministic advisor was run with Quoin 0.23.1 and Quire 0.31.0
(`cli 4f6ed024`, `engine 0.46.0@ca7362d4`) over the 42 obligations the scope mints
(33 acceptance criteria, 5 NFR measurement rows; the 10 constraint rows are coverage
obligations but the advisor emits no advice for them). It reports two mismatches
(NFR-017-AC-3, NFR-017-AC-4), one inconclusive obligation (FR-074-AC-1), and six
uncatalogued authored methods (FR-074-AC-1, NFR-017-M-1..M-5). The rest confirm `Test`
as property-based, example-driven, or golden testing. The residue below is judgement,
labelled as such: the one real defect is a table-parse fault that mangles FR-074-AC-1's
obligation, and the one planning gap is that no registered suite produces the `Property`
or non-validator `Static` evidence the matrix relies on.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-121 | high | FR-074-AC-1's criterion cell contains the unescaped header `` `Column \| Type \| Constraints` ``, so the row splits into extra columns: the obligation's statement is truncated to `...free-column table \`Column` and its authored method reads `Type` (uncatalogued, inconclusive). TC-1367 (Unit) binds to an obligation whose hash covers a mangled statement, not the criterion the author wrote. Escape the pipes or reword the cell. | FR-074-AC-1; TC-1367 |
| FND-122 | medium | NFR-017-M-1..M-5 author prose in the `Method` column (`Load every module in default-modules.yaml`, `Corpus sweep`, `Changed-path gate`, `Manifest schema diff`, `Loader test`); none is a catalog method, so `quoin evidence` can discharge none of them. The advisor's `performance-benchmarking` recommendation (matched on `quantified-threshold`) is a false hit for zero-count gates. Judgement: M-1 and M-5 are `integration-testing`/`unit-testing` already carried by AC-1 and FR-074-AC-3; M-2 is `integration-testing`; M-3 and M-4 are `inspection`-class static gates outside any suite (see FND-125). | NFR-017-M-1..M-5 |
| FND-123 | medium | Three constraint rows are authored `Inspection` (FR-071-CON-1 `Extraction-only inspection`, FR-072-CON-1, FR-075-CON-1) but the matrix types their cases `Static` (TC-1351, TC-1359, TC-1377). The catalog's `inspection` method yields `Manual` evidence discharged through the inspections registry, not a tagged test; a `Static` TC row for a negative-existence claim ("no clause typechecking code path exists") has no automated oracle in any registered suite. Either author them as `Static` with a named grep/architecture-conformance check, or keep `Inspection` and drop the TC type to `Manual`. | FR-071-CON-1; FR-072-CON-1; FR-075-CON-1; TC-1351; TC-1359; TC-1377 |
| FND-124 | low | FR-073-CON-1 is authored `Sandbox test` (run with network denied) but TC-1365 is typed `Static`. A no-network guarantee is an `integration-testing` claim at an I/O boundary; a static grep for `fetch`/`http` proves absence of a call site, not absence of a read. Align to `Integration`, or author the constraint as a static check and say what it greps. | FR-073-CON-1; TC-1365 |
| FND-125 | medium | The suite registry offers Unit (`make test`), Static (`make validate`) and Eval only. TC-1345 is `Property` and no suite produces `Property` evidence; the `Static` rows TC-1342, TC-1343, TC-1352, TC-1378, TC-1381, TC-1382 need a schema diff, a fixture-provenance check, a derived-document scan, and a changed-path gate, none of which `quire validate` performs. With SUITE-001 binding zero obligations (suites.md), every TC in scope is `undischarged` on landing; the plan needs a suite (or a CI job registered as one) per evidence kind before the matrix can go green. | spec/evidence/suites.md; TC-1342..TC-1343; TC-1345; TC-1352; TC-1378; TC-1381..TC-1382 |
| FND-126 | low | NFR-017-AC-3 (`Analysis`, TC-1381 Static) and NFR-017-AC-4 (`Analysis`, TC-1382 Static) are the advisor's only mismatches: `property-based-testing` on the universal determiner ("No corpus repository path") and `unit-testing` on the example shape. Judgement: both are static comparisons of a change set and a schema diff with no runtime subject; `Analysis` stands and the matrix agrees. Recorded so the mismatch flag is not re-litigated. | NFR-017-AC-3; NFR-017-AC-4; TC-1381; TC-1382 |
| FND-127 | low | FR-071-AC-2 is example-shaped ("The equivalent `sysml` fence extracts to a byte-identical...") while TC-1345 is typed `Property`. The universal claim lives in FR-071's Behavior bullet ("Table-authored and fence-authored artifacts with equal content SHALL extract to byte-identical..."), not in the criterion. Either restate AC-2 universally (generated table/fence pairs) so the `Property` type is earned, or type TC-1345 `Unit` and add a golden fixture, which the advisor also recommends (`golden-approval-testing` on `stable-output`). | FR-071-AC-2; TC-1345 |
| FND-128 | low | Four recommendations come from lexical false hits, not from the requirement: `dast` on FR-070-AC-4 (`network-exposed` matched the object-type name `endpoint`), `dast`/`iast`/`sast`/`negative-abuse-testing` on FR-071-AC-4 (`security` matched "resolve ... advisory"), and `model-checking`/`runtime-monitoring` on FR-073-AC-3 (`temporal` matched "while"). Rejected by judgement; `Test` stands for all three. | FR-070-AC-4; FR-071-AC-4; FR-073-AC-3 |
| FND-129 | low | FR-071 and FR-072 define four small grammars (Type cell, Multiplicity cell, Constraints list, SysML subset lines) that parse untrusted artifact text, but no criterion carries a `parser`/`untrusted-input` characteristic, so the advisor never minted `fuzzing` or `grammar-based-fuzzing`. Judgement: plan a grammar-based fuzz target over the four cell grammars alongside the example tests; the matrix has no row for it. | FR-071-AC-4..AC-7; FR-072-AC-2 |
| FND-130 | low | FR-070-CON-1 (`Manifest schema diff`, TC-1342), NFR-017-M-4 (`Manifest schema diff`) and NFR-017-AC-4 (TC-1382) are the same check under three ids; NFR-017-M-1 and NFR-017-AC-1 and FR-070-AC-1 (TC-1336, TC-1379) are the same default-module load. One suite run should discharge each set; the plan should bind one test to all of its ids rather than write it three times. | FR-070-CON-1; FR-070-AC-1; NFR-017-AC-1; NFR-017-AC-4; NFR-017-M-1; NFR-017-M-4; TC-1336; TC-1342; TC-1379; TC-1382 |

## Method allocation

| Claim shape | Obligations | Evidence |
| --- | --- | --- |
| Loader rejection with a named diagnostic (unknown key, undeclared export, out-of-range contract version, duplicate package, digest mismatch, version drift, path escape, URL identity, missing lock entry) | FR-070-AC-3..AC-6; FR-073-AC-2, AC-3, AC-5; FR-075-AC-3, AC-5 | Unit tests over adversarial manifests; property tests where the advisor found a universal determiner |
| Mapping from authored form to declaration set (typed table, fence, clause, operation) | FR-071-AC-1, AC-4..AC-6; FR-072-AC-1, AC-4 | Unit tests against the golden fixtures FR-071-CON-2 publishes; fixture validated against semantic-core JSON Schema |
| Two authored forms, one result | FR-071-AC-2 | Golden fixture now; property test over generated pairs once the fence renderer exists (FND-127) |
| Second-authority rejection at locus (both forms, duplicate clause id, inline plus external clause, dangling pre/post) | FR-071-AC-3; FR-072-AC-3, AC-5, AC-6 | Unit tests asserting the locus, not only the failure |
| Cell and fence grammars | FR-071-AC-5..AC-7; FR-072-AC-2 | Unit examples plus grammar-based fuzz target (FND-129) |
| Legacy-form advisory and promotion guard | FR-074-AC-1..AC-3; FR-073-AC-4 | Unit tests on the unmodified FR-006 fixture; existing-fixture suite for FR-074-CON-1 |
| Authoring pack text | FR-070-AC-2; FR-074-AC-4 | Unit test on `quoin write` output |
| Compatibility of the installed set | FR-070-AC-1; NFR-017-AC-1, AC-2; FR-073-CON-2 | Integration run over `default-modules.yaml` and the corpus fixture set, report mode |
| Static gates (schema `required` diff, `additionalProperties: false`, changed-path gate, `ix://` identities) | FR-070-CON-1, CON-2; FR-075-CON-2; NFR-017-AC-3, AC-4 | Analysis: a registered static suite the registry does not yet carry (FND-125) |
| Boundary constraints (no clause semantics, no publication, extraction-only fence handling) | FR-071-CON-1; FR-072-CON-1; FR-075-CON-1 | Inspection through the inspections registry, or an architecture-conformance check if made static (FND-123) |
| Package derivation and lock fingerprint | FR-075-AC-1, AC-2, AC-4 | Unit tests validating against `package-manifest.schema.json`; lock digest change asserted by mutating an emitted schema |

The advisor emitted no advice for the ten `CON` rows although `quire coverage` mints
them as obligations; their `Validation` column was compared by hand against the matrix
`Type` column, which is where FND-123 and FND-124 come from. FR-074-AC-1's parse fault
(FND-121) is the only finding that changes an obligation hash; everything else is a
planning or typing correction and leaves the `Verification` cells as authored.
