/**
 * Authored assurance arguments with explicit sufficiency decisions.
 *
 * The renderer never promotes an evidence result into a claim. A named actor
 * with declared authority must decide each authored sufficiency criterion, and
 * assumptions and challenges remain independently visible.
 */

import type { DischargeReport } from "./discharge.js";

export interface AssuranceArgumentDefinition {
  id: string;
  title: string;
  type: "AssuranceArgument";
  status: "proposed" | "active" | "retired";
  owner: string;
  profile: string;
  top_claim: { id: string; statement: string; subject: string };
  reasoning: Array<{
    id: string;
    statement: string;
    supports: string;
    sufficiency_criteria: string[];
  }>;
  assumptions: Array<{
    id: string;
    statement: string;
    owner: string;
    status: "open" | "accepted" | "invalidated";
    review_by: string;
  }>;
  participants: Array<{
    id: string;
    role: string;
    authority: string;
    independence: string;
  }>;
  challenges: Array<{
    id: string;
    target: string;
    statement: string;
    status: "open" | "resolved" | "accepted-risk";
    owner: string;
    resolution_refs?: string[];
    expires_at?: string;
  }>;
  relationships: Array<{
    target: string;
    type: "supports" | "challenges" | "references";
  }>;
}

export interface SufficiencyDecision {
  reasoningId: string;
  criterion: string;
  state: "satisfied" | "open";
  evidenceRefs: string[];
  decidedBy: string;
  authority: string;
  decidedAt: string;
  expiresAt: string;
  sourceRevision: string;
  evidenceDigest: string;
  rationale?: string;
}

export interface CriterionView {
  criterion: string;
  status: "supported" | "open";
  reason?: string;
  decision?: SufficiencyDecision;
}

export interface ReasoningView {
  id: string;
  statement: string;
  supports: string;
  status: "supported" | "open";
  criteria: CriterionView[];
}

export interface AssumptionView {
  id: string;
  statement: string;
  owner: string;
  status: "supported" | "open";
  declaredStatus: AssuranceArgumentDefinition["assumptions"][number]["status"];
  reviewBy: string;
  reason?: string;
}

export interface ChallengeView {
  id: string;
  target: string;
  statement: string;
  owner: string;
  status: "resolved" | "open";
  declaredStatus: AssuranceArgumentDefinition["challenges"][number]["status"];
  resolutionRefs: string[];
  expiresAt?: string;
  reason?: string;
}

export interface AuthoredArgumentView {
  schemaVersion: "authored-assurance-view-v1";
  argument: Pick<
    AssuranceArgumentDefinition,
    "id" | "title" | "status" | "owner" | "profile"
  >;
  asOf: string;
  topClaim: AssuranceArgumentDefinition["top_claim"] & {
    status: "supported" | "open";
    reasons: string[];
  };
  reasoning: ReasoningView[];
  assumptions: AssumptionView[];
  participants: AssuranceArgumentDefinition["participants"];
  challenges: ChallengeView[];
  relationships: AssuranceArgumentDefinition["relationships"];
  discharge?: DischargeReport;
  unusedDecisions: Array<{ reasoningId: string; criterion: string }>;
}

export interface BuildAuthoredArgumentRequest {
  argument: unknown;
  decisions: SufficiencyDecision[];
  asOf: string;
  discharge?: DischargeReport;
}

export function buildAuthoredArgumentView(
  request: BuildAuthoredArgumentRequest,
): AuthoredArgumentView {
  const argument = parseAssuranceArgument(request.argument);
  const asOf = instant("asOf", request.asOf);
  const parsedDecisions = request.decisions.map((value) =>
    parseSufficiencyDecision(value),
  );
  const decisions = new Map<string, SufficiencyDecision>();
  for (const decision of parsedDecisions) {
    const participant = argument.participants.find(
      (candidate) => candidate.id === decision.decidedBy,
    );
    if (!participant) {
      throw new Error(
        `sufficiency decision maker ${decision.decidedBy} is not a declared participant`,
      );
    }
    if (participant.authority !== decision.authority) {
      throw new Error(
        `sufficiency decision authority for ${decision.decidedBy} does not match the authored participant`,
      );
    }
    const key = decisionKey(decision.reasoningId, decision.criterion);
    if (decisions.has(key)) {
      throw new Error(
        `duplicate sufficiency decision for ${decision.reasoningId}: ${decision.criterion}`,
      );
    }
    decisions.set(key, decision);
  }

  const used = new Set<string>();
  const reasoning = argument.reasoning.map((reason) => {
    const criteria = reason.sufficiency_criteria.map((criterion) => {
      const key = decisionKey(reason.id, criterion);
      const decision = decisions.get(key);
      if (!decision) {
        return {
          criterion,
          status: "open" as const,
          reason: "no sufficiency decision",
        };
      }
      used.add(key);
      const current = currentDecision(decision, asOf);
      if (current !== null) {
        return {
          criterion,
          status: "open" as const,
          reason: current,
          decision,
        };
      }
      if (decision.state === "open") {
        return {
          criterion,
          status: "open" as const,
          reason: decision.rationale ?? "decision remains open",
          decision,
        };
      }
      return { criterion, status: "supported" as const, decision };
    });
    return {
      id: reason.id,
      statement: reason.statement,
      supports: reason.supports,
      status: criteria.every((criterion) => criterion.status === "supported")
        ? ("supported" as const)
        : ("open" as const),
      criteria,
    };
  });

  const assumptions = argument.assumptions.map((assumption) => {
    let reason: string | undefined;
    if (assumption.status !== "accepted") {
      reason = `assumption is ${assumption.status}`;
    } else if (instant("review_by", assumption.review_by) <= asOf) {
      reason = "assumption review is due";
    }
    return {
      id: assumption.id,
      statement: assumption.statement,
      owner: assumption.owner,
      status: reason ? ("open" as const) : ("supported" as const),
      declaredStatus: assumption.status,
      reviewBy: assumption.review_by,
      ...(reason ? { reason } : {}),
    };
  });

  const challenges = argument.challenges.map((challenge) =>
    evaluateChallenge(challenge, asOf),
  );

  const reasons: string[] = [];
  if (argument.status !== "active") {
    reasons.push(`argument status is ${argument.status}`);
  }
  if (reasoning.some((item) => item.status === "open")) {
    reasons.push("one or more sufficiency criteria are open");
  }
  if (assumptions.some((item) => item.status === "open")) {
    reasons.push(
      "one or more assumptions are open, invalidated, or due for review",
    );
  }
  if (challenges.some((item) => item.status === "open")) {
    reasons.push("one or more challenges are open or no longer current");
  }
  if (request.discharge) {
    if (request.discharge.binding.open.length > 0) {
      reasons.push("the clause discharge report contains open binding clauses");
    }
    if (request.discharge.unresolved.length > 0) {
      reasons.push(
        "the clause discharge report contains unresolved applicability",
      );
    }
  }

  return {
    schemaVersion: "authored-assurance-view-v1",
    argument: {
      id: argument.id,
      title: argument.title,
      status: argument.status,
      owner: argument.owner,
      profile: argument.profile,
    },
    asOf: request.asOf,
    topClaim: {
      ...argument.top_claim,
      status: reasons.length === 0 ? "supported" : "open",
      reasons,
    },
    reasoning,
    assumptions,
    participants: argument.participants.map((participant) => ({
      ...participant,
    })),
    challenges,
    relationships: argument.relationships.map((relationship) => ({
      ...relationship,
    })),
    ...(request.discharge ? { discharge: request.discharge } : {}),
    unusedDecisions: parsedDecisions
      .filter(
        (decision) =>
          !used.has(decisionKey(decision.reasoningId, decision.criterion)),
      )
      .map(({ reasoningId, criterion }) => ({ reasoningId, criterion })),
  };
}

/** Validate the module-owned AssuranceArgument frontmatter contract. */
export function parseAssuranceArgument(
  value: unknown,
): AssuranceArgumentDefinition {
  const object = record("argument", value);
  const allowed = new Set([
    "id",
    "title",
    "type",
    "status",
    "owner",
    "profile",
    "top_claim",
    "reasoning",
    "assumptions",
    "participants",
    "challenges",
    "relationships",
  ]);
  for (const key of Object.keys(object)) {
    if (!allowed.has(key)) throw new Error(`argument has unknown field ${key}`);
  }
  const id = stringAt(object, "id");
  if (!/^AA-[0-9]+$/.test(id))
    throw new Error("argument id must match AA-<number>");
  literal(object.type, "type", ["AssuranceArgument"] as const);
  const status = literal(object.status, "status", [
    "proposed",
    "active",
    "retired",
  ] as const);
  const profile = stringAt(object, "profile");
  if (!profile.startsWith("ix://"))
    throw new Error("profile must be an ix:// reference");

  const top = record("top_claim", object.top_claim);
  exactKeys(top, "top_claim", ["id", "statement", "subject"]);
  const reasoning = arrayAt(object, "reasoning").map((item, index) => {
    const row = record(`reasoning[${index}]`, item);
    exactKeys(row, `reasoning[${index}]`, [
      "id",
      "statement",
      "supports",
      "sufficiency_criteria",
    ]);
    return {
      id: stringAt(row, "id"),
      statement: stringAt(row, "statement"),
      supports: stringAt(row, "supports"),
      sufficiency_criteria: nonEmptyStrings(
        `reasoning[${index}].sufficiency_criteria`,
        row.sufficiency_criteria,
      ),
    };
  });
  if (reasoning.length === 0) throw new Error("reasoning must not be empty");

  const assumptions = arrayAt(object, "assumptions").map((item, index) => {
    const row = record(`assumptions[${index}]`, item);
    exactKeys(row, `assumptions[${index}]`, [
      "id",
      "statement",
      "owner",
      "status",
      "review_by",
    ]);
    const reviewBy = stringAt(row, "review_by");
    instant("review_by", reviewBy);
    return {
      id: stringAt(row, "id"),
      statement: stringAt(row, "statement"),
      owner: stringAt(row, "owner"),
      status: literal(row.status, "assumption status", [
        "open",
        "accepted",
        "invalidated",
      ] as const),
      review_by: reviewBy,
    };
  });

  const participants = arrayAt(object, "participants").map((item, index) => {
    const row = record(`participants[${index}]`, item);
    exactKeys(row, `participants[${index}]`, [
      "id",
      "role",
      "authority",
      "independence",
    ]);
    return {
      id: stringAt(row, "id"),
      role: stringAt(row, "role"),
      authority: stringAt(row, "authority"),
      independence: stringAt(row, "independence"),
    };
  });
  if (participants.length === 0)
    throw new Error("participants must not be empty");

  const challenges = arrayAt(object, "challenges").map((item, index) => {
    const row = record(`challenges[${index}]`, item);
    exactKeys(
      row,
      `challenges[${index}]`,
      [
        "id",
        "target",
        "statement",
        "status",
        "owner",
        "resolution_refs",
        "expires_at",
      ],
      true,
    );
    const expiresAt = optionalStringAt(row, "expires_at");
    if (expiresAt) instant("expires_at", expiresAt);
    const resolutionRefs = row.resolution_refs
      ? nonEmptyStrings(
          `challenges[${index}].resolution_refs`,
          row.resolution_refs,
        )
      : undefined;
    return {
      id: stringAt(row, "id"),
      target: stringAt(row, "target"),
      statement: stringAt(row, "statement"),
      status: literal(row.status, "challenge status", [
        "open",
        "resolved",
        "accepted-risk",
      ] as const),
      owner: stringAt(row, "owner"),
      ...(resolutionRefs ? { resolution_refs: resolutionRefs } : {}),
      ...(expiresAt ? { expires_at: expiresAt } : {}),
    };
  });

  const relationships = arrayAt(object, "relationships").map((item, index) => {
    const row = record(`relationships[${index}]`, item);
    exactKeys(row, `relationships[${index}]`, ["target", "type"]);
    const target = stringAt(row, "target");
    if (!target.startsWith("ix://")) {
      throw new Error(
        `relationships[${index}].target must be an ix:// reference`,
      );
    }
    return {
      target,
      type: literal(row.type, "relationship type", [
        "supports",
        "challenges",
        "references",
      ] as const),
    };
  });

  uniqueIds("reasoning", reasoning);
  uniqueIds("assumptions", assumptions);
  uniqueIds("participants", participants);
  uniqueIds("challenges", challenges);

  const topClaimId = stringAt(top, "id");
  const nodeIds = [
    topClaimId,
    ...reasoning.map((item) => item.id),
    ...assumptions.map((item) => item.id),
  ];
  if (new Set(nodeIds).size !== nodeIds.length) {
    throw new Error("top claim, reasoning, and assumption ids must be unique");
  }
  const reasoningById = new Map(reasoning.map((item) => [item.id, item]));
  for (const reason of reasoning) {
    let cursor = reason;
    const visited = new Set<string>();
    while (cursor.supports !== topClaimId) {
      if (visited.has(cursor.id)) {
        throw new Error(
          `reasoning cycle does not reach top claim from ${reason.id}`,
        );
      }
      visited.add(cursor.id);
      const parent = reasoningById.get(cursor.supports);
      if (!parent) {
        throw new Error(
          `reasoning ${reason.id} supports unknown target ${cursor.supports}`,
        );
      }
      cursor = parent;
    }
  }
  const challengeTargets = new Set([
    topClaimId,
    ...reasoning.map((item) => item.id),
    ...assumptions.map((item) => item.id),
  ]);
  for (const challenge of challenges) {
    if (!challengeTargets.has(challenge.target)) {
      throw new Error(
        `challenge ${challenge.id} targets unknown argument node ${challenge.target}`,
      );
    }
  }

  return {
    id,
    title: stringAt(object, "title"),
    type: "AssuranceArgument",
    status,
    owner: stringAt(object, "owner"),
    profile,
    top_claim: {
      id: topClaimId,
      statement: stringAt(top, "statement"),
      subject: stringAt(top, "subject"),
    },
    reasoning,
    assumptions,
    participants,
    challenges,
    relationships,
  };
}

export function renderAuthoredArgument(view: AuthoredArgumentView): string {
  const mark = (state: "supported" | "open" | "resolved") =>
    state === "supported" || state === "resolved" ? "✓" : "◇";
  const lines = [
    `# ${view.argument.id}: ${view.argument.title}`,
    "",
    `**${mark(view.topClaim.status)} ${view.topClaim.status.toUpperCase()}** — ${view.topClaim.statement}`,
    "",
    `Subject: ${view.topClaim.subject}`,
    `Owner: ${view.argument.owner}`,
    `Evaluated as of: ${view.asOf}`,
    "",
  ];
  if (view.topClaim.reasons.length > 0) {
    lines.push("## Open reasons", "");
    for (const reason of view.topClaim.reasons) lines.push(`- ${reason}`);
    lines.push("");
  }
  lines.push("## Reasoning and sufficiency", "");
  for (const reasoning of view.reasoning) {
    lines.push(
      `### ${mark(reasoning.status)} ${reasoning.id}`,
      "",
      reasoning.statement,
      "",
    );
    for (const criterion of reasoning.criteria) {
      lines.push(
        `- ${mark(criterion.status)} ${criterion.criterion}${criterion.reason ? ` — ${criterion.reason}` : ""}`,
      );
    }
    lines.push("");
  }
  lines.push("## Assumptions", "");
  if (view.assumptions.length === 0) lines.push("_None._", "");
  for (const assumption of view.assumptions) {
    lines.push(
      `- ${mark(assumption.status)} \`${assumption.id}\` (${assumption.owner}): ${assumption.statement}${assumption.reason ? ` — ${assumption.reason}` : ""}`,
    );
  }
  if (view.assumptions.length > 0) lines.push("");
  lines.push("## Challenges", "");
  if (view.challenges.length === 0) lines.push("_None._", "");
  for (const challenge of view.challenges) {
    lines.push(
      `- ${mark(challenge.status)} \`${challenge.id}\` (${challenge.owner}): ${challenge.statement}${challenge.reason ? ` — ${challenge.reason}` : ""}`,
    );
  }
  if (view.challenges.length > 0) lines.push("");
  lines.push("## Participants and authority", "");
  for (const participant of view.participants) {
    lines.push(
      `- \`${participant.id}\` — ${participant.role}; authority: ${participant.authority}; independence: ${participant.independence}`,
    );
  }
  return `${lines.join("\n").trimEnd()}\n`;
}

function evaluateChallenge(
  challenge: AssuranceArgumentDefinition["challenges"][number],
  asOf: number,
): ChallengeView {
  const refs = challenge.resolution_refs ?? [];
  let reason: string | undefined;
  if (challenge.status === "open") {
    reason = "challenge is open";
  } else if (refs.length === 0) {
    reason = "challenge has no resolution reference";
  } else if (challenge.status === "accepted-risk") {
    if (!challenge.expires_at) reason = "accepted risk has no expiry";
    else if (instant("expires_at", challenge.expires_at) <= asOf) {
      reason = "accepted risk is expired";
    }
  }
  return {
    id: challenge.id,
    target: challenge.target,
    statement: challenge.statement,
    owner: challenge.owner,
    status: reason ? "open" : "resolved",
    declaredStatus: challenge.status,
    resolutionRefs: [...refs],
    ...(challenge.expires_at ? { expiresAt: challenge.expires_at } : {}),
    ...(reason ? { reason } : {}),
  };
}

function parseSufficiencyDecision(value: unknown): SufficiencyDecision {
  const decision = record("sufficiency decision", value);
  exactShape(
    decision,
    "sufficiency decision",
    [
      "reasoningId",
      "criterion",
      "state",
      "evidenceRefs",
      "decidedBy",
      "authority",
      "decidedAt",
      "expiresAt",
      "sourceRevision",
      "evidenceDigest",
    ],
    ["rationale"],
  );
  const state = literal(decision.state, "decision state", [
    "satisfied",
    "open",
  ] as const);
  const evidenceRefs = stringArray(
    "evidenceRefs",
    decision.evidenceRefs,
    state === "satisfied",
  );
  const decidedAt = stringAt(decision, "decidedAt");
  const expiresAt = stringAt(decision, "expiresAt");
  instant("decidedAt", decidedAt);
  instant("expiresAt", expiresAt);
  const evidenceDigest = stringAt(decision, "evidenceDigest");
  if (!/^sha256:[0-9a-f]{64}$/.test(evidenceDigest)) {
    throw new Error(
      "decision evidenceDigest must be sha256:<64 lowercase hex>",
    );
  }
  const rationale = optionalStringAt(decision, "rationale");
  return {
    reasoningId: stringAt(decision, "reasoningId"),
    criterion: stringAt(decision, "criterion"),
    state,
    evidenceRefs,
    decidedBy: stringAt(decision, "decidedBy"),
    authority: stringAt(decision, "authority"),
    decidedAt,
    expiresAt,
    sourceRevision: stringAt(decision, "sourceRevision"),
    evidenceDigest,
    ...(rationale ? { rationale } : {}),
  };
}

function currentDecision(
  decision: SufficiencyDecision,
  asOf: number,
): string | null {
  const decidedAt = instant("decidedAt", decision.decidedAt);
  const expiresAt = instant("expiresAt", decision.expiresAt);
  if (expiresAt <= decidedAt) return "decision expiry is not after decision";
  if (decidedAt > asOf) return "decision is in the future";
  if (expiresAt <= asOf) return "decision is expired";
  return null;
}

function decisionKey(reasoningId: string, criterion: string): string {
  return `${reasoningId}\u0000${criterion}`;
}

function record(name: string, value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${name} must be an object`);
  }
  return value as Record<string, unknown>;
}

function arrayAt(object: Record<string, unknown>, key: string): unknown[] {
  const value = object[key];
  if (!Array.isArray(value)) throw new Error(`${key} must be an array`);
  return value;
}

function stringAt(object: Record<string, unknown>, key: string): string {
  const value = object[key];
  if (typeof value !== "string") throw new Error(`${key} must be a string`);
  nonEmpty(key, value);
  return value;
}

function optionalStringAt(
  object: Record<string, unknown>,
  key: string,
): string | undefined {
  if (!(key in object)) return undefined;
  return stringAt(object, key);
}

function exactKeys(
  object: Record<string, unknown>,
  name: string,
  keys: string[],
  optional = false,
): void {
  const allowed = new Set(keys);
  for (const key of Object.keys(object)) {
    if (!allowed.has(key)) throw new Error(`${name} has unknown field ${key}`);
  }
  if (!optional) {
    for (const key of keys) {
      if (!(key in object)) throw new Error(`${name} is missing ${key}`);
    }
  } else {
    const required = keys.filter(
      (key) => key !== "resolution_refs" && key !== "expires_at",
    );
    for (const key of required) {
      if (!(key in object)) throw new Error(`${name} is missing ${key}`);
    }
  }
}

function exactShape(
  object: Record<string, unknown>,
  name: string,
  required: string[],
  optional: string[] = [],
): void {
  const allowed = new Set([...required, ...optional]);
  for (const key of Object.keys(object)) {
    if (!allowed.has(key)) throw new Error(`${name} has unknown field ${key}`);
  }
  for (const key of required) {
    if (!(key in object)) throw new Error(`${name} is missing ${key}`);
  }
}

function nonEmptyStrings(name: string, value: unknown): string[] {
  return stringArray(name, value, true);
}

function stringArray(
  name: string,
  value: unknown,
  requireValue: boolean,
): string[] {
  if (!Array.isArray(value) || (requireValue && value.length === 0)) {
    throw new Error(
      `${name} must be ${requireValue ? "a non-empty" : "an"} array`,
    );
  }
  const strings = value.map((item) => {
    if (typeof item !== "string")
      throw new Error(`${name} must contain strings`);
    nonEmpty(name, item);
    return item;
  });
  if (new Set(strings).size !== strings.length) {
    throw new Error(`${name} must contain unique values`);
  }
  return strings;
}

function uniqueIds(name: string, values: Array<{ id: string }>): void {
  const ids = new Set<string>();
  for (const value of values) {
    if (ids.has(value.id))
      throw new Error(`${name} contains duplicate id ${value.id}`);
    ids.add(value.id);
  }
}

function literal<const T extends readonly string[]>(
  value: unknown,
  name: string,
  allowed: T,
): T[number] {
  if (typeof value !== "string" || !allowed.includes(value)) {
    throw new Error(`${name} must be one of ${allowed.join(", ")}`);
  }
  return value;
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
