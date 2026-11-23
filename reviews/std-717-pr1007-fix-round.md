# PR #1007 fix round 1: STD-717 review disposition

PR: #1007 (`agent-ix/quoin`)
Tracker: STD-717

This fix round applies the independent review comments SR-2026112016223475846354986 through SR-2026112016223475846354993. The exact TC-006 corpus remains an upstream prerequisite. This record does not assert a live run or completed assessment.

| Review | Outcome |
|---|---|
| SR-2026112016223475846354986 | Fixed: FR-184 binds the accepted workflow repository/path/ref/source commit, dispatch principal, Quoin checkout revision, and observed analyzer build revision. Mismatches fail closed. |
| SR-2026112016223475846354987 | Fixed: removed artifact and report digest requirements. The collector supplies authenticated artifact/run/job/invocation identity and the existing FR-151 projection. |
| SR-2026112016223475846354988 | Clean review; no change required. |
| SR-2026112016223475846354989 | Fixed: the default unevaluated section now names three premises. |
| SR-2026112016223475846354990 | Fixed: workflow source ref/commit and owner-selected dispatch principal are required in the accepted FR-093 context. |
| SR-2026112016223475846354991 | Clean review; no change required. |
| SR-2026112016223475846354992 | Fixed: source identity, authorized dispatcher, and actual analyzer build revision are compared to the accepted context. |
| SR-2026112016223475846354993 | Fixed: the GitHub runner is an artifact publisher; a host-side collector retrieves authenticated API/artifact observations and passes a host-private typed observation to the FR-116 qualifier, which checks collector identity and rejects raw workflow data. |

The owner-authorized job remains `workflow_dispatch` only. The specification adds no automatic trigger, no fixed environment or tool version, and no live workflow run.

## Checks

- Quire validation passed for the changed FR-184 artifact.
- The full-tree content-rights check returned `withheld` (2,411 entries) on existing repository findings. The changed FR-184 file has three flagged external links on untouched pre-existing lines; the new review artifact and changed lines introduced no rights findings.
- No tests or live workflows were run.
