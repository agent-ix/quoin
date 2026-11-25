---
id: FRR-001
title: Quoin interaction and advisory feature risk rating
type: FeatureRiskRating
scope: supplier
rating: not_high_process_risk
status: proposed
relationships:
  - target: ix://agent-ix/quoin/IU-001
    type: uses_intended_use
  - target: ix://agent-ix/quoin/FR-003
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-011
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-145
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-146
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-147
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-148
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-189
    type: rates_feature
  - target: ix://agent-ix/quoin/FR-221
    type: rates_feature
---
# Quoin interaction and advisory feature rating

## Rationale

This is a proposed supplier rating for the stated intended use, not an accepted
decision. Each feature meets the not-high threshold because its failure can
mislead or inconvenience a person, while the feature itself neither creates
authoritative verification evidence nor makes an acceptance decision:

Classification rule: rate a feature high-process-risk if its failure can make
an output or persisted record falsely claim that something is verified, traced,
discharged, or approved. Keep a feature in this grouped not-high rating only
when its failure affects presentation, interaction, or advice and the feature
cannot itself make or persist one of those claims. A consequence requiring a
person's separate judgment remains that person's decision.

- FR-003 can misdescribe a command option, but help text does not execute the
  command or create a record.
- FR-011 can misdescribe a catalog type, but a listing does not write a record
  or accept a decision.
- FR-145, FR-146, FR-147, and FR-148 can omit or misstate read-only advice,
  policy estimates, evidence gaps, or undecided items; each leaves the
  consequential decision to a person and writes no accepted decision.
- FR-189 can produce an incomplete formalization draft, but that draft remains
  unapproved and is not a verification result.
- FR-221 can misstate the repository's private reporting route, but it does not
  itself disclose a report or authorize public disclosure.

In every case, the failure affects information used by a person; it does not
silently claim that verification, traceability, discharge, or approval has
occurred. A person remains responsible for reviewing the information and
making any consequential decision.

## Assurance activities

| Activity | Acceptance criterion |
|---|---|
| Check command help against its registered command and option definitions. | FR-003-AC-1 |
| Compare catalog list and show output against loaded type declarations. | FR-011-AC-1 |
| Exercise the advisory output with representative policy and evidence inputs. | FR-145-AC-1; FR-146-AC-1; FR-147-AC-1; FR-148-AC-1 |
| Inspect a completed formalization workflow fixture and verify that it remains a draft until separately approved. | FR-189-AC-1 |
| Inspect the repository security policy for the private report route and prohibited public disclosure. | FR-221-AC-1 |
