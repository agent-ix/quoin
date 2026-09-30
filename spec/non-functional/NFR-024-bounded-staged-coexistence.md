---
id: NFR-024
title: "Staged coexistence is bounded, owned, and never reported as remediation"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-100"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-101"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-102"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-103"
    type: "constrains"
---

# NFR-024: Staged coexistence is bounded, owned, and never reported as remediation

## Statement

While a capability's replaced implementation is retained, Quoin SHALL declare
every non-retention allowance in one checked-in allowance manifest, and SHALL report a retained path as retained-with-successor rather
than as remediated or allowed.

## Scope

- Applies to: every first-party executable path in this repository retained
  during a staged cutover, the checked-in allowance manifest, and the
  enforcement run that publishes the metric.
- Operational context: each candidate revision between the first Rust crate and
  the final deletion, in continuous integration and on a maintainer's machine.
- Not applied to: paths inside the `corpus/` submodule, which belong to
  `agent-ix/qa-corpus`; and to another repository's retentions, which that
  repository declares.

## Rationale

Amendment 1 says coexistence during a staged cutover is not a policy violation.
That sentence is load-bearing in one direction and dangerous in the other: it
makes a long retention safe to have, and it makes an unbounded retention easy to
hide. A retention with no successor is indistinguishable from a decision never to
port, and a retention with no expiry never produces the pressure that closes it.

The metric depends on the distinction. Violations is a level held at zero;
retained-with-successor is a slope and is the programme's only honest measure of
velocity. Collapsing the two — by reporting a retention as allowed, or by
reporting the existence of a burn-down ticket as remediation — makes the slope
unreadable. Containment and burn-down are separate programmes for the same
reason: containment may not report a capability remediated because a burn-down
ticket exists for it.

Two facts make the definitional half of this requirement necessary rather than
pedantic. First, the delivery stage tickets 0 through 9 do not exist: the epic
carries the stages in prose only. Read strictly, a retention with no successor
ticket is a violation, so on the day the enforcement lands the whole retained
surface — the large majority of 105,814 lines — reads as violations rather than
as retentions. What counts as a valid successor reference therefore decides the
metric's opening value, and must be written down rather than inferred.

Second, the allow-set and the retention list are two artefacts, not one, and the
split has to be stated or a path can hold both at once. The named exceptions —
user interface, generated, inert, thin host dispatch, dated owner disposition —
are declarations, so they belong in a checked-in manifest. Naming each file and its required fields is what makes the generated
exception provenance-based rather than directory-name-based, and the thin-host
exception a ceiling rather than an opinion.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Retained paths reported as remediated or allowed | 0 | 0 | Test |
| Allowance classifications not backed by the allowance manifest | 0 | 0 | Test |
| First-party non-Rust executable violations | 0 | 0 | Test |

## Verification

A valid successor reference is an open issue in `agent-ix/quoin` that is a
sub-issue of the burn-down epic, names the delivery stage it discharges, and
names the path or path glob it retires. A reference to the epic itself, to a
prose stage in the epic body, to a closed issue, or to an issue in another
repository is not valid.

The allowance manifest is `.language-allowances.yaml` at the repository root.
Each entry declares its category — `ui`, `generated`, `inert`, `thin-host` or
`owner-disposition` — the path or glob it covers, and the owner recording it. A `thin-host` entry additionally declares the line
ceiling and the branch ceiling that host dispatch may not exceed. An
`owner-disposition` entry additionally declares the date and the deciding owner.
An entry missing a required field, a classification that no entry backs, and an
entry declaring a category this requirement does not admit each fail the run. The second and
third are different directions of the same check and both are needed: without
the third, a sixth category can be invented in the manifest and every path
under it becomes allowed with no requirement admitting it.
Where two path globs overlap, the more specific glob wins, and two entries of
equal specificity covering one path fail the run rather than resolving silently.

The enforcement run classifies every first-party executable path into exactly one
of violation, retained-with-successor, or allowed, and writes the three counts as
a measurement collection into the evidence store. Its population is defined by
role rather than by extension, so an executable path with no governed extension —
a Makefile recipe, a `run:` block under `.github/workflows/**` — is classified
like any other. A run that
classifies nothing is reported as inconclusive rather than as clean.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-024-AC-3 | No report produced by this programme records a retained path as remediated, and no report records the existence of a successor ticket as remediation. | Test (TC-1662) |
| NFR-024-AC-4 | Standing-approved user-interface TypeScript and `filament-core-data`-published types are emitted in the allowed class and never in the retained class. | Test (TC-1663) |
| NFR-024-AC-5 | An enforcement run that classifies an empty population is reported as inconclusive rather than as zero violations. | Test (TC-1664) |
| NFR-024-AC-6 | `.language-allowances.yaml` is checked in at the repository root, every entry declares a category, a path or glob and an owner, and an entry missing a required field fails the run. | Test (TC-1686) |
| NFR-024-AC-8 | Every `thin-host` entry declares a line ceiling and a branch ceiling, and a covered file exceeding either is reported as a violation. | Test (TC-1688) |
| NFR-024-AC-9 | A path classified as allowed with no backing entry in the manifest fails the run. | Test (TC-1689) |
| NFR-024-AC-12 | The classified population includes executable paths with no governed extension, and a planted non-Rust assertion in a Makefile recipe or a workflow `run:` block is classified rather than skipped. | Test (TC-1707) |
| NFR-024-AC-14 | A manifest entry declaring a category this requirement does not admit fails the run, naming the entry and the category. | Test (TC-1710) |

> **CR note (2026-09-30):** The clauses that required each retained path to be
> carried as a row of `docs/rust-burndown/executable-path-matrix.md` are
> withdrawn: the Statement's matrix-row clause, the Verification sentence naming
> that file as the place the stage issues are listed, and the Verification
> sentence placing staged-port retention in that file. The file was an inventory
> snapshot taken at one commit, and 27 of its rows pointed at the deleted
> `scripts/` directory. It is deleted.
>
> NFR-024-AC-1, AC-2, AC-10, AC-11 and AC-13 are withdrawn. They required
> retention rows in that matrix, which no longer exists, and no test stands
> behind them. NFR-024-AC-7 is withdrawn with the Verification sentence
> requiring a `generated` entry to declare a source-schema digest, and so is the
> Rationale sentence placing staged-port retention in the burn-down matrix.
> The Measurement rows and Verification text that only those criteria checked
> (successor and expiry per retention row, provisional references, a path with
> both a manifest entry and a retention row, expired retentions, planted
> retentions and a planted `generated` hand edit) are withdrawn with them.
> TC-1660, TC-1661, TC-1687, TC-1705, TC-1706 and TC-1708 are withdrawn with
> them. The ids are not reused.

## Dependencies

- **Upstream**: [StR-009](../stakeholder/StR-009-one-implementation-language-for-engine-logic.md) and [ADR-0003](../../docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md); quire-research LR03, which owns the shared capability-gap matrix this consumes.
- **Downstream**: [FR-100](../functional/FR-100-rust-evidence-measurement-change-assurance.md), [FR-101](../functional/FR-101-retire-replaced-executable-paths.md) and [FR-103](../functional/FR-103-corpus-consolidation.md), each of which reads the manifest and the successor definition this requirement fixes; quire-research LR08 enforcement, which owns the run and whose bypass probe is the planted-violation half of this requirement's verification.
