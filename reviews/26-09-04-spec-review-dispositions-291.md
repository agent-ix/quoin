---
id: SR-148
title: "Dispositions for the composite review of the advisory corpus measurement"
type: SpecReview
analysis: base
scope: "SR-140..SR-147; US-022; FR-084..FR-092; NFR-021..NFR-023; TC-1500..TC-1584"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "reviews"
  - target: "ix://agent-ix/quoin/issues/291"
    type: "references"
---

# SR-148: Dispositions for the composite review of the advisory corpus measurement

## Summary

Eight analyses ran over the artifacts of commit `2e5d704` — base (SR-140), dependency (SR-141),
EARS conformance (SR-142), evidence (SR-143), failure domain (SR-144), integrity (SR-145),
risk-complexity (SR-146) and scope boundary (SR-147). They raised eighty findings, FND-1400 through
FND-1479. This review records what each one caused.

Three of them, raised independently by three analyses, changed the shape of the work rather than its
wording. SR-147 FND-1470/1471/1472 said the measurement was building a second extractor inside Quoin
for work the epic allocates to Quire; SR-146 FND-1460/1461 said the semantics that a second
implementation would get wrong are stated as English prose, so the disagreement would be silent; and
SR-144 FND-1445 said a module-supplied pattern language evaluated over 24,631 documents is an
unbounded failure surface. They are right, and a fact found while checking them settles it: the
released `quire` CLI 0.31.0 pins engine revision `ca7362d4`, which predates the `quire-rs#388` merge
`17b80e4` that added semantic extraction, and no released CLI surface exposes that extraction at all.

So FR-087 was rewritten to drive the Quire engine and record its diagnostics, and FR-088 was
rewritten to publish the Properties form census — which FR-074 already makes Quoin's — and to record
the field-level semantic dimension as `could-not-run` for every document, citing the toolchain that
cannot decide it. The measurement now reports what the released contract checker actually says, and
reports its own blindness as a number rather than as agreement.

## Verdict

**CONDITIONAL** — 72 findings applied, 8 recorded as not applied or deferred with a reason and an
owner. No finding was closed by weakening a constraint.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-1480 | medium | Nine NFR measurement `Method` cells remain uncatalogued prose (SR-143 FND-1434), deferred to the evidence-catalog owner | spec/non-functional/NFR-021-reproducible-corpus-measurement.md |
| FND-1481 | medium | No suite in the repository can produce `Property` or `Benchmark` evidence for the new rows (SR-143 FND-1438); the plan must stand one up or retype the rows | spec/matrix.md |
| FND-1482 | low | Module staleness is recorded but not bounded (SR-146 FND-1462); the bound belongs to the promotion gate | spec/functional/FR-085-resolve-the-completed-module-set.md |

## Finding detail

### FND-1480 — the NFR measurement methods are prose, not catalog entries

SR-143 FND-1434 found that all nine `Method` cells across NFR-021, NFR-022 and NFR-023 are free
prose, where NFR-004, NFR-005, NFR-009, NFR-010, NFR-012 and NFR-016 each name a catalogued method.

Failure scenario: a reader cannot tell whether "Repeat run and compare SHA-256 digests" is a method
the repository knows how to run or a sentence somebody wrote. Deferred rather than guessed: naming a
catalogued method that does not exist would be worse than naming none. Owner: agent-ix/quoin, on the
evidence-method catalog.

### FND-1481 — no suite produces the evidence kinds the new rows claim

The rows added by this work include ten `Property` rows and two `Benchmark` rows. SR-001 registers
only Unit, Static and Eval suites, so nothing today produces either kind for this area.

Failure scenario: the rows stay `🚧` for ever, or somebody quietly retypes them `Unit` and the
universality the report's honesty depends on is gone. The plan carries this: either the property and
benchmark harnesses exist before those rows are claimed, or the rows say so. Owner: agent-ix/quoin.

### FND-1482 — staleness of the measured module revisions is recorded, not bounded

FR-085-AC-6 records, per module, whether the measured commit equals the commit the catalog pin
resolves to, and the report names every divergence. SR-146 FND-1462 asked for a bound on how far the
measured contract may run ahead of any release.

Failure scenario: a rate is published against a contract that never ships. Not applied here because
the bound is a promotion decision, not a measurement one: agent-ix/quoin#290 owns whether an
unreleased contract may be promoted, and this campaign is forbidden to take that decision. Owner:
agent-ix/quoin#290.

## Disposition ledger

Every finding raised by SR-140..SR-147, and what it caused.

| Finding | Review | Sev | Disposition |
| --- | --- | --- | --- |
| FND-1400 | SR-140 | high | Applied — a legacy form is an advisory finding of class `unsupported-representation` (FR-088), never a conformance failure, and never in a rate |
| FND-1401 | SR-140 | high | Applied — FR-085-CON-3 and FR-085-AC-10 make the required module set a declared input naming all ten modules; US-022 names them |
| FND-1402 | SR-140 | med | Applied — FR-085 exits non-zero for an unresolved required module, before any document is read; FR-092-AC-6 restated to match |
| FND-1403 | SR-140 | med | Applied — FR-089's finding stream now includes advisory findings, module findings and contested-type findings, so a `tool-defect` classification has something to classify |
| FND-1404 | SR-140 | med | Applied — FR-090 publishes a zero-denominator partition with no rate value |
| FND-1405 | SR-140 | med | Applied — FR, NFR and Use Case coverage tables extended; US-022-EX-1..EX-4 each name their rows |
| FND-1406 | SR-140 | med | Applied — FR-091-CON-1 is verified by TC-1581 |
| FND-1407 | SR-140 | med | Applied — NFR-022 names a population of up to 30,000 documents in 300 repositories and records the machine it measured |
| FND-1408 | SR-140 | med | Applied — FR-084 takes a declared corpus identifier and states that the `quire-rs#385` fixture corpus is a different population |
| FND-1409 | SR-140 | low | Applied — FR-088 declares FR-084, FR-085, FR-091 upstream and FR-089, FR-090 downstream |
| FND-1410 | SR-141 | high | Applied — FR-091 names quire-rs#402, quire-rs#403, spec-artifacts-process#81, quoin#347 and the absent `#388` CLI surface; FR-085 cites quoin#347 as the reason it reads the object store |
| FND-1411 | SR-141 | med | Applied — FR-090's Inputs and frontmatter carry FR-084, FR-086, FR-087, FR-088 |
| FND-1412 | SR-141 | med | Applied — frontmatter `depends_on` extended on FR-086, FR-087, FR-088, FR-089, FR-090 and NFR-021 |
| FND-1413 | SR-141 | med | Applied — every configuration input is declared as an input in its owning requirement and recorded verbatim in the output |
| FND-1414 | SR-141 | med | Applied — FR-091's rationale cites the defects by repository and number |
| FND-1415 | SR-141 | med | Accepted with mitigation — the tool-defect ledger carries quire-rs#403, and tag reconciliation is derived from `quire coverage`, never grep |
| FND-1416 | SR-141 | med | Applied — see FND-1409 |
| FND-1417 | SR-141 | low | Applied — FR-084 depends on FR-092; the read-only envelope is FR-092's and holds during enumeration |
| FND-1418 | SR-141 | low | Applied — FR-085 declares FR-070 upstream |
| FND-1419 | SR-141 | low | Applied — downstream sections completed on FR-084, FR-085, FR-087, FR-088, FR-091, FR-092 |
| FND-1420 | SR-142 | high | Applied — see FND-1400 |
| FND-1421 | SR-142 | med | Applied — FR-092's Description carries one obligation |
| FND-1422 | SR-142 | med | Applied — see FND-1402 |
| FND-1423 | SR-142 | med | Applied — NFR-022's Statement is the timing obligation alone; the read-only obligation is FR-092's, with a metric row here |
| FND-1424 | SR-142 | med | Applied — see FND-1404 |
| FND-1425 | SR-142 | low | Applied — see FND-1407 |
| FND-1426 | SR-142 | low | Not applied — the engine reports no finding on these, and `Where <the classified form is X>` reads as the optional-feature qualifier it is; changing them would trade a checked form for an unchecked preference |
| FND-1427 | SR-142 | low | Not applied — the `only when` form is the obligation: it is what stops a class being assigned without ledger evidence, which is FR-089's whole purpose |
| FND-1428 | SR-142 | low | Applied — NFR-023's Statement names the measurement as its subject |
| FND-1429 | SR-142 | low | Applied in part — rationale moved out of the FR-086 and FR-087 Behavior bullets; the `so that` tails on three Descriptions are kept, because each states the property the obligation exists for |
| FND-1430 | SR-143 | high | Applied — FR-090-AC-8 requires a machine-readable index binding every printed figure to its artifact and field; NFR-023 restated as that obligation |
| FND-1431 | SR-143 | high | Applied — NFR-021 is scoped to a committed fixture corpus under `tests/fixtures/corpus-measurement/` |
| FND-1432 | SR-143 | high | Applied by rewrite — FR-088-CON-2 is now an absence-of-code constraint verified by inspection, which a hard-coded copy cannot pass |
| FND-1433 | SR-143 | med | Applied in part — NFR-022 keeps the open-mode assertion as its method; FR-092-AC-3 stays a whole-run assertion over written paths |
| FND-1434 | SR-143 | med | Deferred — recorded here as FND-1480 |
| FND-1435 | SR-143 | med | Applied — NFR-021's verification adds a shuffled-enumeration run, so order independence is observed rather than asserted |
| FND-1436 | SR-143 | med | Applied — see FND-1406 |
| FND-1437 | SR-143 | med | Applied — FR-085-AC-3 and TC-1583 assert ref, index and working-tree equality over module repositories too |
| FND-1438 | SR-143 | med | Deferred — recorded here as FND-1481 |
| FND-1439 | SR-143 | low | Applied in part — TC-1526, TC-1530, TC-1578 and TC-1584 are `Property`; the four criteria named stay example-shaped where the criterion names one input and one outcome |
| FND-1440 | SR-144 | high | Applied — FR-084 never traverses a symbolic link and records each one under the `symlink` rule |
| FND-1441 | SR-144 | high | Applied — a nested `.git` excludes its subtree under `nested-repository` and is evaluated separately |
| FND-1442 | SR-144 | high | Applied — FR-084 re-reads `HEAD` and cleanliness after the walk and records `stable`; NFR-021 excludes unstable and dirty repositories and requires them to be marked |
| FND-1443 | SR-144 | high | Applied — FR-085 records the resolved commit of every ref; FR-090's population identifier carries commits, not refs |
| FND-1444 | SR-144 | high | Applied — FR-085 compares each declared `data_schema` digest against the file read; the vacuous-pass case is moot now the engine decides conformance |
| FND-1445 | SR-144 | high | Applied by rewrite — no module-supplied pattern is evaluated by the measurement; an abnormal engine termination records the batch `could-not-run` |
| FND-1446 | SR-144 | high | Applied — the form census counts documents and states that unit; no check now mixes row and document outcomes |
| FND-1447 | SR-144 | med | Applied — FR-089 defines a line-independent finding identity and reports unmatched ledger entries |
| FND-1448 | SR-144 | med | Applied — a contested type raises a `contract-defect` finding, and every type key is `(module, type)` |
| FND-1449 | SR-144 | med | Applied — FR-084-CON-1 is restated as working-tree-and-refs invariance |
| FND-1450 | SR-145 | high | Applied — the document state is `contested`; `unknown` is only the failure class |
| FND-1451 | SR-145 | high | Applied — see FND-1400 |
| FND-1452 | SR-145 | high | Applied — FR-091 reports `could-not-run` for every check an entry blocks whatever state FR-086 assigned, `unreadable` included |
| FND-1453 | SR-145 | med | Applied — FR-088 declares `not-applicable` and produces `could-not-run`; the outcome vocabulary is stated where it is used |
| FND-1454 | SR-145 | med | Applied — FR-089 gives `malformed-document`, `missing-structure` and `stale-module` assignment rules, and TC-1577 exercises all three |
| FND-1455 | SR-145 | med | Applied — one ledger: FR-091's tool-defect entries are a section of the FR-089 classification ledger, and FR-091-AC-1 names FR-089-AC-3's refusal |
| FND-1456 | SR-145 | med | Applied — the divergence margin is a declared input (FR-090-CON-3, TC-1579) |
| FND-1457 | SR-145 | med | Applied — see FND-1405 |
| FND-1458 | SR-145 | low | Applied — see FND-1406 |
| FND-1459 | SR-145 | low | Applied in part — `depends_on` extended; `traces_to` is kept for the US edge, matching FR-051..FR-055, the requirements this set is a sibling of |
| FND-1460 | SR-146 | high | Applied by rewrite — there is no generic evaluator to disagree with a module's oracle |
| FND-1461 | SR-146 | high | Applied by rewrite — the prose parse rules are never reimplemented |
| FND-1462 | SR-146 | high | Applied in part — divergence from the catalog pin is recorded per module; bounding it is deferred as FND-1482 |
| FND-1463 | SR-146 | high | Applied by rewrite — the measurement resolves no constraint keyword or multiplicity of its own |
| FND-1464 | SR-146 | high | Applied — see FND-1447; unmatched ledger entries are a headline figure |
| FND-1465 | SR-146 | med | Applied — the exclusion vocabulary is a declared input (FR-084-CON-3, TC-1567) recorded verbatim |
| FND-1466 | SR-146 | med | Applied — see FND-1407 |
| FND-1467 | SR-146 | med | Applied — see FND-1442 |
| FND-1468 | SR-146 | med | Applied — the engine decides every measured type, and FR-090 publishes the per-module and per-type breakdowns that show what each rate covers |
| FND-1469 | SR-146 | med | Applied by rewrite — FR-087 no longer enumerates mapping kinds |
| FND-1470 | SR-147 | high | Applied by rewrite — FR-087 drives the Quire engine; FR-087-CON-2 forbids a parser, extractor or record builder of its own |
| FND-1471 | SR-147 | high | Applied by rewrite — FR-088 keeps only the form census FR-074 already makes Quoin's |
| FND-1472 | SR-147 | high | Applied by rewrite — the measurement assigns no severity; it records the severity the engine reported, and a warning is never a failure |
| FND-1473 | SR-147 | med | Applied by rewrite — see FND-1463 |
| FND-1474 | SR-147 | med | Applied — `contract-fix-this-campaign` names the owning module repository, and FR-089-CON-3 states that it amends a wrong declaration and never relaxes a constraint to lower a count |
| FND-1475 | SR-147 | med | Applied — `accepted` names the human and the promotion gate the acceptance is recorded against |
| FND-1476 | SR-147 | med | Applied in part — FR-085 checks the declared `data_schema` digest; the separate reader stays because quoin#347 means no artifact-type module can be installed at all, and the requirement says so |
| FND-1477 | SR-147 | med | Applied in part — FR-086 resolves case-sensitively; the resolution stays here for the same reason as FND-1476 |
| FND-1478 | SR-147 | low | Applied by rewrite — FR-087-CON-2 forbids all parsing, clause content included |
| FND-1479 | SR-147 | low | Applied — FR-085's toolchain record names the engine and CLI versions and source revisions and each module's `semantic_core` version; FR-090 carries them in the population identifier |
