---
id: SR-055
title: "Default-module semantic type-fit review"
type: SpecReview
analysis: architecture-evaluation
scope: "default-modules.yaml complete pinned corpus"
---

# Default-module semantic type-fit review

## Summary

Generated from `semantic-module-type-fit-v1`; verdict **findings**. Canonical findings remain in JSON and this document is their human review projection.

## Denominators

| Population | Source | Inventoried | Reconciled |
| --- | ---: | ---: | --- |
| contractSurfaces | 450 | 450 | yes |
| declarations | 90 | 90 | yes |
| documents | 299 | 299 | yes |
| modules | 10 | 10 | yes |
| parseStates | 299 | 299 | yes |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-046 | high | PROV-001: declaredVersion "0.2.0+4e6522f" disagrees with manifestVersion "0.2.0" | snapshot.modules[0].declaredVersion, snapshot.modules[0].manifestVersion |
| FND-047 | high | PROV-002: declaredVersion "0.18.0" disagrees with manifestVersion "0.1.0" | snapshot.modules[1].declaredVersion, snapshot.modules[1].manifestVersion |
| FND-048 | high | PROV-003: declaredVersion "0.5.0" disagrees with manifestVersion "0.1.0" | snapshot.modules[2].declaredVersion, snapshot.modules[2].manifestVersion |
| FND-049 | high | PROV-004: declaredVersion "0.24.0+d605caa" disagrees with manifestVersion "0.1.0" | snapshot.modules[3].declaredVersion, snapshot.modules[3].manifestVersion |
| FND-050 | high | PROV-005: declaredVersion "0.6.0" disagrees with manifestVersion "0.2.0" | snapshot.modules[4].declaredVersion, snapshot.modules[4].manifestVersion |
| FND-051 | high | PROV-006: declaredVersion "0.6.0" disagrees with manifestVersion "0.2.0" | snapshot.modules[5].declaredVersion, snapshot.modules[5].manifestVersion |
| FND-052 | high | PROV-007: declaredVersion "0.5.0" disagrees with manifestVersion "0.1.0" | snapshot.modules[6].declaredVersion, snapshot.modules[6].manifestVersion |
| FND-053 | high | PROV-008: declaredVersion "0.6.0" disagrees with manifestVersion "0.2.0" | snapshot.modules[7].declaredVersion, snapshot.modules[7].manifestVersion |
| FND-054 | high | PROV-009: declaredVersion "0.7.0" disagrees with manifestVersion "0.1.0" | snapshot.modules[8].declaredVersion, snapshot.modules[8].manifestVersion |
| FND-055 | high | PROV-010: declaredVersion "0.4.0" disagrees with manifestVersion "0.2.0" | snapshot.modules[9].declaredVersion, snapshot.modules[9].manifestVersion |
| FND-056 | medium | CONFLICT-011: module-qualified declarations named Standard have incompatible shapes | inventory.declarations:spec-artifacts-process::artifact::Standard::9, inventory.declarations:spec-artifacts-process::object::standard::0 |
| FND-057 | medium | MISSING-001: run has no dedicated or structural representation in the default module set | inventory.declarations:complete-name-and-field-census |
| FND-058 | medium | MISSING-002: result has no dedicated or structural representation in the default module set | inventory.declarations:complete-name-and-field-census |
| FND-059 | medium | MISSING-003: evidence appears only inside another declaration rather than as a dedicated semantic type | inventory.declarations:embedded-concept-signal |
| FND-060 | medium | MISSING-004: report has no dedicated or structural representation in the default module set | inventory.declarations:complete-name-and-field-census |
| FND-061 | medium | MISSING-005: relationship is represented in the current module vocabulary or structural contract and still requires semantic-boundary review | inventory.declarations:allowed_links |
| FND-062 | medium | MISSING-006: identity is represented in the current module vocabulary or structural contract and still requires semantic-boundary review | inventory.documents:documentId |
| FND-063 | medium | MISSING-007: version is represented in the current module vocabulary or structural contract and still requires semantic-boundary review | snapshot.modules:version+resolvedSha |
| FND-064 | medium | MISSING-008: provenance has no dedicated or structural representation in the default module set | inventory.declarations:complete-name-and-field-census |
| FND-065 | medium | MISSING-009: lifecycle is represented in the current module vocabulary or structural contract and still requires semantic-boundary review | inventory.declarations:lifecycle-signals |
