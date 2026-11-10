---
id: SR-20261103118161
title: spec review of the PLAT-194 spec and matrix edits
type: SpecReview
analysis: base
scope: agent-ix/quoin; spec/functional/FR-077-generated-schema-emission.md; spec/matrix.md
review_set: subset
date: '2026-11-10'
---

# spec review of the PLAT-194 spec and matrix edits

## Summary

Ticket: PLAT-194 (agent-ix/quoin PR #795, head 6501ffdc).

This review covers the FR-077 Inputs, Behavior, AC-5 and AC-8 edits, the FR-077, FR-083 and NFR-020 summary rows, and the TC-1414..1416, 1431, 1448, 1449, 1457 and 1465 rows. The integrity and EARS sub-checks were applied inline to the edited statements.

## Scope

- `FR-077-AC-5` (examined): "An `@jsonSchema` base that disagrees with the package identity fails the emit command naming both values, and no committed file changes."
- `FR-077-AC-8` (examined): "In a mixed rendering with an imported module, a `$ref` to an imported model is written against that imported module's base; a `$ref` matching no declared base fails the emit command naming the reference."
- `FR-077#inputs` (examined): "The package identity (`<org>/<repo>`)"
- `matrix#FR-077` (examined): the summary row.
- `matrix#FR-083` (examined), `matrix#NFR-020` (examined): the summary rows.
- `matrix#TC-1431` (examined): left at 🚧, with the validation leg driven.
- `matrix#TC-1428`, `matrix#TC-1429`, `matrix#TC-1450`, `matrix#TC-1460` (context_only): left at 🚧, untouched.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | This PR rewrote the FR-077 summary row and kept "TC-1408, TC-1409, TC-1413 pass under `make template-gate`". The root Makefile has no `template-gate` target, and no Rust test is traced to FR-077-AC-1..3. The row also leaves out TC-1457, which this PR flipped. The row should name only what runs: TC-1414..1416, 1449 and 1457 on `make test`, with AC-1..3 unbacked. | spec/matrix.md:111 |

## Verdict

The FR-077 edits match the post-#793 driver. `moduleBase` checks the declared base against `https://schemas.agent-ix.org/<identity>/`, where the identity comes from the `package.json` name. The base carries no version. `importedBases` keys each imported base by identity alone. "Manifest `version`" is therefore correctly replaced by package identity in Inputs, the Behavior bullet and AC-5. AC-8 no longer claims a versioned import base. No stale `version` wording is left in FR-077.

The TC-1415 and TC-1457 titles were restated to match. The 🚧 rows are honest:

- TC-1431 names the leg that is driven and the two legs that are not.
- TC-1428, TC-1429, TC-1450 and TC-1460 are unchanged and still need a Python environment.

FND-001 is a false statement in a row this PR authored. Fix it in this PR.
