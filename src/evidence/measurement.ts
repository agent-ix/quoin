/**
 * Validation for policy-free measurement observations (FR-043).
 *
 * The JSON Schema is both the published contract and the runtime authority.
 * Keeping the validator here prevents callers from constructing a typed value,
 * serializing `NaN` as `null`, and only discovering the corruption on read.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { Ajv2020 } from "ajv/dist/2020.js";
import type { ErrorObject, ValidateFunction } from "ajv";

import type { MeasurementRecord } from "./types.js";

export const MEASUREMENT_RECORD_SCHEMA = "measurement-record-v1.schema.json";

const here = dirname(fileURLToPath(import.meta.url));
let compiled: ValidateFunction | undefined;

/** Absolute path to the versioned MeasurementRecord JSON Schema. */
export function measurementSchemaPath(): string {
  return join(here, "schemas", MEASUREMENT_RECORD_SCHEMA);
}

/** Read the schema used by the runtime validator. */
export function readMeasurementSchema(): unknown {
  return JSON.parse(readFileSync(measurementSchemaPath(), "utf8"));
}

/** A record that cannot be persisted without losing or confusing its meaning. */
export class MeasurementValidationError extends Error {
  constructor(readonly errors: string[]) {
    super(
      `measurement record does not satisfy ${MEASUREMENT_RECORD_SCHEMA}: ` +
        errors.join("; "),
    );
    this.name = "MeasurementValidationError";
  }
}

function validator(): ValidateFunction {
  if (compiled) return compiled;
  // `strictNumbers` is load-bearing: JSON.stringify silently converts NaN and
  // infinities to null, so the record must reject them before persistence.
  const ajv = new Ajv2020({
    allErrors: true,
    strict: false,
    strictNumbers: true,
  });
  compiled = ajv.compile(readMeasurementSchema() as object);
  return compiled;
}

function describe(errors: ErrorObject[]): string[] {
  return errors.map((error) => {
    const at = error.instancePath === "" ? "<root>" : error.instancePath;
    return `${at}: ${error.message}`;
  });
}

/** Validate without writing, returning the original value when it conforms. */
export function validateMeasurementRecord(value: unknown): MeasurementRecord {
  const validate = validator();
  if (!validate(value)) {
    // Ajv sets `errors` whenever validation returns false.
    throw new MeasurementValidationError(describe(validate.errors!));
  }
  return value as MeasurementRecord;
}
