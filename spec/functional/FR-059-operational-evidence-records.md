---
id: FR-059
title: "Operational evidence record family"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quoin/US-016"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
---

# FR-059: Operational evidence record family

## Description

An operational evidence record SHALL conform to the versioned JSON Schema below
so that standing control capabilities and control exercises remain distinct,
engine-independent, and traceable to deployed scope, timing, outcome, governance,
raw evidence, and the unchanged FR-044 producer tuple.

## Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quoin/schemas/operational-evidence-v1.schema.json",
  "title": "OperationalEvidenceV1",
  "type": "object",
  "required": [
    "schema_version",
    "record_type",
    "record_id",
    "record_shape",
    "control_kind",
    "subject",
    "producer",
    "scope",
    "configuration",
    "owner",
    "gaps",
    "actions",
    "raw_evidence"
  ],
  "properties": {
    "schema_version": { "const": 1 },
    "record_type": { "const": "operational_evidence" },
    "record_id": { "$ref": "#/$defs/identity" },
    "record_shape": {
      "type": "string",
      "enum": ["standing_capability", "exercise"]
    },
    "control_kind": {
      "type": "string",
      "enum": [
        "release",
        "feature_flag",
        "canary_deployment",
        "shadow_deployment",
        "rollback",
        "kill_switch",
        "human_override",
        "appeal",
        "abstention",
        "safe_fallback",
        "policy_pin",
        "prompt_pin",
        "model_pin",
        "tool_pin",
        "data_pin",
        "reporting"
      ]
    },
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
    "scope": {
      "type": "object",
      "required": ["service", "environment", "population"],
      "properties": {
        "service": { "$ref": "#/$defs/identity" },
        "environment": { "type": "string", "minLength": 1 },
        "population": { "type": "string", "minLength": 1 }
      },
      "additionalProperties": false
    },
    "configuration": {
      "type": "object",
      "required": ["version_pins"],
      "properties": {
        "version_pins": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["kind", "identity", "revision", "digest"],
            "properties": {
              "kind": {
                "type": "string",
                "enum": ["policy", "prompt", "model", "tool", "data"]
              },
              "identity": { "$ref": "#/$defs/identity" },
              "revision": { "type": "string", "minLength": 1 },
              "digest": { "$ref": "#/$defs/digest" }
            },
            "additionalProperties": false
          }
        }
      },
      "additionalProperties": false
    },
    "capability": {
      "type": "object",
      "required": [
        "control_id",
        "status",
        "surface",
        "authorized_roles",
        "coverage",
        "limitations",
        "supported_transitions",
        "clock_support"
      ],
      "properties": {
        "control_id": { "$ref": "#/$defs/identity" },
        "status": {
          "type": "string",
          "enum": ["available", "unavailable", "unknown", "not_applicable"]
        },
        "surface": { "type": "string", "minLength": 1 },
        "authorized_roles": {
          "type": "array",
          "minItems": 1,
          "items": { "type": "string", "minLength": 1 }
        },
        "coverage": { "type": "string", "minLength": 1 },
        "limitations": {
          "type": "array",
          "items": { "type": "string", "minLength": 1 }
        },
        "supported_transitions": {
          "type": "array",
          "minItems": 1,
          "items": { "type": "string", "minLength": 1 }
        },
        "clock_support": {
          "type": "object",
          "required": ["supported", "start_event", "completion_event"],
          "properties": {
            "supported": { "type": "boolean" },
            "start_event": { "type": "string", "minLength": 1 },
            "completion_event": { "type": "string", "minLength": 1 },
            "deadline_seconds": { "type": "integer", "minimum": 1 }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    "exercise": {
      "type": "object",
      "required": [
        "control_id",
        "mode",
        "started_at",
        "completed_at",
        "actor",
        "trigger",
        "outcome",
        "state_before",
        "state_after",
        "observations",
        "clock"
      ],
      "properties": {
        "control_id": { "$ref": "#/$defs/identity" },
        "capability_record_id": { "$ref": "#/$defs/identity" },
        "mode": { "type": "string", "enum": ["actual", "drill"] },
        "started_at": { "type": "string", "format": "date-time" },
        "completed_at": {
          "anyOf": [
            { "type": "string", "format": "date-time" },
            { "type": "null" }
          ]
        },
        "actor": { "type": "string", "minLength": 1 },
        "trigger": { "type": "string", "minLength": 1 },
        "outcome": {
          "type": "string",
          "enum": ["succeeded", "failed", "partial", "aborted"]
        },
        "state_before": { "type": "object" },
        "state_after": { "type": "object" },
        "observations": {
          "type": "array",
          "items": { "type": "string", "minLength": 1 }
        },
        "clock": { "$ref": "#/$defs/clock" }
      },
      "additionalProperties": false
    },
    "owner": { "type": "string", "minLength": 1 },
    "gaps": {
      "type": "array",
      "items": { "type": "string", "minLength": 1 }
    },
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
        "required": ["path", "digest"],
        "properties": {
          "path": { "type": "string", "minLength": 1 },
          "digest": { "$ref": "#/$defs/digest" }
        },
        "additionalProperties": false
      }
    }
  },
  "allOf": [
    {
      "oneOf": [
        {
          "type": "object",
          "properties": {
            "record_shape": { "const": "standing_capability" },
            "capability": {}
          },
          "required": ["record_shape", "capability"],
          "not": {
            "type": "object",
            "properties": { "exercise": {} },
            "required": ["exercise"]
          }
        },
        {
          "type": "object",
          "properties": {
            "record_shape": { "const": "exercise" },
            "exercise": {}
          },
          "required": ["record_shape", "exercise"],
          "not": {
            "type": "object",
            "properties": { "capability": {} },
            "required": ["capability"]
          }
        }
      ]
    },
    {
      "if": {
        "type": "object",
        "properties": {
          "control_kind": {
            "enum": [
              "policy_pin",
              "prompt_pin",
              "model_pin",
              "tool_pin",
              "data_pin"
            ]
          }
        },
        "required": ["control_kind"]
      },
      "then": {
        "properties": {
          "configuration": {
            "type": "object",
            "properties": {
              "version_pins": { "type": "array", "minItems": 1 }
            },
            "required": ["version_pins"]
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
    "clock": {
      "type": "object",
      "required": ["applicability", "status"],
      "properties": {
        "applicability": {
          "type": "string",
          "enum": ["operational_with_clock", "not_applicable"]
        },
        "started_at": { "type": "string", "format": "date-time" },
        "deadline_at": { "type": "string", "format": "date-time" },
        "completed_at": { "type": "string", "format": "date-time" },
        "status": {
          "type": "string",
          "enum": ["not_applicable", "open", "met", "missed", "unknown"]
        }
      },
      "allOf": [
        {
          "if": {
            "type": "object",
            "properties": {
              "applicability": { "const": "operational_with_clock" }
            },
            "required": ["applicability"]
          },
          "then": {
            "properties": {
              "started_at": { "type": "string", "format": "date-time" },
              "deadline_at": { "type": "string", "format": "date-time" },
              "status": { "enum": ["open", "met", "missed", "unknown"] }
            },
            "required": ["started_at", "deadline_at"]
          },
          "else": {
            "properties": { "status": { "const": "not_applicable" } }
          }
        }
      ],
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-059-AC-1 | The envelope requires version, record and subject identities, deployed scope, shape, control kind, governance fields, raw evidence, and every unchanged FR-044 producer-tuple field. | Test (TC-1222) |
| FR-059-AC-2 | The control vocabulary admits releases, flags, canary and shadow deployment, rollback and kill, override and appeal, abstention and fallback, reporting, and policy, prompt, model, tool, and data pinning. | Test (TC-1223) |
| FR-059-AC-3 | A standing-capability record requires its control surface, availability state, authorized roles, coverage, limitations, supported transitions, and clock-start, clock-completion, and deadline declarations. | Test (TC-1224) |
| FR-059-AC-4 | An exercise record requires actual-or-drill mode, start and completion state, actor, trigger, outcome, before/after state, observations, and clock applicability. | Test (TC-1225) |
| FR-059-AC-5 | Exactly one shape payload is present: standing capability excludes exercise data, and exercise excludes capability data. | Test (TC-1226) |
| FR-059-AC-6 | An `operational_with_clock` exercise requires start and deadline timestamps plus an open, met, missed, or unknown status; a not-applicable clock requires `not_applicable` status. | Test (TC-1227) |
| FR-059-AC-7 | Each policy, prompt, model, tool, or data pin record carries at least one typed identity, revision, and digest. | Test (TC-1228) |
| FR-059-AC-8 | Succeeded, failed, partial, and aborted exercises all validate as retained outcomes. | Test (TC-1229) |
| FR-059-AC-9 | Gaps, owner, actions, and at least one content-digested raw-evidence reference are retained, and undeclared fields are refused. | Test (TC-1230) |

## Dependencies

- **Upstream**: [US-016](../usecase/US-016-assess-operational-controls.md)
  states the practitioner need; [FR-044](./FR-044-plan-governed-measurements.md)
  defines producer provenance and the governed reporting boundary.
- **Downstream**: [FR-060](./FR-060-operational-evidence-intake-report.md)
  governs persistence, clocked obligation discharge, and report interpretation.
