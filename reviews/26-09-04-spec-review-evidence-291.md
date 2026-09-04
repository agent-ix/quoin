---
id: SR-143
title: "Evidence-method review of the quoin#291 corpus measurement requirements"
type: SpecReview
analysis: evidence
scope: "FR-084-AC-1..FR-092-AC-6, NFR-021..NFR-023 including their measurement tables, TC-1500..TC-1565"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-084"
    type: "reviews"
  - target: "ix://agent-ix/quoin/NFR-021"
    type: "reviews"
---

# SR-143: Evidence-method review of the quoin#291 corpus measurement requirements

## Summary

Sixty-six acceptance criteria (FR-084..FR-092, NFR-021..NFR-023) and the sixty-six
matrix rows TC-1500..TC-1565 added by commit `2e5d704` were compared against the
deterministic catalog advice. `quoin advise --mismatch-only` reports **no mismatch**
for any criterion in the set: every criterion is authored `Test`, and every
recommendation the catalog returns for them is also of class `Test`. Zero criteria
are inconclusive. The nine rows of the three NFR measurement tables are a different
story: all nine are reported **uncatalogued**, because their `Method` cells carry
free prose rather than a catalog method identifier.

The findings below are therefore not about method *class*. They are about whether
the declared method can produce the evidence the criterion claims. Three cannot:
NFR-023's automated cross-check has no machine-readable report to read, NFR-021 and
NFR-022 draw their evidence from a population no test environment can reconstitute,
and FR-088-CON-2's only bound row is passed identically by the implementation the
constraint forbids. The matrix Type column also runs `Unit` for 46 of 66 rows while
the catalog recommends `property-based-testing (universal)` for 53 of the 66
criteria, and two evidence kinds the new rows require — `Property` and `Benchmark` —
have no suite in SR-001 that can produce them.

The ticket's central obligation — that "the check ran and passed" never merges with
"the check could not run" — is well served at the criterion level: FR-087-AC-4,
FR-090-AC-3, FR-091-AC-2 and FR-091-AC-6 each bind a row that observes the exclusion
from both sides of a rate, and FR-087-CON-1 is bound rather than orphaned. The
weakness is one level up, in whether those rows can execute against anything but the
author's own workspace.

## Verdict

CONDITIONAL. Method classes are correct throughout and need no change. FND-1430,
FND-1431 and FND-1432 name evidence that cannot be produced as declared and should
be settled before TC-1500..TC-1565 are tasked, because each of them is a route by
which a check that did not run reports as a check that passed — the exact defect
this measurement exists to avoid. FND-1433..FND-1438 are gaps in the evidence plan
rather than in the requirements. FND-1439 is advisory.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-1430 | high | NFR-023's three methods ("automated cross-check", "automated recomputation", "automated report lint") read the human-readable report, but no requirement obliges that report to carry machine-readable figure→artifact references: FR-090's outputs are the rate *records*, and NFR-023's scope is "every number in the human-readable report". As authored, TC-1563/TC-1564/TC-1565 have nothing to parse, so the provenance check can report clean without ever having examined a figure. | NFR-023, NFR-023-AC-1, NFR-023-AC-2, NFR-023-AC-3, FR-090 |
| FND-1431 | high | NFR-021 and NFR-022 draw their evidence from "the pinned corpus" — the campaign owner's workspace root — and no fixture corpus is declared anywhere in FR-084..FR-092. TC-1557, TC-1559, TC-1560, TC-1561 and TC-1562 therefore have no population any other machine can reconstitute, which makes the reproducibility NFR the one requirement in the set that cannot itself be reproduced. NFR-021's own scope says "on any machine", yet its verification is two runs on one machine. | NFR-021, NFR-021-AC-1, NFR-021-AC-3, NFR-022-AC-1, NFR-022-AC-2, NFR-022-AC-3 |
| FND-1432 | high | FR-088-CON-2 forbids the measurement from applying its own copy of the constraint vocabulary, but its only matrix row is TC-1529 — a keyword outside the closed vocabulary reports `fail` — which a hard-coded copy passes byte-identically. The constraint has a bound row and no discriminating evidence. FR-087-AC-7 already carries the pattern that works (change the module declaration, observe the evaluation change) and should be applied here. | FR-088-CON-2, TC-1529, FR-087-AC-7 |
| FND-1433 | medium | The two write-scope universals declare no observation mechanism. FR-092-AC-3 ("every file written by a run is under the declared output directory") is typed `Unit` in TC-1553, and NFR-022-AC-3 ("every corpus and module file opened read-only") is typed `Integration` in TC-1562; neither a unit nor an ordinary integration test can enumerate the opens a process performs. NFR-022's measurement table names the mechanism that would work — "syscall-level or wrapper-level open-mode assertion" — and that mechanism reaches neither acceptance criterion nor matrix row. | FR-092-AC-3, NFR-022-AC-3, TC-1553, TC-1562 |
| FND-1434 | medium | All nine NFR measurement-table `Method` cells are free prose and are reported `uncatalogued` by `quoin advise` (NFR-021-M-1..3, NFR-022-M-1..3, NFR-023-M-1..3). Existing quoin NFRs (NFR-004, NFR-005, NFR-009, NFR-010, NFR-012, NFR-016) name a catalog method in that cell, so nothing can check the new rows for conformance and the advisor's per-row recommendation degenerates to a single default. `performance-benchmarking` fits NFR-022-M-1 and M-2; the rest need a named catalog method beside their prose. | NFR-021, NFR-022, NFR-023 |
| FND-1435 | medium | NFR-021 declares two different methods for the same property. Its measurement row NFR-021-M-2 says "Inspection of the emitted ordering contract" while NFR-021-AC-2 declares `Test (TC-1558)`, typed `Property`. Neither observes what the criterion claims: no requirement obliges an enumeration whose order can be perturbed, so a property test over the natural filesystem order would pass on an implementation that depends on it. Settle on one method and require a shuffled-enumeration fixture if it is to be a test. | NFR-021-AC-2, NFR-021, TC-1558 |
| FND-1436 | medium | FR-091-CON-1 is the only obligation in the set authored `Inspection` ("a repository and issue number that a reader can open"), and it appears in no matrix row and in no inspections registry — `spec/evidence/` carries adjudications, measurements and runs but no inspections record. An Inspection produces no source symbol, so this constraint currently has no discharge path of any kind. | FR-091-CON-1 |
| FND-1437 | medium | FR-085-CON-1 forbids checking out, fetching or mutating a module repository, and is bound only to TC-1508, which asserts that content read equals the content at the declared revision. Content equality does not observe a fetch or a mutation. The before/after Git-status row that would (TC-1552) is scoped to "every enumerated corpus repository", and a module repository need not be an enumerated corpus repository. | FR-085-CON-1, TC-1508, TC-1552, FR-092-AC-2 |
| FND-1438 | medium | Two evidence kinds the new rows require have no producing suite. SR-001 registers SUITE-001 (Unit), SUITE-002 (Static) and SUITE-003 (Eval); TC-1500..TC-1565 add 10 `Property` rows and 2 `Benchmark` rows (TC-1560, TC-1561). Per the evidence-strategy rule, a method with no suite that can produce its evidence is a gap in the plan — these twelve rows cannot be discharged until a suite is registered for them. | TC-1560, TC-1561, TC-1505, TC-1516, TC-1517, TC-1531, TC-1537, TC-1538, TC-1544, TC-1558, TC-1564 |
| FND-1439 | low | The catalog recommends `property-based-testing` on the `universal` property shape for 53 of the 66 criteria, while the matrix types 46 of 66 rows `Unit`. The discipline is applied where the partition invariants live (TC-1505, TC-1516, TC-1517, TC-1531, TC-1537) and dropped where the report's honesty also depends on universality: FR-086-AC-2, FR-090-AC-3, FR-091-AC-2 and FR-091-AC-5 each quantify over every document in a state and are typed `Unit`. Judgement, not a mismatch — the class is right either way. | FR-086-AC-2, FR-090-AC-3, FR-091-AC-2, FR-091-AC-5 |

## Evidence

`quoin advise --json`, filtered to the 66 in-scope acceptance criteria, returns
`mismatch: false` and `inconclusive: false` for every one; 53 of the 66 carry a
`property-based-testing` recommendation keyed on the `universal` property shape.
The same run reports `uncatalogued: true` for all nine NFR measurement rows
(NFR-021-M-1..NFR-023-M-3), each falling back to a single
`performance-benchmarking (quantified-threshold)` recommendation. `quoin advise
--mismatch-only` lists 39 mismatches across the repository, none of them in this
set. `quoin catalog methods` supplied the method definitions and applicability
rules cited above. Matrix Type distribution of TC-1500..TC-1565: 46 Unit, 10
Property, 6 Integration, 2 Benchmark, 2 Static. Suite kinds available in
`spec/evidence/suites.md` (SR-001): Unit, Static, Eval.
