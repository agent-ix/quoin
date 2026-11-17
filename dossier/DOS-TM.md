---
id: "DOS-TM"
title: "Threat model"
version: "1.0.0"
type: DossierDocument
document_kind: "threat-model"
generated_by: quoin dossier
---

# Threat model

## TM-1

### Scope

Method: stride_per_interaction

Reviewed by: A. Person

Covered parts: PART-1

Covered trust boundaries: BOUND-1

#### Scope


The complete fixture covers the connection and database sink.


#### Assumptions


The connection uses its recorded controls.


#### Out of scope


No other components are included.


### Covered crossings

| Connection | Carried classifications and categories | Boundary and trust level | Threats (STRIDE class) | Controls and safeguards | Acceptance |
| --- | --- | --- | --- | --- | --- |
| CONN-1 | CLASS-1 (health) | BOUND-1 (trusted) | THREAT-1 (information_disclosure) | CONTROL-1 (not stated) | RISK-1: accepted, A. Person |

### Covered sinks

#### SINK-1

Kind: datastore

Classifications: CLASS-1 (health)

Threats: THREAT-1 (information_disclosure)

Controls and safeguards: CONTROL-1 (not stated)

### Open findings

none

## Gaps

Gaps: none
