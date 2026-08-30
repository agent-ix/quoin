const identity = {
  type: "string",
  pattern: "^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$",
};
const digest = { type: "string", pattern: "^(sha256|blake3):[a-f0-9]{64}$" };
const immutableVersion = {
  type: "string",
  pattern:
    "^(v?[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|(sha256|blake3):[a-f0-9]{64})$",
};
const nonempty = { type: "string", minLength: 1 };
const scalar = {
  anyOf: [
    { type: "number" },
    { type: "string" },
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
const arm = closed(["id", "population", "sample_size", "configuration"], {
  id: identity,
  population: nonempty,
  sample_size: { type: "integer", minimum: 0 },
  configuration: { type: "object" },
});
const qualified = closed(["description", "disposition"], {
  description: nonempty,
  disposition: {
    type: "string",
    enum: ["controlled", "uncontrolled", "unknown", "not_applicable"],
  },
});

export const interventionExperimentSchema = {
  $schema: "https://json-schema.org/draft/2020-12/schema",
  type: "object",
  required: [
    "schema_version",
    "record_type",
    "record_id",
    "observed_at",
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
    "raw_evidence",
  ],
  properties: {
    schema_version: { const: 1 },
    record_type: { const: "intervention_experiment" },
    record_id: identity,
    observed_at: nonempty,
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
        tool_version: immutableVersion,
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
    design: closed(
      ["kind", "repetitions", "assignment", "sampling_conditions"],
      {
        kind: { type: "string", enum: ["repeated", "randomized", "factorial"] },
        repetitions: { type: "integer", minimum: 1 },
        assignment: closed(["method"], {
          method: {
            type: "string",
            enum: [
              "not_applicable",
              "deterministic",
              "randomized",
              "blocked_randomized",
            ],
          },
          seed: nonempty,
        }),
        sampling_conditions: { type: "array", minItems: 1, items: nonempty },
      },
    ),
    baseline: arm,
    treatments: { type: "array", minItems: 1, items: arm },
    changed_variables: {
      type: "array",
      minItems: 1,
      items: closed(
        ["name", "treatment_id", "baseline_value", "treatment_value"],
        {
          name: nonempty,
          treatment_id: identity,
          baseline_value: {},
          treatment_value: {},
        },
      ),
    },
    held_constant: {
      type: "array",
      items: closed(["name", "value"], { name: nonempty, value: {} }),
    },
    measured_effects: {
      type: "array",
      items: closed(
        [
          "treatment_id",
          "metric",
          "baseline_value",
          "treatment_value",
          "effect",
          "unit",
        ],
        {
          treatment_id: identity,
          metric: identity,
          baseline_value: scalar,
          treatment_value: scalar,
          effect: {
            anyOf: [{ type: "number" }, { type: "string" }, { type: "null" }],
          },
          unit: nonempty,
        },
      ),
    },
    interactions: { type: "array", items: qualified },
    confounders: { type: "array", items: qualified },
    status: { type: "string", enum: ["completed", "failed", "inconclusive"] },
    conclusion: closed(["kind", "statement", "attribution_confidence"], {
      kind: {
        type: "string",
        enum: [
          "causal_effect_established",
          "no_effect_observed",
          "cause_not_established",
        ],
      },
      statement: nonempty,
      attribution_confidence: {
        type: "string",
        enum: ["none", "low", "moderate", "high"],
      },
    }),
    gaps: { type: "array", items: nonempty },
    owner: nonempty,
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
