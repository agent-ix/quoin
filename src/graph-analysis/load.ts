/** Read the three explicit FR-062 inputs plus retained Quoin bindings. */

import { existsSync, readFileSync } from "node:fs";

import { z } from "zod";

import { bindingsPath, STORE_SCHEMA_VERSION } from "../evidence/index.js";
import type { Binding } from "../evidence/index.js";
import { parseAssurance } from "../quire/index.js";
import type { GraphAnalysisInput } from "./analysis.js";
import {
  parseAcceptedAssurancePremises,
  parseAuditEnvelope,
  validateAcceptedAssurancePremises,
  validateAuditIdentity,
} from "./input.js";

export interface GraphLoadOptions {
  repo: string;
  exportPath: string;
  premisesPath: string;
  auditPath: string;
}

export interface GraphLoadFailure {
  kind: "graph-input-read" | "graph-input-invalid";
  input: "export" | "premises" | "audit";
  message: string;
}

export type GraphLoadResult =
  | { ok: true; value: GraphAnalysisInput }
  | { ok: false; error: GraphLoadFailure };

const affirmation = z
  .object({
    who: z.string(),
    commit: z.string(),
    note: z.string().optional(),
  })
  .passthrough();

const binding = z
  .object({
    obligation: z.string().min(1),
    statementHashAtBinding: z.string().min(1),
    suite: z.string().min(1),
    commit: z.string().min(1),
    symbols: z.array(z.string()),
    affirmations: z.array(affirmation).optional(),
  })
  .passthrough();

const bindingsFile = z
  .object({
    schemaVersion: z.literal(STORE_SCHEMA_VERSION),
    bindings: z.array(binding),
  })
  .passthrough();

export function loadGraphAnalysisInput(
  options: GraphLoadOptions,
): GraphLoadResult {
  const exportText = readRequired(options.exportPath, "export");
  if (!exportText.ok) return exportText;
  const parsedExport = parseAssurance(exportText.value);
  if (!parsedExport.ok) {
    return invalid("export", parsedExport.error.message);
  }

  const premisesText = readRequired(options.premisesPath, "premises");
  if (!premisesText.ok) return premisesText;
  const premises = parseAcceptedAssurancePremises(premisesText.value);
  if (!premises.ok) return invalid("premises", premises.error.message);
  const rejected = validateAcceptedAssurancePremises(
    parsedExport.value,
    premises.value,
  );
  if (rejected) return invalid("premises", rejected.message);

  const auditText = readRequired(options.auditPath, "audit");
  if (!auditText.ok) return auditText;
  const audit = parseAuditEnvelope(auditText.value);
  if (!audit.ok) return invalid("audit", audit.error.message);
  const mismatched = validateAuditIdentity(audit.value, parsedExport.value);
  if (mismatched) return invalid("audit", mismatched.message);

  const path = bindingsPath(options.repo);
  let bindings: GraphAnalysisInput["bindings"];
  if (!existsSync(path)) {
    bindings = {
      availability: "absent",
      reason: `${path} is absent`,
    };
  } else {
    try {
      // Read the retained bytes directly at this stricter consumer boundary.
      // `readBindings()` deliberately maps JSON `null` to the legacy
      // absent-file default; FR-062 must instead distinguish an existing,
      // malformed store from an absent one.
      const retained = bindingsFile.safeParse(
        JSON.parse(readFileSync(path, "utf8")),
      );
      bindings = retained.success
        ? {
            availability: "available",
            bindings: canonicalizeBindings(retained.data.bindings),
          }
        : {
            availability: "unreadable",
            reason: `${path} is valid JSON but does not match the retained bindings schema: ${retained.error.issues
              .map(
                (issue) =>
                  `${issue.path.join(".") || "<root>"}: ${issue.message}`,
              )
              .join("; ")}`,
          };
    } catch (cause) {
      const detail = cause instanceof Error ? cause.message : String(cause);
      bindings = { availability: "unreadable", reason: detail };
    }
  }
  return {
    ok: true,
    value: {
      assurance: parsedExport.value,
      premises: premises.value,
      audit: audit.value,
      bindings,
    },
  };
}

function canonicalizeBindings(bindings: Binding[]): Binding[] {
  return bindings
    .map((item) => ({
      ...item,
      symbols: [...item.symbols].sort(compare),
      ...(item.affirmations === undefined
        ? {}
        : {
            affirmations: item.affirmations
              .map((entry) => ({ ...entry }))
              .sort(
                (left, right) =>
                  compare(left.who, right.who) ||
                  compare(left.commit, right.commit) ||
                  compare(left.note ?? "", right.note ?? ""),
              ),
          }),
    }))
    .sort(
      (left, right) =>
        compare(left.obligation, right.obligation) ||
        compare(left.suite, right.suite) ||
        compare(left.commit, right.commit) ||
        compare(left.statementHashAtBinding, right.statementHashAtBinding) ||
        compare(JSON.stringify(left.symbols), JSON.stringify(right.symbols)) ||
        compare(
          JSON.stringify(left.affirmations ?? []),
          JSON.stringify(right.affirmations ?? []),
        ),
    );
}

function compare(left: string, right: string): number {
  return left === right ? 0 : left < right ? -1 : 1;
}

function readRequired(
  path: string,
  input: GraphLoadFailure["input"],
): { ok: true; value: string } | { ok: false; error: GraphLoadFailure } {
  try {
    return { ok: true, value: readFileSync(path, "utf8") };
  } catch (cause) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    return {
      ok: false,
      error: {
        kind: "graph-input-read",
        input,
        message: `cannot read ${input} input ${path}: ${detail}`,
      },
    };
  }
}

function invalid(
  input: GraphLoadFailure["input"],
  message: string,
): { ok: false; error: GraphLoadFailure } {
  return {
    ok: false,
    error: { kind: "graph-input-invalid", input, message },
  };
}
