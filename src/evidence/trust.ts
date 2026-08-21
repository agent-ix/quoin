/** Use-specific evidence-producer trust decisions (FR-046). */

import { z } from "zod";

import type {
  ProducerContext,
  TrustAssessment,
  TrustDecision,
  TrustTrigger,
} from "./types.js";

const TrustTriggerSchema = z.enum([
  "producer-version",
  "configuration",
  "adapter",
  "validation-corpus",
  "input-contract",
  "environment",
]);

const REQUIRED_TRIGGERS: readonly TrustTrigger[] = [
  "producer-version",
  "configuration",
  "validation-corpus",
  "input-contract",
  "environment",
];

const ProducerContextSchema = z
  .object({
    name: z.string().min(1),
    version: z.string().min(1),
    configurationDigest: z.string().min(1),
    validationCorpusDigest: z.string().min(1),
    inputContract: z.string().min(1),
    environment: z.string().min(1),
    adapter: z
      .object({ name: z.string().min(1), version: z.string().min(1) })
      .strict()
      .optional(),
  })
  .strict();

/** Strict runtime boundary for machine-written trust-decision JSON. */
export const TrustDecisionSchema = z
  .object({
    schemaVersion: z.literal(1),
    id: z.string().regex(/^ETD-[0-9]+$/),
    use: z
      .object({
        id: z.string().min(1),
        intendedFunction: z.string().min(1),
        permittedDecisions: z.array(z.string().min(1)).min(1),
      })
      .strict(),
    decision: z.enum(["relied-upon", "not-relied-upon"]),
    acceptedContext: ProducerContextSchema,
    observedContext: ProducerContextSchema.optional(),
    revalidateOn: z.array(TrustTriggerSchema).min(1),
    validationEvidence: z
      .array(
        z
          .object({
            id: z.string().min(1),
            digest: z
              .string()
              .regex(/^[a-z0-9]+:[a-f0-9]+$/)
              .optional(),
            reference: z.string().min(1).optional(),
          })
          .strict()
          .refine(
            (item) => item.digest !== undefined || item.reference !== undefined,
            { message: "validation evidence needs digest or reference" },
          ),
      )
      .min(1, { message: "must not be empty" }),
    limitations: z.array(z.string().min(1)),
    owner: z.string().min(1),
    decidedAt: z.string().refine((value) => !Number.isNaN(Date.parse(value)), {
      message: "must be an ISO-8601 timestamp",
    }),
  })
  .strict()
  .superRefine((decision, context) => {
    const uniqueTriggers = new Set(decision.revalidateOn);
    if (uniqueTriggers.size !== decision.revalidateOn.length) {
      context.addIssue({
        code: "custom",
        path: ["revalidateOn"],
        message: "contains duplicate triggers",
      });
    }
    if (
      new Set(decision.use.permittedDecisions).size !==
      decision.use.permittedDecisions.length
    ) {
      context.addIssue({
        code: "custom",
        path: ["use", "permittedDecisions"],
        message: "contains duplicate decisions",
      });
    }
    for (const required of REQUIRED_TRIGGERS) {
      if (!uniqueTriggers.has(required)) {
        context.addIssue({
          code: "custom",
          path: ["revalidateOn"],
          message: `must include ${required}`,
        });
      }
    }
    if (decision.acceptedContext.adapter && !uniqueTriggers.has("adapter")) {
      context.addIssue({
        code: "custom",
        path: ["revalidateOn"],
        message: "must include adapter when an adapter is relied upon",
      });
    }
  });

export function validateTrustDecision(value: unknown): TrustDecision {
  const parsed = TrustDecisionSchema.safeParse(value);
  if (!parsed.success) {
    const detail = parsed.error.issues
      .map((issue) => `${issue.path.join(".") || "record"}: ${issue.message}`)
      .join("; ");
    throw new Error(`invalid trust decision: ${detail}`);
  }
  return parsed.data;
}

export function assessTrust(raw: TrustDecision): TrustAssessment {
  const decision = validateTrustDecision(raw);
  const common = {
    id: decision.id,
    useId: decision.use.id,
    producer: decision.acceptedContext.name,
    permittedDecisions: [...decision.use.permittedDecisions].sort(),
    limitations: [...decision.limitations],
    owner: decision.owner,
  };
  if (decision.decision === "not-relied-upon") {
    return { ...common, status: "not-accepted", triggeredBy: [] };
  }
  if (!decision.observedContext) {
    return { ...common, status: "unobserved", triggeredBy: [] };
  }
  const triggeredBy = decision.revalidateOn.filter((trigger) =>
    differs(trigger, decision.acceptedContext, decision.observedContext!),
  );
  if (triggeredBy.length > 0) {
    return { ...common, status: "invalidated", triggeredBy };
  }
  return {
    ...common,
    status:
      decision.limitations.length > 0
        ? "accepted-with-limitations"
        : "accepted",
    triggeredBy: [],
  };
}

function differs(
  trigger: TrustTrigger,
  accepted: ProducerContext,
  observed: ProducerContext,
): boolean {
  switch (trigger) {
    case "producer-version":
      return (
        accepted.name !== observed.name || accepted.version !== observed.version
      );
    case "configuration":
      return accepted.configurationDigest !== observed.configurationDigest;
    case "adapter":
      return (
        JSON.stringify(accepted.adapter) !== JSON.stringify(observed.adapter)
      );
    case "validation-corpus":
      return (
        accepted.validationCorpusDigest !== observed.validationCorpusDigest
      );
    case "input-contract":
      return accepted.inputContract !== observed.inputContract;
    case "environment":
      return accepted.environment !== observed.environment;
  }
}
