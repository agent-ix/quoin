---
id: SR-114
title: "Code review — producer-facing change assurance CLI surface"
type: SpecReview
analysis: code-review
scope: "src/commands/change-assurance/, tests/change-assurance-command.test.ts, tests/fixtures/change-assurance-cli/, spec/functional/FR-068-change-assurance-cli-surface.md, spec/matrix.md, vite.config.ts, .prettierignore"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/issues/322"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-068"
    type: "references"
---

# SR-114: Code review — producer-facing change assurance CLI surface

## Summary

Reviews the FR-068 `quoin change-assurance` surface added for #322: seven
commands over the already-implemented FR-063/FR-064/FR-065 contracts, plus
TC-1317..TC-1327, canonical goldens, and the vite entries oclif needs. The
surface transcribes and verifies; it runs nothing.

## Verdict

**CONDITIONAL** — no high findings in the change. One medium finding records a
pre-existing suite failure the change does not cause and does not fix, and one
low finding records an environment defect in `make lint`.

## Gates

Run on `feat/322-change-assurance-cli`:

- `tsc --noEmit` — clean.
- `eslint src tests` — clean.
- `prettier --check .` — clean.
- `vitest run` — 918 of 919 pass; the single failure is FND-001 below and is
  reproduced on a stashed, unmodified tree.
- `vite build` + `scripts/copy-quire-schemas.mjs`, then the built
  `bin/quoin.js` driven end to end: `seal-record` → `seal-attestation` →
  `intake` → `receipt` produced a `valid` receipt and exit 0, and
  `change-assurance schema` and `recover` behaved as specified. The commands
  were exercised through the built binary, not only through direct class
  imports, because oclif discovery of `dist/commands` is exactly what the vite
  entry list controls.

`make lint` and `make test` could not be used; see FND-002.

## Findings

| ID      | Severity | Summary                                                                        | Refs                             |
| ------- | -------- | ------------------------------------------------------------------------------ | -------------------------------- |
| FND-001 | medium   | `tests/quire-contract.test.ts` TC-118 fails against the installed quire 0.31.0 | tests/quire-contract.test.ts:541 |
| FND-002 | low      | `make lint` wedges in a nested pnpm install and never completes                | Makefile:233                     |

## Finding detail

### FND-001 — pinned quire contract rejects the installed engine's payload

`TC-118` runs the installed `quire coverage --json` over a temporary fixture
repository and validates the payload against the contract pinned in
`src/quire/contract.ts`. It fails: the payload carries an `unbacked_rows` entry
whose shape the pinned contract refuses.

Failure scenario: any consumer command that parses coverage through
`parseCoverage` refuses a real payload from `quire 0.31.0 (engine 0.46.0)`, so
the failure is not confined to the test.

Not caused here and not fixed here: the failure reproduces with this change
stashed, on an otherwise clean tree, and #322 touches neither `src/quire/` nor
the engine pin. It needs its own ticket — this is the "installed CLI drifts
from the pinned contract" class, not a #322 regression.

### FND-002 — `make lint` cannot run in this environment

`make lint` runs `corepack pnpm run lint`, which triggers pnpm's
deps-status check, which re-invokes `pnpm install` and fails with
`This project is configured to use 11.20.0 of pnpm. Your current pnpm is
v11.1.3`. Invoked directly, `pnpm run lint` reaches the same nested install and
then sits at 0% CPU with no child process until killed.

Failure scenario: the documented lint gate never returns, so a contributor
either waits indefinitely or concludes the gate passed.

Worked around, not fixed: the three stages were run directly (`tsc --noEmit`,
`eslint`, `prettier --check .`) and all pass. The `packageManager` /
`devEngines.packageManager` disagreement behind it is a repository
configuration question, out of scope for #322.

## Boundary and seam notes

- CON-1 is enforced by TC-1327 over the command sources rather than asserted in
  prose: no `child_process`, `execSync`, `spawnSync`, `spawn(`, `fetch(`,
  `simple-git`, or `https://` appears in any of them.
- CON-3 is enforced by removing the one permitted disclaimer sentence from each
  source and then requiring that no claim verb survives. A filter that merely
  skipped lines matching the disclaimer would have spared any line that happened
  to contain it; deleting the sentence first means every remaining occurrence is
  a real claim.
- `refuseSuppliedFields` exists because `sealChangeRecord` and `sealAttestation`
  silently `delete` a supplied `digest` before hashing. Without the check a
  caller could hand in a wrong digest, receive a cleanly sealed record, and be
  told nothing. The refusal is asserted in TC-1317 and TC-1318.
- The goldens are prettier-ignored with a stated reason. Formatting them would
  make the formatter a second serialization authority and turn the byte-identity
  claim into a tautology — the same reasoning already recorded for the vendored
  quire schemas and the captured tool output.
- `receipt` and `verify-receipt` exit 1 on a non-valid outcome while still
  emitting the receipt. An incomplete receipt is neither a pass nor a tool
  failure, and the payload keeps `unavailable` and `not_computed` distinct; the
  exit code only says "not discharged".
- `recover` is included although the deliverable list does not name it. Intake
  documents that an interrupted run leaves staging "for explicit recovery", and
  without a command there is no supported way for a CLI user to perform it.
