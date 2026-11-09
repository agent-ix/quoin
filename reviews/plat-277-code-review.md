---
id: SR-20261103118145
title: "Code review of quoin PR 784 (advisory corpus measurement report)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@03792cc8; analysis/corpus-measurement/contract-measurement-2026-11.md"
review_set: subset
---

# Code review of quoin PR 784

## Summary

Ticket: PLAT-277. Scoped to `git diff origin/main...HEAD` at 03792cc8: one new Markdown report,
`analysis/corpus-measurement/contract-measurement-2026-11.md`. Markdown-only, so no cargo gates;
`quoin validate --repo .` (quoin 0.29.1) reports no findings.

Checks run. Arithmetic: every partition table was re-summed (by-module 13,131 / 11,837; untyped
291 = sum of undeclared types; public + private 6,981 + 6,150; 39 public + 219 banded private =
258 failing repositories; primary classes 1,294; contract-defect rows 392); all agree.
Re-measurement: quire-cli origin/main (2b6cc6f, `quire 0.38.4 (engine 0.53.0)`) was built from
source through `locked-build.sh`, and the published recipe (ten `--module` dirs, `--scope` at the
repo root, `--diagnostics-format json`) was run on six documents with the ten modules at origin/main
(spec-objects-business at 0.9.0, 15f9138, matching the report's manifest version). An
`api_endpoint` FR fails on `operations` required, a `configuration` FR on `fields` required (F1),
`spec/log.md` on `unknown type 'log'` (F3), a `process` FR with a mermaid sequence diagram under
`## Workflow` on missing `steps` plus the misleading `feature-not-extractable` message (F5, F8),
and every run emits the five `DuplicateArchetype` lines and `semantic.inline-data-schema` for
`standard` without changing the exit code (F4). F4 traced to the manifest: `archetypes:` (kinds adr,
plan, review, spec-review, standard) and `artifact_types:` both declare ADR, Plan, Review,
SpecReview and Standard. F3: no module manifest on origin/main declares `log`; `index` is declared
by spec-artifacts-iso. F1: `ApiEndpoint.json` requires `operations` while the locator marks
`## Operations` `required: false`; `UiComponent.json` requires `fields` while the locator mandates
`## Props`. F2 did not reproduce as stated (FND-001).

Leak check: all 39 repositories named in the public table, plus the module and tool repositories
named elsewhere, were checked with `gh repo view` and are PUBLIC. Private repositories appear only as
`private-N` and band counts; no private repository name, path or document title appears. The one
repository holding the Standard documents is private and is correctly left unnamed.

## Verdict

**CHANGES REQUESTED (one high, two medium, two low).** The measurement is sound and honestly
partitioned: every weak partition is surfaced, failures are classified with owners, no private
names leak and the report carries no digest or manifest ceremony of its own. But finding F2, a P1
contract finding, names the wrong schema and the wrong fix, and F6 directs owners to pins that do
not exist. The PR leaves the superseded 2026-09 report and its manifest files beside the new one.
Separately from this PR's defects, PLAT-277's acceptance criterion that P0/P1 findings be resolved
or accepted by a human owner is not met by a report; the ticket stays open after merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | F2 names the wrong contract. It says `schemas/Standard.json` requires `code` and "the Standard locator never reads it", with the fix "feed `code` from frontmatter or drop it from `required`". Reproduced on a failing corpus Standard document: it fails `semantic.record-invalid ... "code" is a required property`, and the same document with only its `object: standard` line removed passes. So `Standard.json` (artifact type `Standard`) accepts it; the failing contract is `object_types: standard`, whose inline `data_schema` requires `code` while its own locator already reads `code` from frontmatter (`required: true`). The proposed cheap fix edits a schema that is not failing. The weak-partition row and the contract-defect rule row repeat the same attribution | analysis/corpus-measurement/contract-measurement-2026-11.md:295, :17, :235 |
| FND-002 | medium | F6 asks for "`quoin module ensure-defaults` pins brought to the completed modules", and the limits section lists "stale-module effects of the shipped `default-modules.yaml` pins". `default-modules.yaml` on quoin main has no pins: each entry checks out the module's default branch. The stale state is the installed CLI and already-installed module clones, not a pin; the finding sends the owner to fix something that does not exist | analysis/corpus-measurement/contract-measurement-2026-11.md:299, :332 |
| FND-003 | medium | The report states it supersedes the 2026-09 `report.md`, "produced by a measurement tool that no longer exists", but the PR keeps `report.md` and its `run-manifest.json`, `figure-index.json`, `census.json`, `rates.json`, `findings.json` and `states.json` in the same directory. A superseded report beside its replacement is two contradictory measurements, and the run manifest and figure-binding index are file-tracking ceremony. Delete them in this PR | analysis/corpus-measurement/contract-measurement-2026-11.md:3 |
| FND-004 | low | "Pass rates by composed object module" says "Documents of type `FR` may carry `object: <type>`" and the weak-partition table heads these rows "Object-typed FRs", but the `spec-artifacts-process / object: standard` row (39 documents) is `type: Standard` documents carrying `object: standard`, not FRs. The same 39 documents are also counted in the `Standard` by-type row, so the section is not FR-only and the heading misstates its population | analysis/corpus-measurement/contract-measurement-2026-11.md:47, :51, :62 |
| FND-005 | low | The recipe reproduces per-document verdicts (confirmed), but not the class partition. The diagnostic-to-class rules, the primary-class order applied in code and the repository enumeration (listing filter, shallow default-branch clone) are stated only in prose, and the raw per-document results are kept nowhere, although the ticket asks to retain them. The normalization campaign needs that work list. State where the per-document results are kept (a private location) or give the enumeration and classification commands | analysis/corpus-measurement/contract-measurement-2026-11.md:28 |

## Examined scope

```yaml
clean: false
scope:
- id: report/headline-and-weak-partitions
  path: analysis/corpus-measurement/contract-measurement-2026-11.md
  role: examined
  excerpt: "11,837 / 13,131 documents pass = 90.1 %. ... Weak partition | Documents | Pass rate | Why"
- id: report/what-was-measured
  path: analysis/corpus-measurement/contract-measurement-2026-11.md
  role: examined
  excerpt: "Reproduce. Clone the repositories, then per document: quire validate --module <each module dir> --scope <repo> --diagnostics-format json <doc>."
- id: report/pass-rates
  path: analysis/corpus-measurement/contract-measurement-2026-11.md
  role: examined
  excerpt: "Pass rates by module / by composed object module / by document type / by repository"
- id: report/failure-partition
  path: analysis/corpus-measurement/contract-measurement-2026-11.md
  role: examined
  excerpt: "1,294 failing documents, every one classified; unclassified = 0."
- id: report/findings-F1-F8
  path: analysis/corpus-measurement/contract-measurement-2026-11.md
  role: examined
  excerpt: "F1 ... F8 findings table with priority, class, module, evidence, owner"
- id: report/limits
  path: analysis/corpus-measurement/contract-measurement-2026-11.md
  role: examined
  excerpt: "What this measurement could not establish"
- id: default-modules.yaml
  path: default-modules.yaml
  role: context_only
  excerpt: "Each entry checks out one module's data subdirectory from its repository's default branch."
```
