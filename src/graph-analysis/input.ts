/** Fail-closed file-input contracts for FR-062 graph analysis. */

import { z } from "zod";

import type { AuditReport } from "../auditor/index.js";
import type {
  AcceptedAssurancePremises,
  AssuranceExport,
  AssuranceModulePremise,
  AssuranceSource,
} from "../quire/index.js";

const schemaPremise = z
  .object({
    archetype: z.string().min(1),
    schema_digest: z.string().regex(/^[0-9a-f]{64}$/),
  })
  .strict();

const modulePremise = z
  .object({
    name: z.string().min(1),
    version: z.string().min(1),
    schemas: z.array(schemaPremise),
  })
  .strict();

const acceptedPremises = z
  .object({
    format: z.literal("quire-assurance"),
    format_version: z.literal(1),
    modules: z.array(modulePremise),
  })
  .strict();

const sourceIdentity = z
  .object({
    repository: z.string().min(1),
    revision: z.string().regex(/^[0-9a-f]{40}$/),
  })
  .strict();

// FR-032 remains authoritative over finding kinds and additive action fields.
// The graph consumer validates only the stable join surface and preserves every
// nested object unchanged instead of minting a second audit schema.
const auditFinding = z
  .object({
    obligation: z.string().min(1),
    kind: z.string().min(1),
    severity: z.enum(["low", "medium", "high"]),
    summary: z.string(),
  })
  .passthrough();

const unevaluatedCheck = z
  .object({
    check: z.string().min(1),
    obligation: z.string().min(1),
    suites: z.array(z.string()),
    reason: z.string(),
  })
  .passthrough();

const auditReport = z
  .object({
    findings: z.array(auditFinding),
    healthy: z.array(z.string().min(1)),
    unevaluated: z.array(unevaluatedCheck),
  })
  .passthrough();

const auditEnvelope = z
  .object({
    format: z.literal("quoin-audit-envelope"),
    format_version: z.literal(1),
    source: sourceIdentity,
    export: acceptedPremises,
    report: auditReport,
  })
  .strict();

export interface AuditEnvelope {
  format: "quoin-audit-envelope";
  format_version: 1;
  source: AssuranceSource;
  /** Exact identity copied from the export, not the caller's acceptance set. */
  export: AcceptedAssurancePremises;
  /** Existing FR-032 payload, preserved rather than recomputed. */
  report: AuditReport;
}

export interface GraphInputViolation {
  kind: "graph-input-violation";
  input: "premises" | "audit";
  errors: string[];
  message: string;
}

export type GraphInputResult<T> =
  { ok: true; value: T } | { ok: false; error: GraphInputViolation };

export function parseAcceptedAssurancePremises(
  text: string,
): GraphInputResult<AcceptedAssurancePremises> {
  const parsed = parseWithSchema(text, "premises", acceptedPremises);
  return parsed.ok
    ? { ok: true, value: canonicalizeAcceptedAssurancePremises(parsed.value) }
    : parsed;
}

export function parseAuditEnvelope(
  text: string,
): GraphInputResult<AuditEnvelope> {
  const parsed = parseWithSchema(
    text,
    "audit",
    auditEnvelope,
  ) as GraphInputResult<AuditEnvelope>;
  return parsed.ok
    ? {
        ok: true,
        value: {
          ...parsed.value,
          export: canonicalizeAcceptedAssurancePremises(parsed.value.export),
          report: canonicalizeAuditReport(parsed.value.report),
        },
      }
    : parsed;
}

export function validateAcceptedAssurancePremises(
  exportValue: AssuranceExport,
  accepted: AcceptedAssurancePremises,
): GraphInputViolation | null {
  const actual = canonicalizeAcceptedAssurancePremises({
    format: exportValue.format,
    format_version: exportValue.format_version,
    modules: exportValue.modules,
  });
  const expected = canonicalizeAcceptedAssurancePremises(accepted);
  return sameJson(actual, expected)
    ? null
    : violation(
        "premises",
        "the accepted format, version, modules, and schema digests must exactly match the assurance export",
      );
}

export function validateAuditIdentity(
  audit: AuditEnvelope,
  exportValue: AssuranceExport,
): GraphInputViolation | null {
  if (!sameJson(audit.source, exportValue.source)) {
    return violation(
      "audit",
      "the audit source repository/revision does not match the assurance export",
    );
  }
  const exportIdentity: AcceptedAssurancePremises = {
    format: exportValue.format,
    format_version: exportValue.format_version,
    modules: exportValue.modules,
  };
  if (
    !sameJson(
      canonicalizeAcceptedAssurancePremises(audit.export),
      canonicalizeAcceptedAssurancePremises(exportIdentity),
    )
  ) {
    return violation(
      "audit",
      "the audit export format/module premises do not match the assurance export",
    );
  }
  return null;
}

export function canonicalizeAcceptedAssurancePremises(
  value: AcceptedAssurancePremises,
): AcceptedAssurancePremises {
  return {
    ...value,
    modules: value.modules
      .map((module) => ({
        ...module,
        schemas: module.schemas
          .map((schema) => ({ ...schema }))
          .sort(
            (left, right) =>
              compare(left.archetype, right.archetype) ||
              compare(left.schema_digest, right.schema_digest),
          ),
      }))
      .sort(
        (left, right) =>
          compare(left.name, right.name) ||
          compare(left.version, right.version),
      ),
  };
}

export function canonicalizeAuditReport(report: AuditReport): AuditReport {
  return {
    ...report,
    findings: report.findings
      .map((finding) => ({ ...finding }))
      .sort((left, right) => compare(stableKey(left), stableKey(right))),
    healthy: [...report.healthy].sort(compare),
    unevaluated: report.unevaluated
      .map((check) => ({ ...check, suites: [...check.suites].sort(compare) }))
      .sort((left, right) => compare(stableKey(left), stableKey(right))),
  };
}

function parseWithSchema<T>(
  text: string,
  input: GraphInputViolation["input"],
  schema: z.ZodType<T>,
): GraphInputResult<T> {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (cause) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    return { ok: false, error: violation(input, `invalid JSON: ${detail}`) };
  }
  const parsed = schema.safeParse(value);
  if (!parsed.success) {
    const errors = parsed.error.issues.map(
      (issue) => `${issue.path.join(".") || "<root>"}: ${issue.message}`,
    );
    return {
      ok: false,
      error: {
        kind: "graph-input-violation",
        input,
        errors,
        message: `${input} input is invalid (${errors.join("; ")})`,
      },
    };
  }
  return { ok: true, value: parsed.data };
}

function violation(
  input: GraphInputViolation["input"],
  detail: string,
): GraphInputViolation {
  return {
    kind: "graph-input-violation",
    input,
    errors: [detail],
    message: `${input} input is invalid: ${detail}`,
  };
}

function sameJson(
  left: AssuranceSource | AcceptedAssurancePremises,
  right: AssuranceSource | AcceptedAssurancePremises,
): boolean {
  return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right));
}

function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value === null || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => (left === right ? 0 : left < right ? -1 : 1))
      .map(([key, child]) => [key, canonical(child)]),
  );
}

function stableKey(value: unknown): string {
  return JSON.stringify(canonical(value));
}

function compare(left: string, right: string): number {
  return left === right ? 0 : left < right ? -1 : 1;
}

/** Exported for report types without exposing the zod schemas. */
export type { AssuranceModulePremise };
