/**
 * Clause discharge accounting over a validated Quire binding report.
 *
 * Applicability and discharge are deliberately separate facts. Quire decides
 * whether a clause is binding, not binding, or unresolved. Quoin partitions
 * only the binding population into direct evidence, an approved disposition,
 * or open. Unresolved applicability stays outside that denominator and no
 * aggregate score is manufactured.
 */

import type {
  ClauseBinding,
  ClauseBindingReport,
  ClauseForce,
  ClauseSetKey,
} from "../quire/index.js";

export interface DischargeAttestation {
  attestedBy: string;
  authority: string;
  attestedAt: string;
  expiresAt: string;
  sourceRevision: string;
  evidenceDigest: string;
}

interface FactBase {
  clauseId: string;
  attestation: DischargeAttestation;
}

export interface DirectDischargeFact extends FactBase {
  kind: "direct";
  evidenceRefs: string[];
}

export interface DispositionFact extends FactBase {
  kind: "disposition";
  decision: "accepted_risk" | "temporary_exception" | "delegated";
  rationale: string;
  approvalRef: string;
}

export type DischargeFact = DirectDischargeFact | DispositionFact;

export type DischargeState =
  "direct" | "disposition" | "open" | "unresolved" | "not_binding";

export interface ClauseDischarge {
  clauseId: string;
  force: ClauseForce;
  state: DischargeState;
  expectedOutputs: string[];
  reason?: string;
  fact?: DischargeFact;
}

export interface UnusedDischargeFact {
  clauseId: string;
  kind: DischargeFact["kind"];
  reason: "unknown_clause" | "not_binding" | "unresolved";
}

export interface DischargeReport {
  schemaVersion: "clause-discharge-v1";
  clauseSet: ClauseSetKey;
  clauseSetDigest: string;
  context: Record<string, string>;
  asOf: string;
  binding: {
    direct: ClauseDischarge[];
    dispositions: ClauseDischarge[];
    open: ClauseDischarge[];
  };
  unresolved: ClauseDischarge[];
  notBinding: ClauseDischarge[];
  unusedFacts: UnusedDischargeFact[];
}

export interface BuildDischargeRequest {
  binding: ClauseBindingReport;
  facts: DischargeFact[];
  /** Explicit evaluation instant; this layer never reads the wall clock. */
  asOf: string;
}

/** Build a complete, non-scored discharge partition. */
export function buildDischargeReport(
  request: BuildDischargeRequest,
): DischargeReport {
  const asOf = instant("asOf", request.asOf);
  const facts = new Map<string, DischargeFact>();
  for (const fact of request.facts) {
    validateFact(fact);
    if (facts.has(fact.clauseId)) {
      throw new Error(`duplicate discharge fact for clause ${fact.clauseId}`);
    }
    facts.set(fact.clauseId, fact);
  }

  const direct: ClauseDischarge[] = [];
  const dispositions: ClauseDischarge[] = [];
  const open: ClauseDischarge[] = [];
  const unresolved: ClauseDischarge[] = [];
  const notBinding: ClauseDischarge[] = [];
  const unusedFacts: UnusedDischargeFact[] = [];
  const known = new Set<string>();

  for (const clause of request.binding.clauses) {
    known.add(clause.clauseId);
    const fact = facts.get(clause.clauseId);
    if (clause.outcome === "unresolved") {
      unresolved.push(entry(clause, "unresolved", reasonFor(clause)));
      if (fact) {
        unusedFacts.push({
          clauseId: clause.clauseId,
          kind: fact.kind,
          reason: "unresolved",
        });
      }
      continue;
    }
    if (clause.outcome === "not_binding") {
      notBinding.push(entry(clause, "not_binding"));
      if (fact) {
        unusedFacts.push({
          clauseId: clause.clauseId,
          kind: fact.kind,
          reason: "not_binding",
        });
      }
      continue;
    }
    if (!fact) {
      open.push(entry(clause, "open", "no discharge fact"));
      continue;
    }

    const current = currentAttestation(fact.attestation, asOf);
    if (current !== null) {
      open.push(entry(clause, "open", current, fact));
      continue;
    }
    if (fact.kind === "direct") {
      direct.push(entry(clause, "direct", undefined, fact));
    } else {
      dispositions.push(entry(clause, "disposition", undefined, fact));
    }
  }

  for (const fact of request.facts) {
    if (!known.has(fact.clauseId)) {
      unusedFacts.push({
        clauseId: fact.clauseId,
        kind: fact.kind,
        reason: "unknown_clause",
      });
    }
  }

  return {
    schemaVersion: "clause-discharge-v1",
    clauseSet: request.binding.clauseSet,
    clauseSetDigest: request.binding.clauseSetDigest,
    context: { ...request.binding.context },
    asOf: request.asOf,
    binding: { direct, dispositions, open },
    unresolved,
    notBinding,
    unusedFacts,
  };
}

/** Render the same complete partition as compact, deterministic Markdown. */
export function renderDischargeReport(report: DischargeReport): string {
  const lines = [
    `# Clause discharge: ${report.clauseSet.authority}/${report.clauseSet.id}@${report.clauseSet.version}`,
    "",
    `- Clause-set digest: \`${report.clauseSetDigest}\``,
    `- Evaluated as of: \`${report.asOf}\``,
    "",
  ];
  section(lines, "Direct evidence", report.binding.direct);
  section(lines, "Approved dispositions", report.binding.dispositions);
  section(lines, "Open binding clauses", report.binding.open);
  section(lines, "Unresolved applicability", report.unresolved);
  section(lines, "Not binding", report.notBinding);
  if (report.unusedFacts.length > 0) {
    lines.push("## Unused facts", "");
    for (const fact of report.unusedFacts) {
      lines.push(`- \`${fact.clauseId}\` (${fact.kind}): ${fact.reason}`);
    }
    lines.push("");
  }
  return `${lines.join("\n").trimEnd()}\n`;
}

function entry(
  clause: ClauseBinding,
  state: DischargeState,
  reason?: string,
  fact?: DischargeFact,
): ClauseDischarge {
  return {
    clauseId: clause.clauseId,
    force: clause.force,
    state,
    expectedOutputs: [...clause.expectedOutputs],
    ...(reason ? { reason } : {}),
    ...(fact ? { fact } : {}),
  };
}

function reasonFor(clause: ClauseBinding): string {
  return clause.reasons.length === 0
    ? "applicability is unresolved"
    : clause.reasons.map((reason) => reason.message).join("; ");
}

function validateFact(fact: DischargeFact): void {
  nonEmpty("clauseId", fact.clauseId);
  const attestation = fact.attestation;
  for (const [name, value] of [
    ["attestedBy", attestation.attestedBy],
    ["authority", attestation.authority],
    ["sourceRevision", attestation.sourceRevision],
  ] as const) {
    nonEmpty(name, value);
  }
  instant("attestedAt", attestation.attestedAt);
  instant("expiresAt", attestation.expiresAt);
  if (!/^sha256:[0-9a-f]{64}$/.test(attestation.evidenceDigest)) {
    throw new Error(
      "attestation evidenceDigest must be sha256:<64 lowercase hex>",
    );
  }
  if (fact.kind === "direct") {
    nonEmptyList("evidenceRefs", fact.evidenceRefs);
  } else {
    nonEmpty("rationale", fact.rationale);
    nonEmpty("approvalRef", fact.approvalRef);
  }
}

function currentAttestation(
  attestation: DischargeAttestation,
  asOf: number,
): string | null {
  const attestedAt = instant("attestedAt", attestation.attestedAt);
  const expiresAt = instant("expiresAt", attestation.expiresAt);
  if (expiresAt <= attestedAt)
    return "attestation expiry is not after attestation";
  if (attestedAt > asOf) return "attestation is in the future";
  if (expiresAt <= asOf) return "discharge fact is expired";
  return null;
}

function instant(name: string, value: string): number {
  if (
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(
      value,
    )
  ) {
    throw new Error(`${name} must be an ISO-8601 instant`);
  }
  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed))
    throw new Error(`${name} must be an ISO-8601 instant`);
  return parsed;
}

function nonEmpty(name: string, value: string): void {
  if (value.trim().length === 0) throw new Error(`${name} must not be empty`);
}

function nonEmptyList(name: string, values: string[]): void {
  if (
    values.length === 0 ||
    values.some((value) => value.trim().length === 0)
  ) {
    throw new Error(`${name} must contain non-empty values`);
  }
}

function section(
  lines: string[],
  title: string,
  entries: ClauseDischarge[],
): void {
  lines.push(`## ${title}`, "");
  if (entries.length === 0) {
    lines.push("_None._", "");
    return;
  }
  for (const item of entries) {
    const outputs =
      item.expectedOutputs.length === 0
        ? "no declared outputs"
        : item.expectedOutputs.map((value) => `\`${value}\``).join(", ");
    lines.push(
      `- \`${item.clauseId}\` — ${item.force}; ${outputs}${item.reason ? `; ${item.reason}` : ""}`,
    );
  }
  lines.push("");
}
