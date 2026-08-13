---
id: FR-028
title: "Generate property tests from classified acceptance criteria"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-011"
    type: "implements"
  - target: "ix://agent-ix/quire-rs/FR-052"
    type: "traces_to"
---

# FR-028: Generate property tests from classified acceptance criteria

## Description

`quoin` SHALL supply a `spec-correctness` skill that consumes the per-criterion
property classification of [quire-rs FR-052](ix://agent-ix/quire-rs/FR-052) and
emits property tests in the target repository's own test framework.

The skill SHALL key every emitted test on the classification's `row_id`, because
`row_id` is the only identifier shared by the classification, the Test Matrix, and
the coverage reconciliation of the `gap-analysis` skill.

The skill SHALL own the mapping from property shape to test harness, because
quire-rs is forbidden to name a framework
([quire-rs FR-052-CON-2](ix://agent-ix/quire-rs/FR-052)).

The skill SHALL own the criteria the deterministic classifier could not settle,
because that classifier's recall is a measured ceiling rather than a defect
([quire-rs FR-052](ix://agent-ix/quire-rs/FR-052)); it SHALL mark them as
requiring review rather than settle them through a further deterministic pass.
The review happens where review already happens — in the pull request the
generated tests arrive in — and what the skill owes that review is a **record of
what it could not ground**, as a validated artifact.

The skill SHALL NOT derive a verdict, a grade, or a threshold from the
classification, and SHALL NOT propose rewording a criterion
([quire-rs FR-052-CON-1](ix://agent-ix/quire-rs/FR-052)).

## Inputs

- The JSON records of `quire properties --scope <repo> --json 'spec/**/*.md'`,
  one per binding acceptance criterion, carrying `row_id`, `statement`, `line`,
  `shape`, `property`, `extractable`, `extraction`, the optional `domain`,
  `precondition` and `oracle` spans, and a `signals` audit trail.
- The specification file each record cites, for the sections that state the
  criterion's domain, precondition and oracle.
- The target repository's source and existing tests, for the symbol under test and
  the repository's established tracking-tag form.

## Outputs

- Property test files in the repository's test tree, each carrying its criterion's
  `row_id`.
- One `SpecReview` artifact at `reviews/YY-MM-DD-<slug>.md` with
  `analysis: spec-correctness`, whose findings are the criteria the run could not
  ground and why.
- A census of the classification reported as counts.

Nothing else is written to the tree.

## Behavior

- The skill SHALL route a record by its `extraction` value: `extractable` is
  emitted unattended; `candidate` is emitted **and recorded as a finding** in the
  review artifact; `not-extractable` is recorded as a finding and yields no test.
- The skill SHALL select a test strategy by the record's `property` value.
- The skill SHALL NOT select a test strategy by the record's `shape` value, because
  `shape` is the grammar axis of
  [quire-rs FR-047](ix://agent-ix/quire-rs/FR-047) rather than the property axis.
- The skill SHALL treat a record whose `property` is `example` or `unclassified` as
  owned by its review-gated second pass.
- The skill SHALL NOT emit an unattended test for a record whose `property` is
  `example` or `unclassified`.
- Every emitted test SHALL carry its `row_id` in a form the coverage reconciliation
  of the `gap-analysis` skill already parses, so a generated test is reconciled by
  the same grep as a hand-written one.
- The skill SHALL NOT emit a `row_id` absent from the classification output, so a
  tracking tag can never name a criterion that does not exist.
- A test the skill emits SHALL run in the repository's runner like any other test.
  A generated test arrives in a pull request, which is what places it under review;
  a test disabled in the tree is a dead test, and a Test Matrix row it would satisfy
  is the `spec-matrix` skill's status column to set, not this skill's to pre-empt.
- The review artifact SHALL name, per finding, the criterion and the reason it
  could not be settled, so the reviewer reads why rather than re-deriving it.
- The skill SHALL derive a criterion's generator domain, precondition and oracle
  from the specification and the code where the classification's spans are absent,
  because those spans reach only a small minority of criteria.
- Where the domain, the precondition, or the oracle cannot be cited to a location
  in the specification or the code, the skill SHALL record the reason.
- Where the domain, the precondition, or the oracle cannot be cited to a location
  in the specification or the code, the skill SHALL NOT emit a test, so no test
  asserts something trivially true.
- The skill SHALL adopt the harness already present in the target repository.
- The skill SHALL NOT install a generator library.
- Where a manifest names no generator library, the skill SHALL emit no test for the
  affected criteria, record each as a finding naming the missing library, and report
  the remedy.
- The skill SHALL NOT write a framework, harness, or generator library name into
  any specification artifact.

## Rationale

The classification is deliberately inert: it carries no severity, no check id, and
no framework, so it can be consumed without constraining the consumer. That leaves
three gaps a consumer has to close, and closing them here rather than in the engine
keeps the engine deterministic. The shape-to-harness mapping belongs where the
repository's harness is visible. The recall residue belongs where a proposal can be
reviewed before it lands, which is a tradeoff the engine cannot make. And the
grounding of clauses belongs where the specification and the code can both be read,
which the engine, reading one statement at a time, cannot do.

The review gate is what makes the residue safe to work on at all. A proposal that
cannot turn a coverage row green until a person accepts it costs nothing when it is
wrong, so the second pass can afford recall the deterministic pass cannot.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-028-CON-1 | The skill SHALL NOT invent an output format. Everything it writes to the tree is either a test in the repository's own harness or an artifact type the module catalog declares; a review is a `SpecReview`. | Architecture | Eval (EV-053) |

## Acceptance Criteria

| ID           | Criteria                                                                                                               | Verification    |
| ------------ | ---------------------------------------------------------------------------------------------------------------------- | --------------- |
| FR-028-AC-1  | A record with `extraction` of `extractable` yields a test emitted unattended                                            | Eval (EV-050)   |
| FR-028-AC-2  | A record with `extraction` of `candidate` yields a test and a finding in the review artifact naming it as requiring review | Eval (EV-051)   |
| FR-028-AC-3  | Any record whose `property` is `example` or `unclassified` yields no unattended test                                    | Eval (EV-050)   |
| FR-028-AC-4  | Every emitted tracking tag names a `row_id` present in the classification output                                        | Eval (EV-050)   |
| FR-028-AC-5  | Every emitted tracking tag is matched by the coverage reconciliation grep of the `gap-analysis` skill                    | Eval (EV-052)   |
| FR-028-AC-6  | Every test the skill emits runs in the repository's runner — no skip, ignore, or disabled marker is written             | Eval (EV-051)   |
| FR-028-AC-7  | The run writes exactly one `SpecReview` at `reviews/YY-MM-DD-<slug>.md` with `analysis: spec-correctness`, and it passes `quire validate` | Eval (EV-051)   |
| FR-028-AC-8  | A criterion whose domain, precondition, or oracle cannot be cited yields a recorded reason and no test                  | Eval (EV-053)   |
| FR-028-AC-9  | The census output carries no threshold, no verdict, and no rewording suggestion                                         | Eval (EV-053)   |
| FR-028-AC-10 | No specification artifact the skill writes names a test framework, harness, or generator library                        | Inspection      |
| FR-028-AC-11 | A repository whose manifest names no generator library yields findings naming it and a reported remedy, not an install and not a test | Eval (EV-053)   |
| FR-028-AC-12 | Strategy selection reads `property` and is unchanged by any `shape` value                                               | Inspection      |

> **CR-001 note (2026-08-13):** This requirement shipped in
> `@agent-ix/quoin@0.12.0`/`0.12.1` with a **review queue** the requirement never
> asked for. [#46](https://github.com/agent-ix/quire-rs/issues/46) said, in full:
> *"`extraction: candidate` → generate, and mark the test as requiring review."*
> What shipped was `tests/props/QUEUE.md` — a report committed into a test tree,
> validated by nothing and read by nothing — plus skipped test files parked in
> `_review/` directories, an acceptance procedure for un-skipping them, and the
> name "queue", which appears nowhere else in this ecosystem.
>
> The root cause was a closed enum. `SpecReview.analysis` had no
> `spec-correctness` value, so the run's output could not be stored as a validated
> artifact. That was the system reporting a **missing artifact type**; it was read
> as an obstacle and routed around. The enum is fixed in
> agent-ix/spec-artifacts-process#11, which blocked this change.
>
> "Mark it for review" already had an answer: the pull request. A generated test in
> a PR is under review by definition, and the queue was a second, worse review
> mechanism built inside the repo. What the PR does *not* carry is why a criterion
> was left ungrounded — so that, and only that, becomes the artifact.
>
> AC-2, AC-6, AC-7 and AC-11 are rewritten rather than struck: the behaviours they
> described are withdrawn, not corrected. FR-028-CON-1 and AC-13 are new, and exist
> so the same failure cannot recur under a different filename.

## Dependencies

- **Upstream**: [StR-004](../stakeholder/StR-004-governed-workflows.md);
  [US-011](../usecase/US-011-generate-property-tests-from-criteria.md). Consumes
  the classification of [quire-rs FR-052](ix://agent-ix/quire-rs/FR-052) as
  exposed by `quire properties --json`.
- **Downstream**: supplies `Property` rows to the Test Matrix built by the
  `spec-matrix` skill, and tracking tags to the coverage reconciliation of the
  `gap-analysis` skill.
