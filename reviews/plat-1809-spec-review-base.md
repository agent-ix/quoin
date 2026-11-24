---
id: SR-137
title: "Base requirements review — risk decisions PR #1048"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-142, FR-145, FR-191, FR-199, FR-213, FR-214; spec/non-functional/NFR-031"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-213", type: references }
  - { target: "ix://agent-ix/quoin/FR-214", type: references }
  - { target: "ix://agent-ix/quoin/FR-142", type: references }
---

## Summary

Reviewed the changed requirement text, criteria, links, ID formats, and criterion trace coverage. The new risk requirements are testable and their 18 new criteria have test tags; the PLAT-1861 behavior is left contradictory between FR-141's shared `--discharges` value and FR-142's unique-clause resolution rule.

## Verdict

**FAIL** — FR-141 still documents one obligation id applied to every created record, while FR-142 now refuses resolution when multiple records share that clause.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-142 requires a clause to be unique per component, but FR-141's `--discharges` input writes the same obligation id to every record created from a multi-component SBOM; this makes those components' advisories permanently unresolved until manual repair. | FR-142-AC-37; FR-141-AC-4 |

## Scope Examined

- FR-142 shared-clause behavior and FR-142-AC-37.
- FR-141 `--discharges` contract; FR-145 command shape; FR-191 gap-kind list; FR-199 dossier outputs and AC-17 through AC-20.
- FR-213 and FR-214 statements and all 13 acceptance criteria; NFR-031-AC-2.
- Matrix check confirms all 18 newly added criteria are tagged.


## Dispositions

| FND | outcome | reason | after_excerpt |
| --- | --- | --- | --- |
| FND-001 | fixed | FR-141 now refuses a multi-record promotion with --discharges before writing records; the contract no longer permits sharing this clause through promotion. | `If --discharges is given and the promotion plans to create more than one new soup_record, then the command SHALL refuse before writing any record.` |
