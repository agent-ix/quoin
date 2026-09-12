---
id: NFR-024
title: "Staged coexistence is bounded, owned, and never reported as remediation"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-101"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-102"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-103"
    type: "constrains"
---

# NFR-024: Staged coexistence is bounded, owned, and never reported as remediation

## Statement

While a capability's replaced implementation is retained, Quoin SHALL carry that
path as an open matrix row naming a valid successor reference and an expiry
date, SHALL declare every non-retention allowance in one checked-in allowance
manifest, and SHALL report a retained path as retained-with-successor rather
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

Second, the allowance manifest the policy requires does not exist. Allowances 1
through 4 are supposed to be declared in a checked-in file and nothing declares
them, so the classifier has no allow-set to read and every user-interface and
generated file is a false positive waiting to happen. Naming the file and its
required fields is what makes allowance 2 provenance-based rather than
directory-name-based, and allowance 4 a ceiling rather than an opinion.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Retained paths naming a valid successor reference | all | all | Test |
| Retained paths with a recorded expiry date | all | all | Test |
| Retained paths reported as remediated or allowed | 0 | 0 | Test |
| Expired retentions with no re-dated owner decision | 0 | 0 | Test |
| Allowance classifications not backed by the allowance manifest | 0 | 0 | Test |
| First-party non-Rust executable violations | 0 | 0 | Test |

## Verification

A valid successor reference is an open issue in `agent-ix/quoin` that is a
sub-issue of the burn-down epic, names the delivery stage it discharges, and
names the path or path glob it retires. A reference to the epic itself, to a
prose stage in the epic body, to a closed issue, or to an issue in another
repository is not valid, and the enforcement run fails the row that carries it.

The allowance manifest is `quoin/.language-allowances.yaml`, checked in at the
repository root. Each entry declares the allowance number (1 through 4), the path
or glob it covers, and the owner recording it. An allowance-2 entry additionally
declares the generator identity and the source-schema digest, so generated status
is decided by provenance and a hand edit loses the allowance. An allowance-4
entry additionally declares the line ceiling and the branch ceiling that thin
host dispatch may not exceed. An entry missing a required field, and a
classification that no entry backs, each fail the run.

The enforcement run classifies every first-party executable path into exactly one
of violation, retained-with-successor, or allowed, and writes the three counts as
a measurement collection into the evidence store. It asserts that every retained
row names a valid successor reference and an expiry date, that no retained row is
emitted in the allowed class, and that a retention whose expiry has passed
without a re-dated owner decision fails. A planted retention with no successor,
a planted retention whose successor is the epic itself, a planted retention with
a passed expiry, and a planted hand edit of an allowance-2 file must each fail
the run. A run that classifies nothing is reported as inconclusive rather than as
clean.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| NFR-024-AC-1 | Every retained path is emitted in the retained-with-successor class naming an open sub-issue of the burn-down epic that names its delivery stage and the path it retires, plus an expiry date. | Test (TC-1660) |
| NFR-024-AC-2 | A planted retention with no successor, one whose successor is the epic itself or a closed issue, and one whose expiry has passed with no re-dated owner decision each fail the enforcement run. | Test (TC-1661) |
| NFR-024-AC-3 | No report produced by this programme records a retained path as remediated, and no report records the existence of a successor ticket as remediation. | Test (TC-1662) |
| NFR-024-AC-4 | Standing-approved user-interface TypeScript and `filament-core-data`-published types are emitted in the allowed class and never in the retained class. | Test (TC-1663) |
| NFR-024-AC-5 | An enforcement run that classifies an empty population is reported as inconclusive rather than as zero violations. | Test (TC-1664) |
| NFR-024-AC-6 | `.language-allowances.yaml` exists at the repository root, every entry declares an allowance number, a path or glob and an owner, and an entry missing a required field fails the run. | Test (TC-1686) |
| NFR-024-AC-7 | Every allowance-2 entry declares a generator identity and a source-schema digest, and a hand edit of a covered file loses the allowance and is reported as a violation. | Test (TC-1687) |
| NFR-024-AC-8 | Every allowance-4 entry declares a line ceiling and a branch ceiling, and a covered file exceeding either is reported as a violation. | Test (TC-1688) |
| NFR-024-AC-9 | A path classified as allowed with no backing entry in the manifest fails the run. | Test (TC-1689) |

## Dependencies

- **Upstream**: [FR-101](../functional/FR-101-retire-replaced-executable-paths.md), which creates each retention; quire-research LR03, which owns the shared capability-gap matrix this consumes.
- **Downstream**: quire-research LR08 enforcement, which emits the metric as a byproduct of the check it already performs, and whose bypass probe is the planted-violation half of this requirement's verification.
