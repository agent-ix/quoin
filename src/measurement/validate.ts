import type {
  MeasurementCollection,
  MeasurementObservation,
  MeasurementPlan,
} from "./types.js";
import { MEASUREMENT_SCHEMA_VERSION } from "./types.js";

export class MeasurementValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MeasurementValidationError";
  }
}

/** Validate the typed envelope and require an active, version-matching plan. */
export function validateMeasurementCollection(
  value: unknown,
  plans: MeasurementPlan[],
): asserts value is MeasurementCollection {
  validateStoredMeasurementCollection(value);

  const byMetric = new Map(plans.map((plan) => [plan.metric, plan]));
  for (const observation of value.observations) {
    const plan = byMetric.get(observation.metric);
    if (!plan) {
      fail(
        `metric \`${observation.metric}\` has no MeasurementPlan under spec/assurance; record refused`,
      );
    }
    if (plan.status !== "active") {
      fail(
        `metric \`${observation.metric}\` plan ${plan.id} is ${plan.status}, not active`,
      );
    }
    if (observation.planId !== plan.id) {
      fail(
        `metric \`${observation.metric}\` names plan ${observation.planId}; active plan is ${plan.id}`,
      );
    }
    if (observation.definitionVersion !== plan.definitionVersion) {
      fail(
        `metric \`${observation.metric}\` definition ${observation.definitionVersion} does not match ${plan.definitionVersion}`,
      );
    }
  }
}

/** Validate a historical stored envelope without rewriting it to the current plan. */
export function validateStoredMeasurementCollection(
  value: unknown,
): asserts value is MeasurementCollection {
  if (!isObject(value)) fail("collection must be an object");
  if (value.schemaVersion !== MEASUREMENT_SCHEMA_VERSION) {
    fail(`schemaVersion must be ${MEASUREMENT_SCHEMA_VERSION}`);
  }
  for (const key of [
    "collectionId",
    "subject",
    "toolIdentity",
    "toolVersion",
    "configDigest",
    "timestamp",
    "sourceRevision",
  ]) {
    if (typeof value[key] !== "string" || value[key].length === 0) {
      fail(`collection requires non-empty \`${key}\``);
    }
  }
  if (!Array.isArray(value.observations) || value.observations.length === 0) {
    fail("collection requires at least one observation");
  }
  if (!isObject(value.environment)) fail("environment must be an object");
  if (!("scope" in value)) fail("collection requires `scope`");
  if (!("rawEvidence" in value))
    fail("collection requires attached `rawEvidence`");

  const seen = new Set<string>();
  for (const observation of value.observations) {
    validateObservation(observation);
    const key = `${observation.metric}\0${dimensionsKey(observation)}`;
    if (seen.has(key))
      fail(
        `duplicate observation for \`${observation.metric}\` and its dimensions`,
      );
    seen.add(key);
  }
}

function validateObservation(
  value: unknown,
): asserts value is MeasurementObservation {
  if (!isObject(value)) fail("observation must be an object");
  for (const key of [
    "metric",
    "planId",
    "definitionVersion",
    "unit",
    "shape",
    "state",
  ]) {
    if (typeof value[key] !== "string" || value[key].length === 0) {
      fail(`observation requires non-empty \`${key}\``);
    }
  }
  if (!new Set(["scalar", "ratio", "count"]).has(value.shape as string)) {
    fail(`observation \`${String(value.metric)}\` has invalid shape`);
  }
  if (!new Set(["measured", "not_computed"]).has(value.state as string)) {
    fail(`observation \`${String(value.metric)}\` has invalid state`);
  }
  if (value.state === "measured" && typeof value.value !== "number") {
    fail(
      `measured observation \`${String(value.metric)}\` requires a numeric value`,
    );
  }
  if (value.state === "not_computed" && value.value !== null) {
    fail(
      `not_computed observation \`${String(value.metric)}\` requires null value`,
    );
  }
}

function dimensionsKey(observation: MeasurementObservation): string {
  return JSON.stringify(
    Object.entries(observation.dimensions ?? {}).sort(([a], [b]) =>
      a === b ? 0 : a < b ? -1 : 1,
    ),
  );
}

function isObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function fail(message: string): never {
  throw new MeasurementValidationError(message);
}
