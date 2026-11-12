---
id: FR-138
title: "Never publish an installed pack"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-137"
    type: "depends_on"
  - target: "ix://agent-ix/quire-rs/spec/functional/FR-083"
    type: "references"
  - target: "ix://agent-ix/quire-cli/spec/functional/FR-029"
    type: "references"
---

# FR-138: Never publish an installed pack

## Description

Any quoin operation that would carry an installed module's files out of the
installed set to a published destination SHALL first obtain the publish-check
decision for each module it includes, and SHALL refuse when that decision names a
restricted clause set. A **publish path** is such an operation: one that packages,
uploads, or lists a module for distribution. The decision is the engine's
`check_publishable` (quire-rs FR-083), the same one `quire publish-check`
(quire-cli FR-029) renders; quoin SHALL NOT restate or re-implement the rights
rule.

## Rationale

The rule lives in one place, the engine. If quoin carried its own copy of
"licensed means restricted", the two could diverge and a pack could pass one gate
and fail the other. Routing every publish path through one guard function means a
new publish path cannot omit the check without it being visible in review.

## Behavior

- Quoin SHALL expose one guard in the module crate that takes a set of installed module names and returns the engine's restricted sets per module, and every publish path SHALL call it before reading or writing module content.
- A publish path that includes a pack SHALL refuse with a non-zero exit and name each restricted set as `authority/id@version`.
- A publish path that refuses SHALL produce no output artifact.
- A refusal SHALL NOT contain clause text.
- `quoin module ensure-defaults` SHALL refuse a default entry whose module is a pack, naming it.
- The guard SHALL read the installed module on each call.
- Quoin SHALL contain no comparison against the `licensed` rights value or the `rights` field; the decision is only the engine's result.
- A publish path that includes only non-pack modules SHALL proceed unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-138-AC-1 | With a synthetic pack and a synthetic non-pack installed, the guard given both names returns exactly the pack's restricted sets as `authority/id@version`, ordered as the engine orders them, and nothing for the non-pack. | Test |
| FR-138-AC-2 | A publish path invoked over a set including the pack exits non-zero, names the restricted sets, and creates no output file or directory. | Test |
| FR-138-AC-3 | The same publish path over only the non-pack succeeds and produces its output. | Test |
| FR-138-AC-4 | A `default-modules.yaml` naming the pack makes `ensure-defaults` exit non-zero naming the pack, and the registry is unchanged. | Test |
| FR-138-AC-5 | Neither stdout nor stderr of any refusal contains the fixture's clause-text sentinel. | Test |
| FR-138-AC-6 | A static check over quoin source outside the engine dependency finds none of the tokens `licensed` or `clearance_ref`, and the guard's result equals `check_publishable` on the same loaded module for a hand-built fixture. | Test, Inspection |
| FR-138-AC-7 | After editing the installed pack's clause set to `text: original`, the guard returns no restricted sets with no reinstall; after restoring `licensed` it returns them again. | Test |
| FR-138-AC-8 | Every function in quoin that writes module content outside the `IX_HOME` modules directory appears in a checked list of publish paths and calls the guard; adding a writer that is not in the list fails the test. | Test |

## Constraints

Fixtures SHALL be original synthetic content with `example.invalid` authorities.

## Dependencies

- **Upstream**: [FR-137](./FR-137-detect-and-place-licensed-packs.md), quire-rs FR-083, quire-cli FR-029
