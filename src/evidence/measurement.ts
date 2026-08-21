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

import type { MeasurementCollection, MeasurementRecord } from "./types.js";

export const MEASUREMENT_RECORD_SCHEMA = "measurement-record-v1.schema.json";
export const MEASUREMENT_COLLECTION_SCHEMA =
  "measurement-collection-v1.schema.json";

const here = dirname(fileURLToPath(import.meta.url));
let compiled: ValidateFunction | undefined;
let compiledCollection: ValidateFunction | undefined;

/** Absolute path to the versioned MeasurementRecord JSON Schema. */
export function measurementSchemaPath(): string {
  return join(here, "schemas", MEASUREMENT_RECORD_SCHEMA);
}

/** Read the schema used by the runtime validator. */
export function readMeasurementSchema(): unknown {
  return JSON.parse(readFileSync(measurementSchemaPath(), "utf8"));
}

/** Read the physical collection schema used by the store validator. */
export function readMeasurementCollectionSchema(): unknown {
  return JSON.parse(
    readFileSync(join(here, "schemas", MEASUREMENT_COLLECTION_SCHEMA), "utf8"),
  );
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

function collectionValidator(): ValidateFunction {
  if (compiledCollection) return compiledCollection;
  const ajv = new Ajv2020({
    allErrors: true,
    strict: false,
    strictNumbers: true,
  });
  ajv.addSchema(readMeasurementSchema() as object);
  compiledCollection = ajv.compile(readMeasurementCollectionSchema() as object);
  return compiledCollection;
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

/**
 * Validate a physical collection against the published envelope contract.
 * Collection-only identity invariants are checked here so the
 * on-disk envelope cannot hide duplicates or an empty/partial population.
 */
export function validateMeasurementCollection(
  value: unknown,
): MeasurementCollection {
  const validate = collectionValidator();
  if (!validate(value)) {
    throw new MeasurementValidationError([
      `${MEASUREMENT_COLLECTION_SCHEMA}: ${describe(validate.errors!).join("; ")}`,
    ]);
  }
  const collection = value as MeasurementCollection;
  const seen = new Set<string>();
  for (const observation of collection.observations) {
    const key = JSON.stringify({
      scope: {
        repository: collection.repository,
        path: observation.path ?? null,
      },
      subject: observation.subject,
    });
    if (seen.has(key)) {
      throw new MeasurementValidationError([
        `/observations: duplicate subject/scope identity ${observation.subject.kind}:${observation.subject.id}`,
      ]);
    }
    seen.add(key);
  }
  return collection;
}

/** Materialize the logical records used by generic query/comparison code. */
export function measurementRecordsFromCollection(
  collection: MeasurementCollection,
): MeasurementRecord[] {
  return collection.observations.map((observation) => ({
    schemaVersion: collection.schemaVersion,
    plan: collection.plan,
    subject: observation.subject,
    scope: {
      repository: collection.repository,
      ...(observation.path === undefined ? {} : { path: observation.path }),
    },
    sourceRevision: collection.sourceRevision,
    value: observation.value,
    unit: observation.unit,
    ...(observation.distribution === undefined
      ? {}
      : { distribution: observation.distribution }),
    tool: collection.tool,
    environment: collection.environment,
    ...(collection.sampling === undefined
      ? {}
      : { sampling: collection.sampling }),
    collectedAt: collection.collectedAt,
    rawEvidence: collection.rawEvidence,
  }));
}
