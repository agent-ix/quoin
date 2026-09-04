---
id: SR-145
title: "Integrity review of the quoin#291 corpus measurement requirements"
type: SpecReview
analysis: integrity
scope: "US-022, FR-084..FR-092, NFR-021..NFR-023, matrix rows TC-1500..TC-1565"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-084"
    type: "reviews"
  - target: "ix://agent-ix/quoin/NFR-021"
    type: "reviews"
---

# SR-145: Integrity review of the quoin#291 corpus measurement requirements

## Summary

Thirteen requirement-bearing documents and 66 matrix rows were reviewed — US-022,
FR-084..FR-092, NFR-021..NFR-023 and TC-1500..TC-1565, the artifacts added by commit
`2e5d704` for `agent-ix/quoin#291`. Nothing in the corpus itself was edited.

Traceability is complete in the direction the campaign needs most. Every one of the
66 acceptance criteria in the set carries exactly one matrix row, and every one of
the 66 matrix rows cites exactly one acceptance criterion: FR-084 AC-1..AC-6 to
TC-1500..TC-1505, FR-085 to TC-1506..TC-1511, FR-086 to TC-1512..TC-1517, FR-087
AC-1..AC-7 to TC-1518..TC-1524, FR-088 to TC-1525..TC-1530, FR-089 AC-1..AC-7 to
TC-1531..TC-1537, FR-090 AC-1..AC-7 to TC-1538..TC-1544, FR-091 to TC-1545..TC-1550,
FR-092 to TC-1551..TC-1556, NFR-021 to TC-1557..TC-1559, NFR-022 to TC-1560..TC-1562
and NFR-023 to TC-1563..TC-1565. No criterion is orphaned and no row is duplicated or
invented. Every FR traces to US-022, US-022 traces to StR-002 and StR-005, and all
three NFRs `constrains` a functional requirement in the set. Seventeen of the
eighteen declared constraints also carry a row.

The defects are in the vocabularies, not the trace. Four vocabularies are declared
across the set — the four document states of FR-086, the three outcomes of FR-087,
the eight failure classes of FR-089 and the four dispositions of FR-089 — and the
requirements do not agree on their membership or their boundaries. Two tokens are
defined twice with different meanings (`unknown`, `unsupported-representation`), one
token is used normatively in two requirements without ever being declared
(`not-applicable`), one declared outcome is never produced (`could-not-run` in
FR-088), and one state transition the user story explicitly demands — a document
unreadable because of a known tool defect appearing as neither pass nor fail — cannot
occur under the requirements as written. Three failure classes have no assignment
rule at all while FR-089-CON-1 asserts the eight are exhaustive.

Findings FND-1450..FND-1459 are this review. FND-1451 restates, from the vocabulary
side, the contradiction SR-142 recorded as FND-1420 from the grammar side; the two
should be resolved by one edit. SR-142 FND-1422 (the FR-085 continue / FR-092
exit-non-zero boundary) is a live consistency defect in this set and is not
re-filed here.

## Verdict

CONDITIONAL. The set is structurally complete: the criterion-to-row bijection holds
at 66/66, and no requirement lacks an owning story or a verification method.
FND-1450..FND-1452 are high and should be resolved before the set is tasked, because
each of the three decides what a published number counts — and this campaign exists
because published numbers in this programme were wrong about what they counted. The
right shape of the fix is one shared state model: a single document-level outcome
vocabulary declared in one requirement and referenced by the rest, with the failure
partition drawing its classes from that vocabulary rather than restating it.
FND-1453..FND-1457 are medium and cheap to fix while the statements are still text.
FND-1458 and FND-1459 are bookkeeping.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-1450 | high | `unknown` is defined twice with different meanings — a document state in FR-086 (a type declared by two modules) and a failure class in FR-089 (a failure the ledger does not classify) — and FR-089-AC-2 makes an "unknown count" a headline figure with nothing saying which of the two it counts. | FR-086, FR-089, FR-090 |
| FND-1451 | high | `unsupported-representation` is both a non-failure outcome in FR-088 ("SHALL report the document `unsupported-representation` rather than `fail`") and one of the eight failure classes of FR-089, whose FR-089-CON-1 declares the classes exhaustive over reported failures; either the class is unreachable or FR-088's "rather than `fail`" is wrong. | FR-088, FR-089 |
| FND-1452 | high | A document unreadable because of a known tool defect can never reach `could-not-run`: FR-086 assigns it `unreadable`, FR-087 evaluates only `measured` documents and so produces no outcome for it, yet FR-091-AC-2 requires it to report `could-not-run` and US-022-EX-2 is written entirely about this case. No requirement bridges the FR-086 state vocabulary to the FR-087 outcome vocabulary. | FR-086, FR-087, FR-091, US-022 |
| FND-1453 | medium | `not-applicable` is used normatively by FR-088's Behavior, FR-088-CON-1 and FR-090's exclusion rule but is a member of no declared vocabulary, while FR-088's Description declares `pass`/`fail`/`could-not-run` and its Behavior never states a condition that produces `could-not-run`. FR-088 also mixes row-level and document-level outcomes without a stated rollup. | FR-088, FR-090 |
| FND-1454 | medium | Three of the eight failure classes — `malformed-document`, `missing-structure` and `stale-module` — have no assignment rule in FR-089's Behavior, no producing requirement, and no acceptance criterion, while the catch-all rule sends every unledgered failure to `unknown`; as written they are unassignable and FR-089-CON-1's exhaustiveness claim is vacuous for them. | FR-089 |
| FND-1455 | medium | The same obligation is stated twice in different words: FR-089 refuses a `tool-defect` classification whose ledger entry lacks a repository and issue number (AC-3 / TC-1533) and FR-091 refuses a tool-defect ledger entry lacking a repository and issue number (AC-1 / TC-1545). Two ledgers are named — FR-089's "declared classification ledger" and FR-091's "declared tool-defect ledger" — and no requirement says whether they are one artifact. | FR-089, FR-091 |
| FND-1456 | medium | FR-090's divergence list turns on "a declared margin" that no requirement, NFR or input declares, and no requirement says who declares it; FR-090-AC-5 and TC-1542 are therefore not verifiable without an out-of-band value. | FR-090, TC-1542 |
| FND-1457 | medium | The commit added no rows to the matrix's prose coverage tables: `## Functional Requirements` still ends at FR-083, `## Non-Functional Requirements` at NFR-019, and `## Use Case Coverage` at US-021, so US-022-EX-1..EX-4 have no realising rows even though the neighbouring US-020 and US-021 entries carry theirs. (The generated `## Functional Requirement Coverage` table is correctly untouched — it is derived from tracking tags, and no test exists yet.) | matrix.md, US-022 |
| FND-1458 | low | FR-091-CON-1 is the only constraint in the set with no matrix row; its Validation is `Inspection` and no inspection row exists, whereas the other seventeen constraints each ride an acceptance-criterion row (for example FR-084-CON-1 and FR-087-CON-2 on TC-1552). | FR-091, matrix.md |
| FND-1459 | low | Frontmatter `relationships` omit upstreams the prose Dependencies name — FR-086 and FR-087 omit FR-085, FR-088 omits FR-085 and FR-086, FR-089 omits FR-088, NFR-021 lists FR-085 in prose and FR-090 in frontmatter — so the machine-readable graph and the human-readable one differ; the set also uses `traces_to` toward its story where FR-076..FR-083 use `implements`. | FR-086, FR-087, FR-088, FR-089, NFR-021 |

## Evidence

Criterion-to-row bijection was checked by enumerating every `AC-` row of the twelve
requirement tables against the `TC-1500`..`TC-1565` block of `spec/matrix.md`: 66
criteria, 66 rows, one cited criterion per row, no criterion cited twice. Constraint
coverage was checked the same way over the eighteen `CON-` rows; seventeen appear as
secondary references on an existing row and FR-091-CON-1 appears nowhere.

Vocabulary membership was collected from the four declaring statements — FR-086's
Description (`measured`, `out-of-model`, `unreadable`, `unknown`), FR-087's
Description (`pass`, `fail`, `could-not-run`), and FR-089's Description and Behavior
(eight classes, four dispositions) — and every normative use of each token across
FR-084..FR-092 and NFR-021..NFR-023 was then read against those declarations. The
tokens carrying a defect are `unknown` (two declarations), `unsupported-representation`
(two declarations), `not-applicable` (used, never declared), and `could-not-run`
(declared by FR-087, required by FR-091 for documents FR-087 never sees).

Structural validation: `quire validate --scope
/home/peter/dev/quoin/.worktrees/291-corpus-measurement
"reviews/26-09-04-spec-review-integrity-291.md"` reports this document valid.
