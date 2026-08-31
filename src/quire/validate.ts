/**
 * Validate quire JSON payloads against the published schemas (FR-029).
 *
 * This is the first quoin code to parse quire output. Before it, both shapes
 * lived as prose in skill markdown and were consumed by agents reading them —
 * so when one drifted, the failure surfaced as an agent confused mid-skill
 * rather than as a named diagnostic. `spec/review.md` Finding 8 recorded
 * exactly that: "no contract test against quire".
 */

import { Ajv2020 } from "ajv/dist/2020.js";
import type { ErrorObject, ValidateFunction } from "ajv";

import { readSchema, type SchemaName } from "./contract.js";
import type { AssuranceExport } from "./assurance.js";
import type { CoverageReport, PropertiesReport } from "./types.js";

/** A payload that did not satisfy the published contract. */
export interface ContractViolation {
  readonly kind: "contract-violation";
  readonly schema: SchemaName;
  /** One line per failing path, in the order ajv reported them. */
  readonly errors: string[];
  readonly message: string;
}

export type ValidationResult<T> =
  { ok: true; value: T } | { ok: false; error: ContractViolation };

/**
 * Compiled once per schema, per process.
 *
 * Compilation is the expensive half and the schemas never change at runtime,
 * so a validator that recompiled per call would put schema compilation on the
 * path of every payload a batch reads.
 */
const compiled = new Map<SchemaName, ValidateFunction>();

function validatorFor(name: SchemaName): ValidateFunction {
  const existing = compiled.get(name);
  if (existing) return existing;
  // `strict: false` because the published schemas are authored for
  // interoperability, not for ajv: they carry `description` on every node and
  // ajv's strict mode objects to keywords it considers unknown in context.
  // Validation semantics are unaffected.
  const ajv = new Ajv2020({ allErrors: true, strict: false });
  const validate = ajv.compile(readSchema(name) as object);
  compiled.set(name, validate);
  return validate;
}

function describe(errors: ErrorObject[] | null | undefined): string[] {
  return (errors ?? []).map((e) => {
    const at = e.instancePath === "" ? "<root>" : e.instancePath;
    return `${at}: ${e.message ?? "invalid"}`;
  });
}

function validate<T>(name: SchemaName, payload: unknown): ValidationResult<T> {
  const validator = validatorFor(name);
  if (validator(payload)) {
    return { ok: true, value: payload as T };
  }
  const errors = describe(validator.errors);
  return {
    ok: false,
    error: {
      kind: "contract-violation",
      schema: name,
      errors,
      message:
        `the quire payload does not satisfy the published ${name} contract ` +
        `(${errors.length} error${errors.length === 1 ? "" : "s"}). This is a ` +
        `shape drift between the quire release in use and the contract quoin ` +
        `pins, not a defect in the document being read.`,
    },
  };
}

/** Validate a `quire coverage --json` payload. */
export function validateCoverage(
  payload: unknown,
): ValidationResult<CoverageReport> {
  return validate<CoverageReport>("coverage-v1.schema.json", payload);
}

/** Validate a `quire properties --json` payload. */
export function validateProperties(
  payload: unknown,
): ValidationResult<PropertiesReport> {
  return validate<PropertiesReport>("properties-v1.schema.json", payload);
}

/** Validate a supplied source-grounded assurance-v1 export. */
export function validateAssurance(
  payload: unknown,
): ValidationResult<AssuranceExport> {
  return validate<AssuranceExport>("assurance-v1.schema.json", payload);
}

/**
 * Parse and validate in one step, so a caller never holds an unvalidated
 * payload it could accidentally use.
 *
 * A JSON parse failure is reported as a contract violation rather than thrown:
 * from the caller's position "quire emitted something unreadable" and "quire
 * emitted the wrong shape" are the same actionable fact — the tool's output
 * cannot be consumed — and both want the same named diagnostic.
 */
export function parseCoverage(text: string): ValidationResult<CoverageReport> {
  return parseThen(text, "coverage-v1.schema.json", validateCoverage);
}

/** As {@link parseCoverage}, for the properties payload. */
export function parseProperties(
  text: string,
): ValidationResult<PropertiesReport> {
  return parseThen(text, "properties-v1.schema.json", validateProperties);
}

/** As {@link parseCoverage}, for an existing assurance-export artifact. */
export function parseAssurance(
  text: string,
): ValidationResult<AssuranceExport> {
  return parseThen(text, "assurance-v1.schema.json", validateAssurance);
}

function parseThen<T>(
  text: string,
  schema: SchemaName,
  then: (payload: unknown) => ValidationResult<T>,
): ValidationResult<T> {
  let payload: unknown;
  try {
    payload = JSON.parse(text);
  } catch (cause) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    return {
      ok: false,
      error: {
        kind: "contract-violation",
        schema,
        errors: [`<root>: ${detail}`],
        message:
          `the quire output is not valid JSON, so the ${schema} contract could ` +
          `not be checked. Run the command directly to see what it emitted — ` +
          `a diagnostic on stdout rather than stderr is the usual cause.`,
      },
    };
  }
  return then(payload);
}
