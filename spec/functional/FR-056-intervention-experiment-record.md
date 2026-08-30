---
id: FR-056
title: "Intervention-experiment evidence record"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quoin/US-015"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
---

# FR-056: Intervention-experiment evidence record

## Description

An intervention-experiment evidence record SHALL conform to the versioned JSON
Schema below so that its design, observations, attribution limits, governance, and
producer provenance remain engine-independent and independently auditable.

## Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/intervention-experiment-v1.schema.json",
  "title": "InterventionExperimentEvidenceV1",
  "type": "object",
  "required": [
    "schema_version",
    "record_type",
    "record_id",
    "subject",
    "producer",
    "design",
    "baseline",
    "treatments",
    "changed_variables",
    "held_constant",
    "measured_effects",
    "interactions",
    "confounders",
    "status",
    "conclusion",
    "gaps",
    "owner",
    "actions",
    "raw_evidence"
  ],
  "properties": {
    "schema_version": { "const": 1 },
    "record_type": { "const": "intervention_experiment" },
    "record_id": { "$ref": "#/$defs/identity" },
    "subject": {
      "type": "object",
      "required": ["id", "revision"],
      "properties": {
        "id": { "$ref": "#/$defs/identity" },
        "revision": { "type": "string", "minLength": 1 }
      },
      "additionalProperties": false
    },
    "producer": {
      "type": "object",
      "required": [
        "tool_identity",
        "tool_version",
        "configuration_digest",
        "source_revision",
        "environment",
        "definition_version"
      ],
      "properties": {
        "tool_identity": { "type": "string", "minLength": 1 },
        "tool_version": { "$ref": "#/$defs/immutable_version" },
        "configuration_digest": { "$ref": "#/$defs/digest" },
        "source_revision": { "type": "string", "minLength": 1 },
        "environment": {
          "type": "object",
          "minProperties": 1,
          "additionalProperties": {
            "anyOf": [
              { "type": "string" },
              { "type": "number" },
              { "type": "boolean" },
              { "type": "null" }
            ]
          }
        },
        "definition_version": { "type": "string", "minLength": 1 }
      },
      "additionalProperties": false
    },
    "design": {
      "type": "object",
      "required": ["kind", "repetitions", "assignment", "sampling_conditions"],
      "properties": {
        "kind": {
          "type": "string",
          "enum": ["repeated", "randomized", "factorial"]
        },
        "repetitions": { "type": "integer", "minimum": 1 },
        "assignment": {
          "type": "object",
          "required": ["method"],
          "properties": {
            "method": {
              "type": "string",
              "enum": [
                "not_applicable",
                "deterministic",
                "randomized",
                "blocked_randomized"
              ]
            },
            "seed": { "type": "string", "minLength": 1 }
          },
          "allOf": [
            {
              "if": {
                "properties": {
                  "method": { "enum": ["randomized", "blocked_randomized"] }
                },
                "required": ["method"]
              },
              "then": { "required": ["seed"] }
            }
          ],
          "additionalProperties": false
        },
        "sampling_conditions": {
          "type": "array",
          "minItems": 1,
          "items": { "type": "string", "minLength": 1 }
        }
      },
      "allOf": [
        {
          "if": {
            "properties": { "kind": { "const": "randomized" } },
            "required": ["kind"]
          },
          "then": {
            "properties": {
              "assignment": {
                "properties": {
                  "method": {
                    "enum": ["randomized", "blocked_randomized"]
                  }
                }
              }
            }
          }
        }
      ],
      "additionalProperties": false
    },
    "baseline": { "$ref": "#/$defs/arm" },
    "treatments": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/$defs/arm" }
    },
    "changed_variables": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/$defs/changed_variable" }
    },
    "held_constant": {
      "type": "array",
      "items": { "$ref": "#/$defs/held_variable" }
    },
    "measured_effects": {
      "type": "array",
      "items": {
        "type": "object",
        "required": [
          "treatment_id",
          "metric",
          "baseline_value",
          "treatment_value",
          "effect",
          "unit"
        ],
        "properties": {
          "treatment_id": { "$ref": "#/$defs/identity" },
          "metric": { "$ref": "#/$defs/identity" },
          "baseline_value": {
            "anyOf": [
              { "type": "number" },
              { "type": "string" },
              { "type": "boolean" },
              { "type": "null" }
            ]
          },
          "treatment_value": {
            "anyOf": [
              { "type": "number" },
              { "type": "string" },
              { "type": "boolean" },
              { "type": "null" }
            ]
          },
          "effect": {
            "anyOf": [
              { "type": "number" },
              { "type": "string" },
              { "type": "null" }
            ]
          },
          "unit": { "type": "string", "minLength": 1 }
        },
        "additionalProperties": false
      }
    },
    "interactions": {
      "type": "array",
      "items": { "$ref": "#/$defs/qualified_observation" }
    },
    "confounders": {
      "type": "array",
      "items": { "$ref": "#/$defs/qualified_observation" }
    },
    "status": {
      "type": "string",
      "enum": ["completed", "failed", "inconclusive"]
    },
    "conclusion": {
      "type": "object",
      "required": ["kind", "statement", "attribution_confidence"],
      "properties": {
        "kind": {
          "type": "string",
          "enum": [
            "causal_effect_established",
            "no_effect_observed",
            "cause_not_established"
          ]
        },
        "statement": { "type": "string", "minLength": 1 },
        "attribution_confidence": {
          "type": "string",
          "enum": ["none", "low", "moderate", "high"]
        }
      },
      "additionalProperties": false
    },
    "gaps": {
      "type": "array",
      "items": { "type": "string", "minLength": 1 }
    },
    "owner": { "type": "string", "minLength": 1 },
    "actions": {
      "type": "array",
      "minItems": 1,
      "items": { "type": "string", "minLength": 1 }
    },
    "raw_evidence": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "required": ["path", "media_type", "size_bytes", "digest"],
        "properties": {
          "path": { "type": "string", "minLength": 1 },
          "media_type": { "type": "string", "minLength": 1 },
          "size_bytes": { "type": "integer", "minimum": 0 },
          "digest": { "$ref": "#/$defs/digest" }
        },
        "additionalProperties": false
      }
    }
  },
  "allOf": [
    {
      "if": {
        "type": "object",
        "properties": {
          "status": { "enum": ["failed", "inconclusive"] }
        },
        "required": ["status"]
      },
      "then": {
        "properties": {
          "conclusion": {
            "type": "object",
            "properties": { "kind": { "const": "cause_not_established" } },
            "required": ["kind"]
          }
        }
      }
    },
    {
      "if": {
        "type": "object",
        "properties": {
          "conclusion": {
            "type": "object",
            "properties": { "kind": { "const": "causal_effect_established" } },
            "required": ["kind"]
          }
        },
        "required": ["conclusion"]
      },
      "then": {
        "properties": {
          "baseline": {
            "type": "object",
            "properties": { "sample_size": { "minimum": 1 } }
          },
          "treatments": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": { "sample_size": { "minimum": 1 } }
            }
          },
          "measured_effects": {
            "type": "array",
            "minItems": 1,
            "items": {
              "type": "object",
              "properties": {
                "baseline_value": { "not": { "type": "null" } },
                "treatment_value": { "not": { "type": "null" } },
                "effect": { "not": { "type": "null" } }
              }
            }
          },
          "interactions": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "disposition": { "enum": ["controlled", "not_applicable"] }
              }
            }
          },
          "confounders": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "disposition": { "enum": ["controlled", "not_applicable"] }
              }
            }
          },
          "conclusion": {
            "type": "object",
            "properties": {
              "attribution_confidence": {
                "enum": ["low", "moderate", "high"]
              }
            }
          }
        }
      }
    },
    {
      "if": {
        "type": "object",
        "properties": {
          "conclusion": {
            "type": "object",
            "properties": { "kind": { "const": "no_effect_observed" } },
            "required": ["kind"]
          }
        },
        "required": ["conclusion"]
      },
      "then": {
        "properties": {
          "baseline": {
            "type": "object",
            "properties": { "sample_size": { "minimum": 1 } }
          },
          "treatments": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": { "sample_size": { "minimum": 1 } }
            }
          },
          "measured_effects": {
            "type": "array",
            "minItems": 1,
            "items": {
              "type": "object",
              "properties": {
                "baseline_value": { "not": { "type": "null" } },
                "treatment_value": { "not": { "type": "null" } }
              }
            }
          }
        }
      }
    }
  ],
  "$defs": {
    "identity": {
      "type": "string",
      "pattern": "^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$"
    },
    "digest": {
      "type": "string",
      "pattern": "^(sha256|blake3):[a-f0-9]{64}$"
    },
    "immutable_version": {
      "type": "string",
      "pattern": "^(v?[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|(sha256|blake3):[a-f0-9]{64})$"
    },
    "arm": {
      "type": "object",
      "required": ["id", "population", "sample_size", "configuration"],
      "properties": {
        "id": { "$ref": "#/$defs/identity" },
        "population": { "type": "string", "minLength": 1 },
        "sample_size": { "type": "integer", "minimum": 0 },
        "configuration": { "type": "object" }
      },
      "additionalProperties": false
    },
    "changed_variable": {
      "type": "object",
      "required": ["name", "treatment_id", "baseline_value", "treatment_value"],
      "properties": {
        "name": { "type": "string", "minLength": 1 },
        "treatment_id": { "$ref": "#/$defs/identity" },
        "baseline_value": {},
        "treatment_value": {}
      },
      "additionalProperties": false
    },
    "held_variable": {
      "type": "object",
      "required": ["name", "value"],
      "properties": {
        "name": { "type": "string", "minLength": 1 },
        "value": {}
      },
      "additionalProperties": false
    },
    "qualified_observation": {
      "type": "object",
      "required": ["description", "disposition"],
      "properties": {
        "description": { "type": "string", "minLength": 1 },
        "disposition": {
          "type": "string",
          "enum": ["controlled", "uncontrolled", "unknown", "not_applicable"]
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

## Cross-record integrity

- The baseline id and every treatment id SHALL be unique within the record.
- Every `changed_variables[].treatment_id` and
  `measured_effects[].treatment_id` SHALL resolve to exactly one treatment in the
  same record.
- Each `(treatment_id, name)` changed-variable key and each
  `(treatment_id, metric)` measured-effect key SHALL occur at most once.
- Every raw-evidence path SHALL be a normalized relative path within the evidence
  store. Absolute paths, parent traversal, and paths that resolve outside the
  evidence store SHALL be refused.
- A `causal_effect_established` conclusion SHALL be refused when any interaction
  or confounder is `uncontrolled` or `unknown`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-056-AC-1 | The schema requires version, record type and identity, subject identity and revision, and the unchanged FR-044 producer tuple. | Test (TC-1195) |
| FR-056-AC-2 | The producer tuple rejects a missing field, a mutable tool version, an empty environment, and a malformed configuration digest. | Test (TC-1196) |
| FR-056-AC-3 | Repeated, randomized, and factorial designs require a positive repetition count, assignment method, and non-empty sampling conditions; randomized assignment requires a reproducible seed. | Test (TC-1197) |
| FR-056-AC-4 | Every record carries one uniquely identified baseline, one or more uniquely identified treatments, treatment-linked changed variables, and an explicit list of held-constant variables. | Test (TC-1198) |
| FR-056-AC-5 | Measured effects preserve treatment identity, baseline value, treatment value, effect, metric identity, and unit without replacing a missing value with zero or permitting duplicate treatment/metric keys. | Test (TC-1199) |
| FR-056-AC-6 | Interactions and confounders remain separate collections whose entries distinguish controlled, uncontrolled, unknown, and not-applicable dispositions. | Test (TC-1200) |
| FR-056-AC-7 | Completed, failed, and inconclusive statuses validate; failed and inconclusive records require the first-class `cause_not_established` conclusion. | Test (TC-1201) |
| FR-056-AC-8 | A causal-effect conclusion requires positive baseline and treatment samples, at least one non-null measured effect, non-`none` attribution confidence, and no uncontrolled or unknown interaction or confounder; a no-effect conclusion requires positive samples and an observed comparison. | Test (TC-1202) |
| FR-056-AC-9 | Gaps, owner, actions, and at least one content-digested raw-evidence reference with media type and byte size are retained; unsafe paths and undeclared fields are refused. | Test (TC-1203) |

## Dependencies

- **Upstream**: [US-015](../usecase/US-015-assess-intervention-experiments.md)
  states the practitioner need; [FR-044](./FR-044-plan-governed-measurements.md)
  defines the producer tuple and governed evidence-store boundary.
- **Downstream**: [FR-057](./FR-057-intervention-experiment-intake-report.md)
  governs validation, persistence, and report interpretation of this record.
