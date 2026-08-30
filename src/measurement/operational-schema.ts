const identity = {
  type: "string",
  pattern: "^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$",
};
const digest = { type: "string", pattern: "^(sha256|blake3):[a-f0-9]{64}$" };
const immutable = {
  type: "string",
  pattern:
    "^(v?[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|(sha256|blake3):[a-f0-9]{64})$",
};
const nonempty = { type: "string", minLength: 1 };
const scalar = {
  anyOf: [
    { type: "string" },
    { type: "number" },
    { type: "boolean" },
    { type: "null" },
  ],
};
const closed = (required: string[], properties: Record<string, unknown>) => ({
  type: "object",
  required,
  properties,
  additionalProperties: false,
});
const clock = closed(["applicability", "status"], {
  applicability: {
    type: "string",
    enum: ["operational_with_clock", "not_applicable"],
  },
  started_at: nonempty,
  deadline_at: nonempty,
  completed_at: nonempty,
  status: {
    type: "string",
    enum: ["not_applicable", "open", "met", "missed", "unknown"],
  },
});

export const operationalEvidenceSchema = {
  $schema: "https://json-schema.org/draft/2020-12/schema",
  type: "object",
  required: [
    "schema_version",
    "record_type",
    "record_id",
    "observed_at",
    "record_shape",
    "control_kind",
    "subject",
    "producer",
    "scope",
    "configuration",
    "owner",
    "gaps",
    "actions",
    "raw_evidence",
  ],
  properties: {
    schema_version: { const: 1 },
    record_type: { const: "operational_evidence" },
    record_id: identity,
    observed_at: nonempty,
    record_shape: { type: "string", enum: ["standing_capability", "exercise"] },
    control_kind: {
      type: "string",
      enum: [
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
        "reporting",
      ],
    },
    subject: closed(["id", "revision"], { id: identity, revision: nonempty }),
    producer: closed(
      [
        "tool_identity",
        "tool_version",
        "configuration_digest",
        "source_revision",
        "environment",
        "definition_version",
      ],
      {
        tool_identity: nonempty,
        tool_version: immutable,
        configuration_digest: digest,
        source_revision: nonempty,
        environment: {
          type: "object",
          minProperties: 1,
          additionalProperties: scalar,
        },
        definition_version: nonempty,
      },
    ),
    scope: closed(["service", "environment", "population"], {
      service: identity,
      environment: nonempty,
      population: nonempty,
    }),
    configuration: closed(["version_pins"], {
      version_pins: {
        type: "array",
        items: closed(["kind", "identity", "revision", "digest"], {
          kind: {
            type: "string",
            enum: ["policy", "prompt", "model", "tool", "data"],
          },
          identity,
          revision: nonempty,
          digest,
        }),
      },
    }),
    capability: closed(
      [
        "control_id",
        "status",
        "surface",
        "authorized_roles",
        "coverage",
        "limitations",
        "supported_transitions",
        "clock_support",
      ],
      {
        control_id: identity,
        status: {
          type: "string",
          enum: ["available", "unavailable", "unknown", "not_applicable"],
        },
        surface: nonempty,
        authorized_roles: { type: "array", minItems: 1, items: nonempty },
        coverage: nonempty,
        limitations: { type: "array", items: nonempty },
        supported_transitions: { type: "array", minItems: 1, items: nonempty },
        clock_support: closed(["supported"], {
          supported: { type: "boolean" },
          start_event: nonempty,
          completion_event: nonempty,
          deadline_seconds: { type: "integer", minimum: 1 },
        }),
      },
    ),
    exercise: closed(
      [
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
        "clock",
      ],
      {
        control_id: identity,
        capability_record_id: identity,
        mode: { type: "string", enum: ["actual", "drill"] },
        started_at: nonempty,
        completed_at: nonempty,
        actor: nonempty,
        trigger: nonempty,
        outcome: {
          type: "string",
          enum: ["succeeded", "failed", "partial", "aborted"],
        },
        state_before: { type: "object" },
        state_after: { type: "object" },
        observations: { type: "array", minItems: 1, items: nonempty },
        clock,
      },
    ),
    owner: nonempty,
    gaps: { type: "array", items: nonempty },
    actions: { type: "array", minItems: 1, items: nonempty },
    raw_evidence: {
      type: "array",
      minItems: 1,
      items: closed(["path", "media_type", "size_bytes", "digest"], {
        path: nonempty,
        media_type: nonempty,
        size_bytes: { type: "integer", minimum: 0 },
        digest,
      }),
    },
  },
  additionalProperties: false,
} as const;
