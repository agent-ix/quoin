# Default-module semantic type-fit audit

Canonical schema: `semantic-module-type-fit-v1`

Verdict: **findings**

## Denominators

| Population | Source | Inventoried | Reconciled |
| --- | ---: | ---: | --- |
| contractSurfaces | 450 | 450 | yes |
| declarations | 90 | 90 | yes |
| documents | 299 | 299 | yes |
| modules | 10 | 10 | yes |
| parseStates | 299 | 299 | yes |

## Module census

| Module | Declarations | Markdown paths | Matched instances | Conflicts |
| --- | ---: | ---: | ---: | ---: |
| engineering-assurance | 5 | 11 | 5 | 1 |
| spec-artifacts-iso | 10 | 31 | 24 | 1 |
| spec-artifacts-app | 2 | 16 | 1 | 1 |
| spec-artifacts-process | 13 | 87 | 58 | 2 |
| spec-objects-business | 10 | 26 | 10 | 1 |
| spec-objects-architecture | 10 | 26 | 10 | 1 |
| spec-objects-enterprise | 7 | 23 | 7 | 1 |
| spec-objects-operational | 8 | 24 | 8 | 1 |
| spec-objects-security | 23 | 39 | 23 | 1 |
| spec-objects-safety | 2 | 16 | 2 | 1 |


## Type dispositions

| Disposition | Declarations |
| --- | ---: |
| fits | 16 |
| incomplete | 74 |


## Findings

| ID | Kind | Severity | Status | Rationale |
| --- | --- | --- | --- | --- |
| PROV-001 | provenance-conflict | high | open | declaredVersion "0.2.0+4e6522f" disagrees with manifestVersion "0.2.0" |
| PROV-002 | provenance-conflict | high | open | declaredVersion "0.18.0" disagrees with manifestVersion "0.1.0" |
| PROV-003 | provenance-conflict | high | open | declaredVersion "0.5.0" disagrees with manifestVersion "0.1.0" |
| PROV-004 | provenance-conflict | high | open | declaredVersion "0.24.0+d605caa" disagrees with manifestVersion "0.1.0" |
| PROV-005 | provenance-conflict | high | open | declaredVersion "0.6.0" disagrees with manifestVersion "0.2.0" |
| PROV-006 | provenance-conflict | high | open | declaredVersion "0.6.0" disagrees with manifestVersion "0.2.0" |
| PROV-007 | provenance-conflict | high | open | declaredVersion "0.5.0" disagrees with manifestVersion "0.1.0" |
| PROV-008 | provenance-conflict | high | open | declaredVersion "0.6.0" disagrees with manifestVersion "0.2.0" |
| PROV-009 | provenance-conflict | high | open | declaredVersion "0.7.0" disagrees with manifestVersion "0.1.0" |
| PROV-010 | provenance-conflict | high | open | declaredVersion "0.4.0" disagrees with manifestVersion "0.2.0" |
| CONFLICT-011 | duplicate-type | medium | open | module-qualified declarations named Standard have incompatible shapes |
| MISSING-001 | missing-type | medium | open | run has no dedicated or structural representation in the default module set |
| MISSING-002 | missing-type | medium | open | result has no dedicated or structural representation in the default module set |
| MISSING-003 | missing-type | medium | overloaded-review-required | evidence appears only inside another declaration rather than as a dedicated semantic type |
| MISSING-004 | missing-type | medium | open | report has no dedicated or structural representation in the default module set |
| MISSING-005 | missing-type | medium | represented-review-required | relationship is represented in the current module vocabulary or structural contract and still requires semantic-boundary review |
| MISSING-006 | missing-type | medium | represented-review-required | identity is represented in the current module vocabulary or structural contract and still requires semantic-boundary review |
| MISSING-007 | missing-type | medium | represented-review-required | version is represented in the current module vocabulary or structural contract and still requires semantic-boundary review |
| MISSING-008 | missing-type | medium | open | provenance has no dedicated or structural representation in the default module set |
| MISSING-009 | missing-type | medium | represented-review-required | lifecycle is represented in the current module vocabulary or structural contract and still requires semantic-boundary review |

## Repository impact

| Repository/boundary | Impact | Effort | Risk | Wave | Confidence |
| --- | --- | --- | --- | --- | --- |
| engineering-assurance | required | medium | medium | module-contract-review | high |
| spec-artifacts-iso | required | medium | medium | module-contract-review | high |
| spec-artifacts-app | required | medium | medium | module-contract-review | high |
| spec-artifacts-process | required | medium | medium | module-contract-review | high |
| spec-objects-business | required | medium | medium | module-contract-review | high |
| spec-objects-architecture | required | medium | medium | module-contract-review | high |
| spec-objects-enterprise | required | medium | medium | module-contract-review | high |
| spec-objects-operational | required | medium | medium | module-contract-review | high |
| spec-objects-security | required | medium | medium | module-contract-review | high |
| spec-objects-safety | required | medium | medium | module-contract-review | high |
| quoin | required | unknown | medium | gated-follow-up | low |
| quire | candidate | unknown | medium | gated-follow-up | low |
| filament-core-data | candidate | unknown | medium | gated-follow-up | low |
| compiler | candidate | unknown | high | gated-follow-up | low |
| generated-packages | candidate | unknown | high | gated-follow-up | low |
| database | candidate | unknown | high | gated-follow-up | low |
| api | candidate | unknown | high | gated-follow-up | low |
| cli | candidate | unknown | medium | gated-follow-up | low |
| ui | candidate | unknown | medium | gated-follow-up | low |

## Interpretation

The census is complete for the recorded pins; a `findings` verdict means the data cannot be promoted as a conflict-free semantic contract. Every compiler, schema, migration, publication, enforcement, retirement, database, API, CLI, UI, and generated-package recommendation remains a separately gated follow-up.
