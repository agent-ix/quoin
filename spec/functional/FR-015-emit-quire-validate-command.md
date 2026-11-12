---
id: FR-015
title: "Emit a scoped Quire validation command"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-004"
    type: "implements"
---

# FR-015: Emit a scoped Quire validation command

## Description

The `write` command SHALL emit a Quire validation command of the form
`quire validate --scope <repo_root> "spec/**/*.md"`, scoped to the resolved repo
root, shell-quoting the repo root when it contains characters that require
quoting.

## Inputs

- The resolved repo root.

## Outputs

- A ready-to-run `quire validate` command string, plus its scope and globs, in
  the authoring pack.

## Behavior

- The command SHALL scope validation to the resolved repo root.
- The command SHALL target the `spec/**/*.md` glob.
- The command SHALL render a clean repo path unquoted, and SHALL single-quote a
  repo path that contains characters requiring shell quoting (for example a
  space).

## Acceptance Criteria

| ID          | Criteria                                                                                                     | Verification                        |
| ----------- | ------------------------------------------------------------------------------------------------------------ | ----------------------------------- |
| FR-015-AC-1 | The emitted command is `quire validate --scope <repo_root> "spec/**/*.md"` with the resolved scope and globs | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-015-AC-2 | A clean repo path is rendered without surrounding quotes                                                     | Test (quoin-cli/src/write.rs) |
| FR-015-AC-3 | A repo path containing a space is single-quoted                                                              | Test (quoin-cli/src/write.rs) |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-004](../usecase/US-004-validate-changed-spec-files.md).
- **Downstream**: the calling agent runs the emitted command; the `write` command
  emits it rather than executing it. (`quoin` validates documents elsewhere
  through the linked `quire-rs` engine, not through this command.)
