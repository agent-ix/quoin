# Quoin Skills Audit

Evaluation of the spec-authoring skills quoin ships (`skills/*`) against the
upstream reference repo `agent-ix/spec-skills`, to find content lost or degraded
when the skills were ported into quoin. Done 2026-06-20.

## Design context

The per-artifact `spec-write-fr/us/nfr/str` skills were **intentionally retired**:
each requirement type's rules now live in its **template** (the skeleton + schema
that `quoin write --types <T>` returns from the catalog). So their absence is by
design, not a gap. What must NOT be lost is the **orchestration** — telling the
agent to author each requested type as a discrete artifact file, in sequence —
and the **process/reasoning** that templates can't encode (US→FR derivation,
integration-test reality rules).

## Findings

| Skill                                                                                                             | Status                                                                                                                                                                                                                                                             | Action                                                                                                                                                |
| ----------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify`                                                                                                         | **Degraded** — dropped the workflow orchestration, the "author each type as a file" rule, US→FR derivation, app-spec scoping; became a thin `quoin write` wrapper. This caused a backport to produce FRs as `spec.md` table rows instead of `FR-XXX.md` artifacts. | **Fixed** — SKILL.md rewritten to request-scoped orchestration (author StR/US/FR/NFR/IT as discrete files, stop at IT, then ask about matrix/review). |
| `spec-us-to-fr`                                                                                                   | Removed (process skill — derivation reasoning, not a template rule).                                                                                                                                                                                               | **Folded** into `specify/references/us-to-fr.md` (loaded on demand).                                                                                  |
| `spec-write-it`                                                                                                   | Removed (process skill — real-by-default IT rules, not template-encoded).                                                                                                                                                                                          | **Folded** into `specify/references/integration-tests.md`.                                                                                            |
| `spec-write-fr/us/nfr/str`                                                                                        | Removed; rules now template-encoded.                                                                                                                                                                                                                               | None — intended.                                                                                                                                      |
| `spec-blueprint`                                                                                                  | Removed (archetype FR-composition audit).                                                                                                                                                                                                                          | **Excluded** by decision.                                                                                                                             |
| `spec-object-review`                                                                                              | Carries `references/object-type-guide.md` whose `assets/*-template.md` paths point at templates that live in the catalog, not the skill.                                                                                                                           | **Fixed** — added a note that templates resolve via `quoin write` / `quoin catalog show`.                                                             |
| `spec-app-review`                                                                                                 | `artifact_type`→`type` rename only.                                                                                                                                                                                                                                | None.                                                                                                                                                 |
| `spec-dependency/evidence/failure-domain/integrity/matrix/review/risk-complexity/scope-boundary/security/to-plan` | Parity with upstream.                                                                                                                                                                                                                                              | None.                                                                                                                                                 |
| `spec-ideation`                                                                                                   | quoin-only addition (exploratory drafting gate before `specify`).                                                                                                                                                                                                  | None.                                                                                                                                                 |

## Open follow-up (not fixed here)

`skills/spec-review/`, `skills/spec-matrix/`, and `skills/spec-to-plan/` each
vendor a compiled `workflow-assets/dist/index.js` whose **flow registry still
lists the removed skills** (`spec-write-str/us/fr/nfr/it`, `spec-us-to-fr`,
`spec-blueprint`). If those `ix-flow` workflows try to invoke the missing skills
as steps, they break. The source is `ix-spec-workflows` (archived), so
regenerating the bundles is non-trivial — tracked as a separate follow-up, not
part of the `specify` fix.

## Guard

TC-EV-021..TC-EV-025 in `spec/evals.md` (the `artifacts` check in `evals/lib/assert.mjs`)
assert that spec-change requests produce the right artifact **types as discrete
files** — a regression guard for the `specify` degradation above.
