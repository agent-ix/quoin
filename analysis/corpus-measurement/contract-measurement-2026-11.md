# Corpus measurement against the completed module contracts

**Advisory. Report only.** No corpus repository was edited, no validator was weakened, and nothing was normalized. Findings feed the later corpus normalization campaign that PLAT-277 (Quoin#291) names; that campaign is not yet a ticket and this report does not create one. This report supersedes the 2026-09 measurement (`report.md` in this directory), which was produced by a measurement tool that no longer exists.

Tools run, stated once: `quire 0.38.4 (engine 0.53.0)`, built from the current `quire-cli` main; the ten default modules from their current `main`, at manifest versions engineering-assurance 0.8.0, spec-artifacts-app 0.2.0, spec-artifacts-iso 0.3.0, spec-artifacts-process 0.2.1, spec-objects-architecture 0.5.0, spec-objects-business 0.9.0, spec-objects-enterprise 0.2.0, spec-objects-operational 0.3.0, spec-objects-safety 0.3.0, spec-objects-security 0.2.0. The modules require `quire ^0.52` and that engine was built from source because the internal PyPI is not reachable from this machine (HTTP 401); nothing in this run used a private registry.

## Headline

**11,837 / 13,131 documents pass = 90.1 %.** 1,294 fail. Every document received a verdict: exit 0 or exit 3 only, no timeouts, no crashes, no document in an unavailable or unknown state.

That figure is not the result. It is dominated by the 9,863 documents typed by `spec-artifacts-iso` (94.1 %), and it hides partitions that fail almost entirely:

| Weak partition | Documents | Pass rate | Why |
|---|---|---|---|
| Object-typed FRs, `spec-objects-architecture` | 292 | 3.4 % | data schema requires a member the module's own locator treats as optional (F1) |
| Object-typed FRs, `spec-objects-operational` | 52 | 5.8 % | same (F1) |
| `Standard` (`spec-artifacts-process`) | 41 | 4.9 % | schema requires `code`, which the locator never reads (F2) |
| `TestMatrix` (`spec-artifacts-process`) | 250 | 22.8 % | `Coverage Status` header against the accepted `Status` contract |
| Object-typed FRs, `spec-objects-business` | 452 | 59.1 % | `process` FRs carry Workflow diagrams where a Step table is required (F8), plus the F1 shape |
| Documents typed with a value no module declares | 291 | 0.0 % | `type: log` (228) and legacy spellings |
| Documents with no `type` | 162 | 0.0 % | no frontmatter `type` |

## What was measured

- **Population.** `spec/**/*.md` on the default branch of every non-archived `agent-ix` repository that has a `spec/` directory, read from fresh shallow clones on 2026-11-09. 328 repositories were listed; 283 contain at least one Markdown document under `spec/` (58 public, 225 private); 45 contain none and are unavailable as a population, not passes. `quire-research` was excluded by policy. Documents outside `spec/` (`plan/`, `reviews/`, `docs/`) are outside this population.
- **Documents:** 13,131. Document `type` frontmatter is the only routing; a document with no `type` is validated as such and fails, it is not skipped.
- **Method.** One `quire validate` per document with the ten module directories passed as `--module` (a closed module set; `IX_FILAMENT_MODULES_PATH` and the default install root are not consulted), `--scope` at the repository root, diagnostics as JSON. Pass means exit 0. This evaluates each module's frontmatter schema, locator assertions and the semantic data schemas, which are the completed contracts. Advisory grammar warnings never fail a document and are reported separately below.
- **Reproduce.** Clone the repositories, then per document: `quire validate --module <each module dir> --scope <repo> --diagnostics-format json <doc>`. The per-document results are not committed: they name files in private repositories. Pass and fail counts here are the counts of that run.

## Pass rates by module (module owning the document `type`)

| Partition | Documents | Pass | Fail | Pass rate |
|---|---|---|---|---|
| (type declared by no module) | 291 | 0 | 291 | 0.0 % |
| (no type declared) | 162 | 0 | 162 | 0.0 % |
| engineering-assurance | 115 | 99 | 16 | 86.1 % |
| spec-artifacts-process | 2641 | 2399 | 242 | 90.8 % |
| spec-artifacts-iso | 9863 | 9282 | 581 | 94.1 % |
| spec-objects-architecture | 45 | 43 | 2 | 95.6 % |
| spec-objects-business | 13 | 13 | 0 | 100.0 % |
| spec-artifacts-app | 1 | 1 | 0 | 100.0 % |

`spec-objects-security`, `spec-objects-enterprise` and `spec-objects-safety` own no `type` that any corpus document declares, so they have no measured population: unknown, not passing. `spec-artifacts-app` has 1 document, too few to read as a rate.

## Pass rates by composed object module

Documents of type `FR` may carry `object: <type>`, which composes a second module's contract onto the ISO one. The by-module table above credits such a document to `spec-artifacts-iso` and so hides this partition; it is shown separately.

| Partition | Documents | Pass | Fail | Pass rate |
|---|---|---|---|---|
| spec-artifacts-process | 39 | 0 | 39 | 0.0 % |
| spec-objects-architecture | 292 | 10 | 282 | 3.4 % |
| spec-objects-operational | 52 | 3 | 49 | 5.8 % |
| spec-objects-business | 452 | 267 | 185 | 59.1 % |
| (object declared by no module) | 3 | 3 | 0 | 100.0 % |

| Object partition | Documents | Pass | Fail | Pass rate |
|---|---|---|---|---|
| spec-objects-business / object: process | 128 | 0 | 128 | 0.0 % |
| spec-objects-architecture / object: api_endpoint | 124 | 0 | 124 | 0.0 % |
| spec-objects-architecture / object: ui_component | 78 | 0 | 78 | 0.0 % |
| spec-artifacts-process / object: standard | 39 | 0 | 39 | 0.0 % |
| spec-objects-architecture / object: action | 18 | 0 | 18 | 0.0 % |
| spec-objects-business / object: event | 9 | 0 | 9 | 0.0 % |
| spec-objects-architecture / object: binary_format | 6 | 0 | 6 | 0.0 % |
| spec-objects-business / object: nested_entity | 4 | 0 | 4 | 0.0 % |
| spec-objects-architecture / object: external_contract | 3 | 0 | 3 | 0.0 % |
| spec-objects-architecture / object: queue | 3 | 0 | 3 | 0.0 % |
| spec-objects-operational / object: deployment | 2 | 0 | 2 | 0.0 % |
| spec-objects-operational / object: configuration | 50 | 3 | 47 | 6.0 % |
| spec-objects-architecture / object: interface | 15 | 1 | 14 | 6.7 % |
| spec-objects-business / object: state_machine | 14 | 2 | 12 | 14.3 % |
| spec-objects-architecture / object: data_schema | 45 | 9 | 36 | 20.0 % |
| spec-objects-business / object: aggregate_root | 7 | 2 | 5 | 28.6 % |
| spec-objects-business / object: entity | 77 | 62 | 15 | 80.5 % |
| spec-objects-business / object: domain | 22 | 20 | 2 | 90.9 % |
| spec-objects-business / object: enumeration | 33 | 31 | 2 | 93.9 % |
| spec-objects-business / object: value_object | 158 | 150 | 8 | 94.9 % |
| (object declared by no module) / object: cli_command | 1 | 1 | 0 | 100.0 % |
| (object declared by no module) / object: cli_module | 1 | 1 | 0 | 100.0 % |
| (object declared by no module) / object: kubernetes_resource | 1 | 1 | 0 | 100.0 % |

## Pass rates by document type

| Partition | Documents | Pass | Fail | Pass rate |
|---|---|---|---|---|
| log | 228 | 0 | 228 | 0.0 % |
| (none) | 162 | 0 | 162 | 0.0 % |
| user-story | 16 | 0 | 16 | 0.0 % |
| functional-requirement | 13 | 0 | 13 | 0.0 % |
| MasterRequirements | 11 | 0 | 11 | 0.0 % |
| discovery | 7 | 0 | 7 | 0.0 % |
| stakeholder-requirement | 6 | 0 | 6 | 0.0 % |
| non-functional-requirement | 4 | 0 | 4 | 0.0 % |
| test-matrix | 2 | 0 | 2 | 0.0 % |
| review | 1 | 0 | 1 | 0.0 % |
| spec-analysis | 1 | 0 | 1 | 0.0 % |
| SpecAnalysis | 1 | 0 | 1 | 0.0 % |
| Evidence | 1 | 0 | 1 | 0.0 % |
| Standard | 41 | 2 | 39 | 4.9 % |
| TestMatrix | 250 | 57 | 193 | 22.8 % |
| MeasurementPlan | 56 | 41 | 15 | 73.2 % |
| FR | 4043 | 3482 | 561 | 86.1 % |
| AssuranceProfile | 9 | 8 | 1 | 88.9 % |
| interface | 45 | 43 | 2 | 95.6 % |
| NFR | 803 | 791 | 12 | 98.5 % |
| StR | 538 | 533 | 5 | 99.1 % |
| SpecReview | 1960 | 1950 | 10 | 99.5 % |
| master-requirements | 280 | 279 | 1 | 99.6 % |
| US | 1091 | 1090 | 1 | 99.9 % |
| index | 1118 | 1117 | 1 | 99.9 % |
| TC | 1712 | 1712 | 0 | 100.0 % |
| IT | 276 | 276 | 0 | 100.0 % |
| ADR | 188 | 188 | 0 | 100.0 % |
| Review | 91 | 91 | 0 | 100.0 % |
| Task | 78 | 78 | 0 | 100.0 % |
| ArchitectureDescription | 48 | 48 | 0 | 100.0 % |
| Plan | 23 | 23 | 0 | 100.0 % |
| value_object | 9 | 9 | 0 | 100.0 % |
| SuiteRegistry | 5 | 5 | 0 | 100.0 % |
| TestMatrixIndex | 3 | 3 | 0 | 100.0 % |
| Spec | 2 | 2 | 0 | 100.0 % |
| Finding | 2 | 2 | 0 | 100.0 % |
| AssuranceArgument | 1 | 1 | 0 | 100.0 % |
| ComponentAssuranceContract | 1 | 1 | 0 | 100.0 % |
| ApplicationSpec | 1 | 1 | 0 | 100.0 % |
| domain | 1 | 1 | 0 | 100.0 % |
| enumeration | 1 | 1 | 0 | 100.0 % |
| state_machine | 1 | 1 | 0 | 100.0 % |
| process | 1 | 1 | 0 | 100.0 % |

Types at 100 % with fewer than 10 documents are not evidence of a sound contract.

## Pass rates by repository

25 repositories pass completely. 258 fail at least one document. Public repositories are named; private repositories are anonymized (names are not published here) but are not aggregated away.

Public repositories: 6,780 / 6,981 = 97.1 %. Private repositories: 5,057 / 6,150 = 82.2 %.

### Public repositories with failures

| Repository | Documents | Pass | Fail | Pass rate |
|---|---|---|---|---|
| quire-corpus | 19 | 10 | 9 | 52.6 % |
| docker-actions | 3 | 2 | 1 | 66.7 % |
| pytest-summary | 3 | 2 | 1 | 66.7 % |
| python-service-actions | 3 | 2 | 1 | 66.7 % |
| ix-cli-core | 45 | 33 | 12 | 73.3 % |
| ecaz | 263 | 195 | 68 | 74.1 % |
| ix-cli | 69 | 53 | 16 | 76.8 % |
| nodejs-actions | 5 | 4 | 1 | 80.0 % |
| quire-wasm | 13 | 11 | 2 | 84.6 % |
| ix-board | 40 | 35 | 5 | 87.5 % |
| ts-plugin-kit | 36 | 32 | 4 | 88.9 % |
| ix-trace-rs | 11 | 10 | 1 | 90.9 % |
| ix-relay | 125 | 115 | 10 | 92.0 % |
| spec-objects-business | 28 | 26 | 2 | 92.9 % |
| quire-mltl | 14 | 13 | 1 | 92.9 % |
| cli-agent-evals | 15 | 14 | 1 | 93.3 % |
| ix-ui | 38 | 36 | 2 | 94.7 % |
| quire-rs | 210 | 200 | 10 | 95.2 % |
| ix-flow | 48 | 46 | 2 | 95.8 % |
| spec-objects-enterprise | 24 | 23 | 1 | 95.8 % |
| spec-objects-operational | 25 | 24 | 1 | 96.0 % |
| spec-objects-safety | 25 | 24 | 1 | 96.0 % |
| spec-artifacts-app | 26 | 25 | 1 | 96.2 % |
| spec-objects-architecture | 26 | 25 | 1 | 96.2 % |
| quire | 54 | 52 | 2 | 96.3 % |
| spec-objects-security | 27 | 26 | 1 | 96.3 % |
| tl-mltl | 125 | 121 | 4 | 96.8 % |
| spec-artifacts-process | 35 | 34 | 1 | 97.1 % |
| quire-protocol | 110 | 107 | 3 | 97.3 % |
| filament-core-data | 298 | 290 | 8 | 97.3 % |
| quoin | 306 | 298 | 8 | 97.4 % |
| quire-verification | 116 | 113 | 3 | 97.4 % |
| tl-rewrite | 135 | 132 | 3 | 97.8 % |
| tl-syntax | 224 | 220 | 4 | 98.2 % |
| quire-contract-codegen | 124 | 122 | 2 | 98.4 % |
| tl-parse | 99 | 98 | 1 | 99.0 % |
| quire-contract-ir | 241 | 240 | 1 | 99.6 % |
| quire-specification | 778 | 776 | 2 | 99.7 % |
| quire-spec-language | 2173 | 2169 | 4 | 99.8 % |

### Private repositories with failures, by pass-rate band

| Pass-rate band | Repositories | Documents |
|---|---|---|
| 0-<50 % | 15 | 264 |
| 50-<80 % | 78 | 1578 |
| 80-<90 % | 62 | 1457 |
| 90-<95 % | 51 | 1967 |
| 95-<100 % | 13 | 598 |

Ten weakest private repositories (anonymized, ordered by pass rate then size):

| Repository | Documents | Pass | Fail | Pass rate |
|---|---|---|---|---|
| private-1 | 65 | 0 | 65 | 0.0 % |
| private-2 | 30 | 0 | 30 | 0.0 % |
| private-3 | 16 | 0 | 16 | 0.0 % |
| private-4 | 16 | 0 | 16 | 0.0 % |
| private-5 | 13 | 0 | 13 | 0.0 % |
| private-6 | 12 | 0 | 12 | 0.0 % |
| private-7 | 1 | 0 | 1 | 0.0 % |
| private-8 | 1 | 0 | 1 | 0.0 % |
| private-9 | 1 | 0 | 1 | 0.0 % |
| private-10 | 43 | 3 | 40 | 7.0 % |

## Failure partition

1,294 failing documents, every one classified; unclassified = 0. A document with failures in several classes is counted once under its **primary** class, taken in this order: contract defect, unsupported representation, legitimate undeclared value, missing structure, malformed document. The order puts first the class a contract change could cure. The per-class "documents with the class" column counts a document under every class it shows.

| Class | Documents (primary) | Documents with the class | Diagnostics |
|---|---|---|---|
| contract-defect | 392 | 392 | 392 |
| unsupported-representation | 161 | 161 | 309 |
| legitimate-undeclared-value | 243 | 243 | 263 |
| missing-structure | 142 | 172 | 323 |
| malformed-document | 356 | 377 | 563 |
| stale-module | 0 | 0 | 0 |
| tool-defect | 0 | 0 | 0 |
| **unclassified** | 0 | 0 | 0 |

`stale-module` and `tool-defect` are 0 as document-failure classes; the stale-tool and tool findings below are about the run, not any one document. 369 of the 392 contract-defect documents fail on the contract defect alone: every structure the module's locators make mandatory is present in them.

### Rules, owners and dispositions

Each failing diagnostic maps to exactly one row, so every failure has an owner and a disposition. Rows repeating the same shape for one-off documents are merged.

| Class | Rule | Documents | Repositories | Owner | Disposition |
|---|---|---|---|---|---|
| contract-defect | record-invalid: `operations` required (api_endpoint) | 124 | 18 | spec-objects-architecture | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `fields` required (ui_component) | 78 | 15 | spec-objects-architecture | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `fields` required (configuration) | 47 | 26 | spec-objects-operational | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `code` required (standard) | 39 | 1 | spec-artifacts-process | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `fields` required (data_schema) | 36 | 4 | spec-objects-architecture | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `operations` required (action) | 18 | 4 | spec-objects-architecture | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `featureOrder` required (interface) | 14 | 7 | spec-objects-architecture | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `fields` required (event) | 9 | 9 | spec-objects-business | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `operations` required (state_machine) | 9 | 8 | spec-objects-business | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `clauses` required (binary_format) | 6 | 1 | spec-objects-architecture | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `operations` required (external_contract) | 3 | 2 | spec-objects-architecture | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `fields` required (entity) | 3 | 2 | spec-objects-business | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | record-invalid: `fields` required (queue) | 3 | 3 | spec-objects-architecture | Contract fix (finding F1/F2); corpus untouched |
| contract-defect | 2 one-off or two-document rules (each at most 2 documents) | 3 | - | owning repository | Contract fix (finding F1/F2); corpus untouched |
| unsupported-representation | missing structure (FR/process) | 128 | 23 | spec-objects-business | Module owner decides if the representation is admitted (finding F8); else deferred corpus fix |
| unsupported-representation | feature-not-extractable: steps (process) | 128 | 23 | spec-objects-business | Module owner decides if the representation is admitted (finding F8); else deferred corpus fix |
| unsupported-representation | feature-not-extractable: relationships (entity) | 12 | 6 | spec-objects-business | Module owner decides if the representation is admitted (finding F8); else deferred corpus fix |
| unsupported-representation | feature-not-extractable: values (value_object) | 6 | 2 | spec-objects-business | Module owner decides if the representation is admitted (finding F8); else deferred corpus fix |
| unsupported-representation | 6 one-off or two-document rules (each at most 2 documents) | 6 | - | spec-objects-business | Module owner decides if the representation is admitted (finding F8); else deferred corpus fix |
| unsupported-representation | feature-not-extractable: members (aggregate_root) | 5 | 3 | spec-objects-business | Module owner decides if the representation is admitted (finding F8); else deferred corpus fix |
| unsupported-representation | feature-not-extractable: relationships (nested_entity) | 4 | 1 | spec-objects-business | Module owner decides if the representation is admitted (finding F8); else deferred corpus fix |
| unsupported-representation | feature-not-extractable: states (state_machine) | 3 | 3 | spec-objects-business | Module owner decides if the representation is admitted (finding F8); else deferred corpus fix |
| legitimate-undeclared-value | unknown type `log` | 228 | 216 | spec-artifacts-iso | Contract fix (finding F3) |
| legitimate-undeclared-value | 6 one-off or two-document rules (each at most 2 documents) | 8 | - | owning module (see findings) | Module owner decides declare-or-retype (finding F7); else deferred corpus fix |
| legitimate-undeclared-value | unknown type `discovery` | 7 | 1 | owning module (see findings) | Module owner decides declare-or-retype (finding F7); else deferred corpus fix |
| missing-structure | missing structure (TestMatrix) | 89 | 87 | owning repository | Deferred to the later corpus normalization campaign |
| missing-structure | missing structure (FR) | 25 | 6 | owning repository | Deferred to the later corpus normalization campaign |
| missing-structure | missing structure (FR/interface) | 14 | 7 | owning repository | Deferred to the later corpus normalization campaign |
| missing-structure | missing structure (FR/state_machine) | 11 | 10 | owning repository | Deferred to the later corpus normalization campaign |
| missing-structure | missing structure (MeasurementPlan) | 8 | 1 | owning repository | Deferred to the later corpus normalization campaign |
| missing-structure | required frontmatter property absent (MeasurementPlan) | 8 | 2 | owning repository | Deferred to the later corpus normalization campaign |
| missing-structure | 5 one-off or two-document rules (each at most 2 documents) | 6 | - | owning repository | Deferred to the later corpus normalization campaign |
| missing-structure | missing structure (FR/aggregate_root) | 5 | 3 | owning repository | Deferred to the later corpus normalization campaign |
| missing-structure | missing structure (StR) | 4 | 2 | owning repository | Deferred to the later corpus normalization campaign |
| missing-structure | missing structure (NFR) | 3 | 2 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | frontmatter has no type | 162 | 33 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | TestMatrix header `Coverage Status` vs asserted `Status` | 111 | 103 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | table row width differs from header | 32 | 17 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | unknown type `user-story` | 16 | 1 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | unknown type `functional-requirement` | 13 | 2 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | unknown type `MasterRequirements` | 11 | 6 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | table columns differ from asserted (TestMatrix) | 8 | 8 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | section does not match required pattern (NFR) | 8 | 1 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | unknown type `stakeholder-requirement` | 6 | 2 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | cell does not match pattern (TestMatrix) | 5 | 5 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | 4 one-off or two-document rules (each at most 2 documents) | 5 | - | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | table columns differ from asserted (FR) | 4 | 3 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | unknown type `non-functional-requirement` | 4 | 2 | owning repository | Deferred to the later corpus normalization campaign |
| malformed-document | enum cell outside declared set (TestMatrix) | 3 | 3 | owning repository | Deferred to the later corpus normalization campaign |

"Owning repository" means the repository holding the document; its fix is a rewrite of that document and belongs to the later normalization campaign, citing the contract named in the row. The `Coverage Status` rows cite the accepted contract: spec-artifacts-process#89 collapsed two names for one column into `Status`.

### Bad rule or bad corpus

Per the repository convention, each class was settled by opening sampled documents, not by the count. Sampled and read: an `api_endpoint` FR (has Endpoint and Inputs; no `## Operations`, which its locator marks optional while its schema requires `operations`: rule, not corpus); a `ui_component` FR (has the mandatory `## Props` table; schema wants `fields`: rule); a `Standard` (carries `code:` in frontmatter: rule); a `process` FR (Workflow is a sequence diagram: corpus form the contract does not admit); `TestMatrix` coverage tables (`Coverage Status`: corpus against an accepted rename); `spec/log.md` (a valid OKF log: undeclared type, not a bad document); an acceptance-criteria table whose rows carry more cells than its header (corpus). Nothing was reclassified by count.

## Findings

Findings are listed, not applied: no module or validator was edited. "Cheap" means a small, local contract edit that the measured cases clearly justify.

| Id | Priority | Class | Module | Finding and evidence | Owner |
|---|---|---|---|---|---|
| F1 | P1 | contract defect | `spec-objects-architecture`, `spec-objects-business`, `spec-objects-operational` | A data schema marks `operations`, `fields`, `featureOrder` or `clauses` required while the same module's locator declares the matching section `required: false` or extracts nothing for it. A document that satisfies every mandatory locator still fails. Probe: a fully formed `process` FR (valid Workflow Step table, Description, ACs) fails only on `fields` required. 392 documents; 369 fail on this alone. Largest: `api_endpoint` `operations` 124, `ui_component` `fields` 78, `configuration` `fields` 47, `data_schema` `fields` 36. Decision needed: relax the schema, make the locator mandatory, or declare the mapping that feeds the member. Not cheap: it picks between three contracts. | Module owners; needs a human owner decision |
| F2 | P1 | contract defect | `spec-artifacts-process` | `schemas/Standard.json` requires `code`; the Standard locator never reads it and the corpus carries `code:` in frontmatter. 39 of 41 `Standard` documents (all in one repository) fail. The manifest comment states every Standard document validates without the typed sections. Cheap: feed `code` from frontmatter or drop it from `required`. | spec-artifacts-process owner |
| F3 | P1 | legitimate undeclared value | `spec-artifacts-iso` | `type: log` is declared by no module: 228 documents in 216 repositories (the OKF bundle update log, `spec/log.md`), against the declared sibling `index`. Cheap: declare a `log` archetype beside `index`. | spec-artifacts-iso owner |
| F4 | P2 | contract defect (loader) | `spec-artifacts-process` | Every run reports `DuplicateArchetype` for ADR, Plan, Review, SpecReview and Standard contributed twice by this one module (first-wins), and `semantic.inline-data-schema` for `standard`. Non-fatal. Cheap: declare each once and use the `{ schema }` reference form. | spec-artifacts-process owner |
| F5 | P2 | tool defect | `quire-rs` | `semantic.feature-not-extractable` says "the module manifest does not declare it" when the manifest does declare the feature through a locator and the document's section is in another form (a Workflow diagram). The message sends the reader to the wrong fix. 140 diagnostics on `process` alone. | quire-rs owner |
| F6 | P1 | stale tool | `quire-cli` release lane | The `quire` installed on this machine (0.36.2, engine 0.50.2) cannot load the completed modules: a one-document probe reports `unknown type 'FR'` for a valid FR, so every typed document would fail. The modules require `quire ^0.52`. Only this probe was run with the installed CLI; the corpus was not measured with it. Needs the installed CLI and `quoin module ensure-defaults` pins brought to the completed modules before enforcement. | quire-cli / quoin release owners |
| F7 | P2 | legitimate undeclared value | `spec-objects-business`, `spec-artifacts-process`, `spec-artifacts-iso` | Small counts of values the contract rejects and a human must rule on: kebab-case enumeration values rejected as non-`Identifier` (22 diagnostics, 2 documents); TestMatrix `Status` cell outside the emoji pattern (`Superseded`); `type: discovery` (7 documents, one repository), `Evidence`, `SpecAnalysis`; an NFR category `interoperability`. Declare or retype each; not cheap, because the Identifier rule is shared. | Module owners |
| F8 | P2 | unsupported representation | `spec-objects-business` | 128 `process` FRs in 23 repositories carry `## Workflow` as a sequence diagram where the module requires a Step table; smaller counts for `## Relationships`, `## States`, `## Members`, `## Properties` in non-table forms. Decide whether a diagram is an admitted representation; otherwise the documents are rewritten in the normalization campaign. | Module owner, then corpus campaign |

F1, F2, F3 and F6 are P0/P1 contract findings awaiting an explicit human-owner acceptance or fix; none is resolved by this report. A 9.9 % failure rate pauses promotion of these constraints to enforcement; it does not justify weakening a validator, and none was weakened.

## Advisory grammar findings

1,553 documents carry at least one advisory warning. They never fail a document and are not part of the pass rate. Most frequent checks, by documents carrying them:

| Check | Documents | Diagnostics |
|---|---|---|
| quality:agentless-passive | 529 | 716 |
| ears:non-singular | 375 | 594 |
| ac:non-canonical-shape | 289 | 1128 |
| semantic.legacy-properties-form | 161 | 161 |
| quality:ambiguous-term | 154 | 176 |
| ears:missing-subject | 128 | 184 |
| ears:unclassifiable | 128 | 184 |
| quality:mixed-modal | 127 | 140 |
| ears:vague-response | 116 | 125 |
| ears:non-canonical-trigger | 77 | 98 |
| ac:vague-response | 60 | 67 |
| semantic.unresolved-type | 49 | 106 |

`semantic.legacy-properties-form` (161 documents) is the legacy-form advisory that modules declare as `legacy_forms: warning`: failing those documents would be this report inventing a rule.

## What this measurement could not establish

- The 45 listed repositories with no `spec/` Markdown are unavailable as a population.
- The three modules `spec-objects-security`, `spec-objects-enterprise`, `spec-objects-safety` have no measured population in this corpus; their contracts are untested here.
- Documents outside `spec/` and non-Markdown spec files (JSON, YAML) were not validated.
- Documents were validated one at a time with `--scope` at their repository, so cross-document and cross-repository link resolution is whatever a one-document run sees.
- Stale-module effects of the shipped `default-modules.yaml` pins against current `main` were not measured beyond F6.
- The `quire` field-level semantic checks beyond what the engine emitted in this run were not separately assessed.
