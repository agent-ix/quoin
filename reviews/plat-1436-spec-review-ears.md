---
id: SR-207
title: "EARS conformance review of PLAT-1436 dossier spec (quoin PR #924)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quoin; PR #924 @ ab633975: requirement statements and Behavior bullets of FR-188..FR-192 and NFR-039"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-188"
    type: references
---

## Summary

Ticket: PLAT-1436. Automated check: `quire validate` reports no grammar
warnings on the seven documents. Commits 97fd398c and ab633975 fixed the earlier
ones.

Manual reading: the main statements have correct EARS shape.

- FR-189 is event-driven: "When ... selects the SRS, the SRS SHALL list ...".
- FR-190, FR-191 and FR-192 are ubiquitous statements.
- NFR-039 is four atomic ubiquitous statements.
- FR-192's conditional rules are split into atomic "If ..., the RMM SHALL ..."
  bullets.

The defects are normative rules written without a modal verb, and one compound
statement.

## Verdict

PASS with low findings. Nothing here blocks the PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Several binding rules are worded as plain statements with no SHALL, so the grammar check cannot see them and a reader cannot tell a rule from a remark. In FR-188: "`--version` is required by `generate`", "A missing file counts as `differs`", "`gap_kind` is closed" and "Gaps are ordered by ...". In FR-191: "Static-only `quire matrix` ... is not the source". FR-190's "Dependency gap" note sits in Behavior as if it were a behavior. | spec/functional/FR-188-dossier-command.md:48; spec/functional/FR-188-dossier-command.md:53; spec/functional/FR-188-dossier-command.md:59; spec/functional/FR-188-dossier-command.md:61; spec/functional/FR-191-dossier-trace-matrix-content.md:41; spec/functional/FR-190-dossier-architecture-content.md:41 |
| FND-002 | low | FR-188's Description is one SHALL covering three subcommands and three actions ("provide generate, check and export, which write, compare and export"). Each subcommand's obligation lives in Behavior, so the statement is compound. | spec/functional/FR-188-dossier-command.md:16 |
