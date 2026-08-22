/** Profile-selected evidence lineage and independence policy (FR-047). */

import { readFileSync } from "node:fs";

import { z } from "zod";

import type {
  Binding,
  EvidenceLineage,
  IndependenceAssessment,
  IndependenceDimension,
  IndependencePolicy,
  IndependenceRequirement,
} from "./types.js";

const IndependenceDimensionSchema = z.enum([
  "actor",
  "implementation-toolchain",
  "technique",
  "data-source",
  "review-path",
]);

export const EvidenceLineageSchema = z
  .object({
    actor: z.string().trim().min(1).optional(),
    implementationToolchain: z.string().trim().min(1).optional(),
    technique: z.string().trim().min(1).optional(),
    dataSource: z.string().trim().min(1).optional(),
    reviewPath: z.string().trim().min(1).optional(),
  })
  .strict()
  .refine((lineage) => Object.keys(lineage).length > 0, {
    message: "must name at least one lineage dimension",
  });

const IndependenceRequirementSchema = z
  .object({
    id: z.string().trim().min(1),
    obligation: z.string().trim().min(1),
    dimensions: z.array(IndependenceDimensionSchema).min(1),
    rationale: z.string().trim().min(1),
  })
  .strict()
  .superRefine((requirement, context) => {
    if (
      new Set(requirement.dimensions).size !== requirement.dimensions.length
    ) {
      context.addIssue({
        code: "custom",
        path: ["dimensions"],
        message: "contains duplicate dimensions",
      });
    }
  });

export const IndependencePolicySchema = z
  .object({
    schemaVersion: z.literal(1),
    profile: z
      .string()
      .trim()
      .regex(/^AP-[0-9]+$/),
    requirements: z.array(IndependenceRequirementSchema).min(1),
  })
  .strict()
  .superRefine((policy, context) => {
    const ids = new Set<string>();
    const obligations = new Set<string>();
    for (const [index, requirement] of policy.requirements.entries()) {
      if (ids.has(requirement.id)) {
        context.addIssue({
          code: "custom",
          path: ["requirements", index, "id"],
          message: "duplicates another requirement id",
        });
      }
      if (obligations.has(requirement.obligation)) {
        context.addIssue({
          code: "custom",
          path: ["requirements", index, "obligation"],
          message: "duplicates another obligation requirement",
        });
      }
      ids.add(requirement.id);
      obligations.add(requirement.obligation);
    }
  });

export function validateEvidenceLineage(value: unknown): EvidenceLineage {
  return parse(EvidenceLineageSchema, value, "evidence lineage");
}

export function validateIndependencePolicy(value: unknown): IndependencePolicy {
  return parse(IndependencePolicySchema, value, "independence policy");
}

export function readEvidenceLineage(path: string): EvidenceLineage {
  return validateEvidenceLineage(readJson(path, "evidence lineage"));
}

export function readIndependencePolicy(path: string): IndependencePolicy {
  return validateIndependencePolicy(readJson(path, "independence policy"));
}

/** Refuse a stale/typoed projection rather than silently evaluating nothing. */
export function requireKnownPolicyObligations(
  policy: IndependencePolicy,
  knownObligations: Iterable<string>,
): void {
  const known = new Set(knownObligations);
  const unknown = policy.requirements
    .map((requirement) => requirement.obligation)
    .filter((obligation) => !known.has(obligation))
    .sort(compare);
  if (unknown.length > 0) {
    throw new Error(
      `independence policy names unknown obligation(s): ${unknown.join(", ")}`,
    );
  }
}

/**
 * Find two distinct evidence relationships separated on every selected axis.
 *
 * A dimension absent on either side cannot demonstrate separation. Two suites
 * sharing one model/parser/corpus therefore remain visibly common-mode when the
 * profile selects the corresponding dimension.
 */
export function assessIndependence(
  profile: string,
  requirement: IndependenceRequirement,
  bindings: Binding[],
): IndependenceAssessment {
  const ordered = [...bindings].sort((a, b) => compare(a.suite, b.suite));
  const dimensions = requirement.dimensions.map((dimension) => {
    const values = new Set<string>();
    const missingSuites: string[] = [];
    for (const binding of ordered) {
      const value = valueFor(binding.lineage, dimension);
      if (value === undefined) missingSuites.push(binding.suite);
      else values.add(value);
    }
    return {
      dimension,
      values: [...values].sort(compare),
      missingSuites,
    };
  });

  for (let left = 0; left < ordered.length; left += 1) {
    for (let right = left + 1; right < ordered.length; right += 1) {
      const a = ordered[left];
      const b = ordered[right];
      if (
        a.suite !== b.suite &&
        requirement.dimensions.every((dimension) => {
          const first = valueFor(a.lineage, dimension);
          const second = valueFor(b.lineage, dimension);
          return (
            first !== undefined && second !== undefined && first !== second
          );
        })
      ) {
        return {
          profile,
          requirement: requirement.id,
          obligation: requirement.obligation,
          status: "satisfied",
          dimensions,
          satisfiedBy: [a.suite, b.suite],
          summary:
            `${requirement.id} is satisfied by ${a.suite} and ${b.suite}; ` +
            `they differ on ${requirement.dimensions.join(", ")}.`,
        };
      }
    }
  }

  const detail = dimensions
    .map((item) => {
      const missing =
        item.missingSuites.length === 0
          ? ""
          : `; missing on ${item.missingSuites.join(", ")}`;
      return `${item.dimension}: ${item.values.length} distinct${missing}`;
    })
    .join("; ");
  return {
    profile,
    requirement: requirement.id,
    obligation: requirement.obligation,
    status: "insufficient",
    dimensions,
    summary:
      `${requirement.id} requires two evidence lines differing on every selected ` +
      `dimension (${requirement.dimensions.join(", ")}); ${detail}.`,
  };
}

function valueFor(
  lineage: EvidenceLineage | undefined,
  dimension: IndependenceDimension,
): string | undefined {
  let value: unknown;
  switch (dimension) {
    case "actor":
      value = lineage?.actor;
      break;
    case "implementation-toolchain":
      value = lineage?.implementationToolchain;
      break;
    case "technique":
      value = lineage?.technique;
      break;
    case "data-source":
      value = lineage?.dataSource;
      break;
    case "review-path":
      value = lineage?.reviewPath;
      break;
  }
  if (typeof value !== "string" || value.trim().length === 0) return undefined;
  return value.trim();
}

function readJson(path: string, label: string): unknown {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (cause) {
    throw new Error(
      `${label} at ${path} is not readable JSON: ` +
        (cause instanceof Error ? cause.message : String(cause)),
    );
  }
}

function parse<T>(schema: z.ZodType<T>, value: unknown, label: string): T {
  const parsed = schema.safeParse(value);
  if (parsed.success) return parsed.data;
  const detail = parsed.error.issues
    .map((issue) => `${issue.path.join(".") || "record"}: ${issue.message}`)
    .join("; ");
  throw new Error(`invalid ${label}: ${detail}`);
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
