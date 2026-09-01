import { createHash } from "node:crypto";

import { canonicalBytes, digestValue, assertDigest } from "./integrity.js";
import type {
  ChangeAssuranceRecord,
  Check,
  DecisionHistory,
  ReceiptDecisionEvent,
} from "./types.js";

const DIGEST = /^[a-f0-9]{64}$/;
const IDENTITY = /^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$/;

export function sealChangeRecord(
  input:
    | Omit<ChangeAssuranceRecord, "digest">
    | (Omit<ChangeAssuranceRecord, "digest"> & { digest?: undefined }),
): ChangeAssuranceRecord {
  const rest = { ...input } as Record<string, unknown>;
  delete rest.digest;
  const normalized = normalizeRecord(
    rest as unknown as Omit<ChangeAssuranceRecord, "digest">,
  );
  validateRecordShape(normalized, false);
  const digest = digestValue(normalized as unknown as Record<string, unknown>);
  return verifyChangeRecord({ ...normalized, digest });
}

export function verifyChangeRecord(value: unknown): ChangeAssuranceRecord {
  validateRecordShape(value, true);
  assertDigest(value as unknown as Record<string, unknown>);
  return value as ChangeAssuranceRecord;
}

export function recordBytes(value: ChangeAssuranceRecord): Uint8Array {
  return canonicalBytes(verifyChangeRecord(value));
}

export function verifyLineage(
  record: ChangeAssuranceRecord,
  parents: ChangeAssuranceRecord[],
): string[] {
  verifyChangeRecord(record);
  const ordered = parents.slice().sort((a, b) => a.revision - b.revision);
  if (record.revision === 1) {
    if (record.parent_digest !== null || ordered.length !== 0) {
      throw new Error("revision 1 must have a null parent and no parent chain");
    }
    return [];
  }
  if (ordered.length !== record.revision - 1)
    throw new Error("revision gap in parent chain");
  let expectedParent: string | null = null;
  for (let index = 0; index < ordered.length; index++) {
    const parent = verifyChangeRecord(ordered[index]);
    if (parent.record_id !== record.record_id)
      throw new Error("cross-record parent");
    if (parent.revision !== index + 1)
      throw new Error("revision gap in parent chain");
    if (parent.parent_digest !== expectedParent)
      throw new Error("parent digest mismatch");
    expectedParent = parent.digest;
  }
  if (record.parent_digest !== expectedParent)
    throw new Error("immediate parent mismatch");
  return ordered.map((parent) => parent.digest);
}

export function validateDecision(
  record: ChangeAssuranceRecord,
  history: DecisionHistory,
): Check & { event: ReceiptDecisionEvent | null } {
  if (!history) {
    return {
      outcome: "incomplete",
      reasons: ["event_chain_missing"],
      event: null,
    };
  }
  try {
    const retainedHistory = object(history, "decision history");
    exact(retainedHistory, ["run_id", "events"]);
    identity(retainedHistory.run_id, "decision history run_id");
    array(retainedHistory.events, "decision history events", false);
  } catch {
    return {
      outcome: "invalid",
      reasons: ["event_chain_invalid"],
      event: null,
    };
  }
  if (!verifyIxFlowChain(history.events)) {
    return {
      outcome: "invalid",
      reasons: ["event_chain_invalid"],
      event: null,
    };
  }
  if (history.run_id !== record.review_workflow.run_id) {
    return { outcome: "invalid", reasons: ["decision_mismatch"], event: null };
  }
  if (history.events.length === 0) {
    return {
      outcome: "incomplete",
      reasons: ["decision_missing"],
      event: null,
    };
  }
  const candidates = history.events.filter(
    (event) => event.kind === record.review_workflow.decision_event_kind,
  );
  if (candidates.length === 0) {
    return {
      outcome: "incomplete",
      reasons: ["decision_missing"],
      event: null,
    };
  }
  const matching = candidates.filter((event) => {
    try {
      const rawEvent = object(event, "decision event");
      exact(rawEvent, [
        "id",
        "ts",
        "actor",
        "kind",
        "payload",
        "prevHash",
        "hash",
      ]);
      identity(event.id, "decision event id");
      nonempty(event.ts, "decision event timestamp");
      digest(event.prevHash, "decision previous hash");
      digest(event.hash, "decision event hash");
      const actor = object(event.actor, "decision actor");
      exact(actor, [
        "kind",
        "id",
        ...(event.actor.ackToken === undefined ? [] : ["ackToken"]),
      ]);
      oneOf(
        event.actor.kind,
        ["agent", "human", "service"],
        "decision actor kind",
      );
      nonempty(event.actor.id, "decision actor id");
      if (event.actor.ackToken !== undefined)
        nonempty(event.actor.ackToken, "decision actor ackToken");
      const payload = object(event.payload, "decision payload");
      exact(payload, [
        "schema_version",
        "record_id",
        "revision",
        "record_digest",
        "decision",
        ...(payload.note === undefined ? [] : ["note"]),
      ]);
      const decisionPayload = payload as unknown as {
        schema_version: number;
        record_id: string;
        revision: number;
        record_digest: string;
        decision: "approved" | "rejected" | "revise";
        note?: string;
      };
      if (decisionPayload.note !== undefined)
        nonempty(decisionPayload.note, "decision note");
      return (
        event.actor.kind === "human" &&
        decisionPayload.schema_version === 1 &&
        decisionPayload.record_id === record.record_id &&
        decisionPayload.revision === record.revision &&
        decisionPayload.record_digest === record.digest &&
        ["approved", "rejected", "revise"].includes(decisionPayload.decision)
      );
    } catch {
      return false;
    }
  });
  if (matching.length !== 1 || matching.length !== candidates.length) {
    return { outcome: "invalid", reasons: ["decision_mismatch"], event: null };
  }
  const source = matching[0];
  const sourcePayload = source.payload as {
    decision: "approved" | "rejected" | "revise";
  };
  const event: ReceiptDecisionEvent = {
    run_id: history.run_id,
    event_id: source.id,
    event_hash: source.hash,
    chain_tail_hash: history.events.at(-1)?.hash ?? source.hash,
    recorded_actor: source.actor.id,
    decision: sourcePayload.decision,
  };
  if (sourcePayload.decision === "rejected") {
    return { outcome: "invalid", reasons: ["review_rejected"], event };
  }
  if (sourcePayload.decision === "revise") {
    return {
      outcome: "invalid",
      reasons: ["review_revision_requested"],
      event,
    };
  }
  return { outcome: "valid", reasons: [], event };
}

function normalizeRecord(
  input: Omit<ChangeAssuranceRecord, "digest">,
): Omit<ChangeAssuranceRecord, "digest"> {
  const value = structuredClone(input);
  value.subject.scope.sort();
  value.source_connections.sort((a, b) =>
    compareUtf16(a.source_id, b.source_id),
  );
  value.impact_snapshot.gaps.sort();
  value.definition.requirements.sort((a, b) => compareUtf16(a.id, b.id));
  value.definition.preservation_constraints.sort((a, b) =>
    compareUtf16(a.id, b.id),
  );
  value.definition.proof_obligations.sort((a, b) =>
    compareUtf16(a.proof_id, b.proof_id),
  );
  value.definition.unknowns.sort((a, b) => compareUtf16(a.id, b.id));
  for (const requirement of value.definition.requirements)
    requirement.source_ids.sort();
  for (const constraint of value.definition.preservation_constraints)
    constraint.source_ids.sort();
  for (const proof of value.definition.proof_obligations)
    proof.obligation_ids.sort();
  return value;
}

function validateRecordShape(
  value: unknown,
  sealed: boolean,
): asserts value is ChangeAssuranceRecord {
  const root = object(value, "record");
  exact(root, [
    "schema_version",
    "record_type",
    "record_id",
    "revision",
    "parent_digest",
    ...(sealed ? ["digest"] : []),
    "subject",
    "source_connections",
    "impact_snapshot",
    "definition",
    "review_workflow",
  ]);
  equal(root.schema_version, 1, "schema_version");
  equal(root.record_type, "change_assurance", "record_type");
  identity(root.record_id, "record_id");
  if (!Number.isInteger(root.revision) || (root.revision as number) < 1)
    fail("revision");
  if (root.parent_digest !== null) digest(root.parent_digest, "parent_digest");
  if (root.revision === 1 && root.parent_digest !== null)
    fail("revision 1 parent_digest");
  if ((root.revision as number) > 1 && root.parent_digest === null)
    fail("successor parent_digest");
  if (sealed) digest(root.digest, "digest");

  const subject = object(root.subject, "subject");
  exact(subject, ["repository", "base_revision", "scope"]);
  nonempty(subject.repository, "subject.repository");
  nonempty(subject.base_revision, "subject.base_revision");
  stringArray(subject.scope, "subject.scope", true, true);
  sorted(subject.scope as string[], (value) => value, "subject.scope");

  const sources = array(root.source_connections, "source_connections", true);
  const sourceIds = sources.map((item, index) => {
    const source = object(item, `source_connections[${index}]`);
    exact(source, ["source_id", "kind", "revision", "digest"]);
    identity(source.source_id, "source_id");
    oneOf(
      source.kind,
      [
        "requirement",
        "test",
        "api",
        "architecture",
        "impact_evidence",
        "recovery_evidence",
        "other",
      ],
      "source kind",
    );
    nonempty(source.revision, "source revision");
    digest(source.digest, "source digest");
    return source.source_id as string;
  });
  unique(sourceIds, "duplicate source identity");
  sorted(
    sources,
    (value) => (value as { source_id: string }).source_id,
    "source_connections",
  );

  const impact = object(root.impact_snapshot, "impact_snapshot");
  exact(impact, [
    "identity",
    "revision",
    "digest",
    "completeness",
    "truncated",
    "gaps",
  ]);
  identity(impact.identity, "impact identity");
  nonempty(impact.revision, "impact revision");
  digest(impact.digest, "impact digest");
  oneOf(impact.completeness, ["complete", "incomplete"], "impact completeness");
  if (typeof impact.truncated !== "boolean") fail("impact truncated");
  stringArray(impact.gaps, "impact gaps", false, false);
  sorted(impact.gaps as string[], (value) => value, "impact gaps");

  const definition = object(root.definition, "definition");
  exact(definition, [
    "requirements",
    "preservation_constraints",
    "proof_obligations",
    "unknowns",
  ]);
  const requirementIds = validateStatements(
    definition.requirements,
    "requirements",
    true,
  );
  unique(requirementIds, "duplicate requirement identity");
  sorted(
    array(definition.requirements, "requirements", true),
    (value) => (value as { id: string }).id,
    "requirements",
  );
  const constraintIds = validateStatements(
    definition.preservation_constraints,
    "preservation_constraints",
    false,
  );
  unique(constraintIds, "duplicate constraint identity");
  sorted(
    array(
      definition.preservation_constraints,
      "preservation_constraints",
      false,
    ),
    (value) => (value as { id: string }).id,
    "preservation_constraints",
  );
  const proofs = array(definition.proof_obligations, "proof_obligations", true);
  const proofIds = proofs.map((item, index) => {
    const proof = object(item, `proof_obligations[${index}]`);
    exact(proof, [
      "proof_id",
      "statement",
      "obligation_ids",
      "evidence_kind",
      "command",
      "tool_identity",
      "configuration_digest",
    ]);
    identity(proof.proof_id, "proof_id");
    nonempty(proof.statement, "proof statement");
    stringArray(proof.obligation_ids, "obligation_ids", true, true);
    sorted(
      proof.obligation_ids as string[],
      (entry) => entry,
      "obligation_ids",
    );
    nonempty(proof.evidence_kind, "evidence_kind");
    validateCommand(proof.command);
    nonempty(proof.tool_identity, "tool_identity");
    digest(proof.configuration_digest, "configuration_digest");
    return proof.proof_id as string;
  });
  unique(proofIds, "duplicate proof identity");
  sorted(
    proofs,
    (value) => (value as { proof_id: string }).proof_id,
    "proof_obligations",
  );
  const unknowns = array(definition.unknowns, "unknowns", false);
  const unknownIds = unknowns.map((item, index) => {
    const unknown = object(item, `unknowns[${index}]`);
    const allowed = [
      "id",
      "statement",
      "disposition",
      "owner",
      ...(unknown.resolution === undefined ? [] : ["resolution"]),
    ];
    exact(unknown, allowed);
    identity(unknown.id, "unknown id");
    nonempty(unknown.statement, "unknown statement");
    nonempty(unknown.owner, "unknown owner");
    oneOf(
      unknown.disposition,
      ["open", "accepted", "deferred", "resolved"],
      "unknown disposition",
    );
    if (unknown.disposition === "resolved")
      nonempty(unknown.resolution, "unknown resolution");
    else if (unknown.resolution !== undefined)
      fail("non-resolved unknown resolution");
    return unknown.id as string;
  });
  unique(unknownIds, "duplicate unknown identity");
  sorted(unknowns, (value) => (value as { id: string }).id, "unknowns");

  const workflow = object(root.review_workflow, "review_workflow");
  exact(workflow, ["run_id", "decision_event_kind"]);
  identity(workflow.run_id, "run_id");
  equal(
    workflow.decision_event_kind,
    "change_assurance.review_decided",
    "decision_event_kind",
  );
}

function validateStatements(
  value: unknown,
  name: string,
  nonemptyArray: boolean,
): string[] {
  return array(value, name, nonemptyArray).map((item, index) => {
    const statement = object(item, `${name}[${index}]`);
    exact(statement, ["id", "statement", "source_ids"]);
    identity(statement.id, `${name} id`);
    nonempty(statement.statement, `${name} statement`);
    stringArray(statement.source_ids, `${name} source_ids`, true, true);
    sorted(
      statement.source_ids as string[],
      (entry) => entry,
      `${name} source_ids`,
    );
    return statement.id as string;
  });
}

export function validateCommand(value: unknown): void {
  const command = object(value, "command");
  exact(command, ["argv", "working_directory"]);
  const argv = array(command.argv, "argv", true);
  for (const entry of argv) if (typeof entry !== "string") fail("argv");
  const cwd = nonempty(command.working_directory, "working_directory");
  const segments = cwd.split("/");
  if (
    cwd !== "." &&
    (cwd.startsWith("/") ||
      cwd.includes("\\") ||
      cwd.endsWith("/") ||
      segments.some(
        (segment) => segment === "" || segment === ".." || segment === ".",
      ))
  ) {
    fail("working_directory must be normalized repository-relative POSIX path");
  }
}

const IX_FLOW_GENESIS_HASH = "0".repeat(64);

/** Pure local implementation of ix-flow FR-013's public event hash contract. */
export function hashIxFlowEvent(
  event: Omit<DecisionHistory["events"][number], "hash">,
): string {
  return createHash("sha256")
    .update(
      canonicalIxFlowJson({
        id: event.id,
        ts: event.ts,
        actor: event.actor,
        kind: event.kind,
        payload: event.payload,
        prevHash: event.prevHash,
      }),
    )
    .digest("hex");
}

export function verifyIxFlowChain(events: DecisionHistory["events"]): boolean {
  let previous = IX_FLOW_GENESIS_HASH;
  for (const event of events) {
    try {
      const raw = object(event, "ix-flow event");
      exact(raw, ["id", "ts", "actor", "kind", "payload", "prevHash", "hash"]);
      if (event.prevHash !== previous || event.hash !== hashIxFlowEvent(event))
        return false;
    } catch {
      return false;
    }
    previous = event.hash;
  }
  return true;
}

function canonicalIxFlowJson(value: unknown): string {
  if (Array.isArray(value))
    return `[${value.map(canonicalIxFlowJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const record = value as Record<string, unknown>;
    return `{${Object.keys(record)
      .sort()
      .map(
        (key) => `${JSON.stringify(key)}:${canonicalIxFlowJson(record[key])}`,
      )
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

export function object(value: unknown, name: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value))
    fail(`${name} object`);
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null)
    fail(`${name} plain object`);
  return value as Record<string, unknown>;
}

export function exact(value: Record<string, unknown>, allowed: string[]): void {
  const expected = new Set(allowed);
  for (const key of Object.keys(value))
    if (!expected.has(key)) fail(`extra field ${key}`);
  for (const key of expected) {
    if (!Object.hasOwn(value, key)) fail(`missing field ${key}`);
  }
}

export function array(
  value: unknown,
  name: string,
  nonemptyValue: boolean,
): unknown[] {
  if (!Array.isArray(value) || (nonemptyValue && value.length === 0))
    fail(name);
  return value;
}

export function nonempty(value: unknown, name: string): string {
  if (typeof value !== "string" || value.length === 0) fail(name);
  return value;
}

export function identity(value: unknown, name: string): void {
  if (typeof value !== "string" || !IDENTITY.test(value)) fail(name);
}

export function digest(value: unknown, name: string): void {
  if (typeof value !== "string" || !DIGEST.test(value)) fail(name);
}

function stringArray(
  value: unknown,
  name: string,
  nonemptyValue: boolean,
  uniqueValue: boolean,
): void {
  const values = array(value, name, nonemptyValue);
  for (const entry of values)
    if (typeof entry !== "string" || entry.length === 0) fail(name);
  if (uniqueValue) unique(values as string[], `duplicate ${name}`);
}

function unique(values: string[], message: string): void {
  if (new Set(values).size !== values.length) fail(message);
}

function sorted<T>(values: T[], key: (value: T) => string, name: string): void {
  for (let index = 1; index < values.length; index++) {
    if (compareUtf16(key(values[index - 1]), key(values[index])) > 0)
      fail(`${name} order`);
  }
}

function oneOf(value: unknown, values: readonly unknown[], name: string): void {
  if (!values.includes(value)) fail(name);
}

function equal(value: unknown, expected: unknown, name: string): void {
  if (value !== expected) fail(name);
}

function fail(message: string): never {
  throw new Error(`invalid change assurance record: ${message}`);
}

export function compareUtf16(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}
