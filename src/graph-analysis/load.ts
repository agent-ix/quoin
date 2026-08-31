/** Read the three explicit FR-062 inputs plus retained Quoin bindings. */

import { existsSync, readFileSync } from "node:fs";

import {
  bindingsPath,
  readBindings,
  StoreReadError,
} from "../evidence/index.js";
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
      bindings = {
        availability: "available",
        bindings: readBindings(options.repo).bindings,
      };
    } catch (cause) {
      const detail =
        cause instanceof StoreReadError || cause instanceof Error
          ? cause.message
          : String(cause);
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
