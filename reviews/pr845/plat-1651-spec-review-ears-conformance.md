---
id: SR-20261113051917
title: "EARS conformance review of quoin risk advise (FR-145..FR-148, NFR-031)"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quoin; PR #845 (git diff origin/main...HEAD at 96566e29): spec/functional/FR-145..FR-148, spec/non-functional/NFR-031, spec/functional/index.md, spec/non-functional/index.md, spec/spec.md"
---

# EARS conformance review of quoin risk advise (FR-145..FR-148, NFR-031)

## Summary

Ticket: PLAT-1651. Each SHALL statement of FR-145..148 and NFR-031 checked for EARS form
(ubiquitous, event, state, unwanted-behaviour) and for one testable obligation. `quire validate`
over the five files reports one warning: quality:ambiguous-term `acceptable` at FR-146 line 32.
Most statements are clean single-obligation EARS.

On the `acceptable` warning: fix it (FND-001). The word is the safety module's defined term, so
the output string should stay, but the statement should cite the definition so a reader does
not read "acceptable" as the tool's judgement, which is exactly what NFR-031 polices.

Examined (all role examined): FR-145..FR-148 statements and all their criteria, NFR-031
statement and NFR-031-AC-1..AC-7. Context only: spec-objects-safety FR-101..107, FR-202..205,
FR-301..305 on origin/main, quoin FR-136, FR-144, StR-007, StR-009.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | quire warns ambiguous term `acceptable`. Keep the string and say in the statement that "acceptable region" is as defined in spec-objects-safety FR-101 (regions whose outside_acceptable flag is false); if the lint still fires on the quoted string, record it as a known false positive in the Description. | FR-146:32 |
| FND-002 | low | Vacuous condition: every region list is ordered by the people who authored it, so the If clause is always true and the statement repeats line 32; delete it. | FR-148:33 |
| FND-003 | low | No object: "a link" names no link and the statement imposes no checkable obligation of its own; fold it into lines 37-38 ("... tests only that the edge exists"). | FR-147:42 |
| FND-004 | low | Reads as Markdown and JSON being the same bytes; AC-3 means each format is deterministic. Reword: "For identical inputs, each format SHALL render byte-identical output." | FR-145:46 |
| FND-005 | low | Unlike FR-146 and FR-147, which fix their Markdown strings, this wording is unspecified, so no positive test is possible and NFR-031-AC-2 cannot check it; give the exact string. | FR-148:42 |

## Verdict

Mergeable on EARS grounds after low wording fixes.
