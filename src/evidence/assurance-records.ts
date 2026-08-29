/**
 * Content-addressed, append-only experiment and operational evidence (FR-048).
 *
 * These records have their own producer-provenance contract. They do not alter
 * FR-030 run records: the accepted provenance boundary leaves those records
 * unchanged and binds richer provenance only where it is part of the record's
 * declared schema.
 */

import { createHash, randomUUID } from "node:crypto";
import {
  existsSync,
  linkSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";

import { canonicalJson, storeRoot } from "./store.js";

export const EXPERIMENTS_DIR = "experiments";
export const OPERATIONAL_EVIDENCE_DIR = "operational";

export interface ProducerProvenance {
  schemaVersion: "producer-provenance-v1";
  identity: string;
  version: string;
  sourceRevision: string;
  sourceState: "clean" | "dirty";
  executableDigest: string;
  configurationDigest: string;
  capabilities: string[];
  artifacts: Array<{ name: string; digest: string }>;
}

export interface EvidenceSubject {
  kind: string;
  id: string;
  sourceRevision: string;
}

export interface ExperimentRecordInput {
  schemaVersion: "experiment-record-v1";
  subject: EvidenceSubject;
  recordedAt: string;
  hypothesis: string;
  design: {
    timeBox: string;
    corpusRefs: string[];
    comparisonMethod: string;
    decisionRule: string;
  };
  result: {
    status: "supported" | "not_supported" | "inconclusive";
    summary: string;
    evidenceRefs: string[];
  };
  producerProvenance: ProducerProvenance;
}

export interface ExperimentRecord extends ExperimentRecordInput {
  recordId: string;
}

export interface OperationalEvidenceRecordInput {
  schemaVersion: "operational-evidence-record-v1";
  subject: EvidenceSubject;
  recordedAt: string;
  window: { startedAt: string; endedAt: string };
  environment: string;
  observations: Array<{
    signal: string;
    value: string;
    unit?: string;
    interpretation: string;
    evidenceRefs: string[];
  }>;
  outcome: "within_bounds" | "outside_bounds" | "inconclusive";
  producerProvenance: ProducerProvenance;
}

export interface OperationalEvidenceRecord extends OperationalEvidenceRecordInput {
  recordId: string;
}

export type AssuranceEvidenceRecord =
  ExperimentRecord | OperationalEvidenceRecord;

export interface StoredAssuranceRecord<T extends AssuranceEvidenceRecord> {
  record: T;
  path: string;
  created: boolean;
}

export function experimentRecordPath(repo: string, recordId: string): string {
  return recordPath(repo, EXPERIMENTS_DIR, recordId);
}

export function operationalEvidenceRecordPath(
  repo: string,
  recordId: string,
): string {
  return recordPath(repo, OPERATIONAL_EVIDENCE_DIR, recordId);
}

export function writeExperimentRecord(
  repo: string,
  input: unknown,
): StoredAssuranceRecord<ExperimentRecord> {
  const parsed = parseExperimentRecordInput(input);
  const record = identify(parsed);
  const outcome = publish(experimentRecordPath(repo, record.recordId), record);
  return { record, ...outcome };
}

export function writeOperationalEvidenceRecord(
  repo: string,
  input: unknown,
): StoredAssuranceRecord<OperationalEvidenceRecord> {
  const parsed = parseOperationalEvidenceRecordInput(input);
  const record = identify(parsed);
  const outcome = publish(
    operationalEvidenceRecordPath(repo, record.recordId),
    record,
  );
  return { record, ...outcome };
}

export function readExperimentRecord(
  repo: string,
  recordId: string,
): ExperimentRecord | null {
  return readRecord(experimentRecordPath(repo, recordId), recordId, (value) => {
    const object = objectAt("experiment record", value);
    const { recordId: _recordId, ...input } = object;
    return {
      ...parseExperimentRecordInput(input),
      recordId: stringValue("recordId", _recordId),
    };
  });
}

export function readOperationalEvidenceRecord(
  repo: string,
  recordId: string,
): OperationalEvidenceRecord | null {
  return readRecord(
    operationalEvidenceRecordPath(repo, recordId),
    recordId,
    (value) => {
      const object = objectAt("operational evidence record", value);
      const { recordId: _recordId, ...input } = object;
      return {
        ...parseOperationalEvidenceRecordInput(input),
        recordId: stringValue("recordId", _recordId),
      };
    },
  );
}

export function listExperimentRecords(repo: string): string[] {
  return listRecords(repo, EXPERIMENTS_DIR);
}

export function listOperationalEvidenceRecords(repo: string): string[] {
  return listRecords(repo, OPERATIONAL_EVIDENCE_DIR);
}

export function parseExperimentRecordInput(
  value: unknown,
): ExperimentRecordInput {
  const root = objectAt("experiment record", value);
  exact(root, "experiment record", [
    "schemaVersion",
    "subject",
    "recordedAt",
    "hypothesis",
    "design",
    "result",
    "producerProvenance",
  ]);
  literal(root.schemaVersion, "schemaVersion", [
    "experiment-record-v1",
  ] as const);
  const design = objectAt("design", root.design);
  exact(design, "design", [
    "timeBox",
    "corpusRefs",
    "comparisonMethod",
    "decisionRule",
  ]);
  const result = objectAt("result", root.result);
  exact(result, "result", ["status", "summary", "evidenceRefs"]);
  const recordedAt = stringValue("recordedAt", root.recordedAt);
  instant("recordedAt", recordedAt);
  return {
    schemaVersion: "experiment-record-v1",
    subject: parseSubject(root.subject),
    recordedAt,
    hypothesis: stringValue("hypothesis", root.hypothesis),
    design: {
      timeBox: stringValue("timeBox", design.timeBox),
      corpusRefs: strings("corpusRefs", design.corpusRefs, true),
      comparisonMethod: stringValue(
        "comparisonMethod",
        design.comparisonMethod,
      ),
      decisionRule: stringValue("decisionRule", design.decisionRule),
    },
    result: {
      status: literal(result.status, "result.status", [
        "supported",
        "not_supported",
        "inconclusive",
      ] as const),
      summary: stringValue("result.summary", result.summary),
      evidenceRefs: strings("result.evidenceRefs", result.evidenceRefs, true),
    },
    producerProvenance: parseProducerProvenance(root.producerProvenance),
  };
}

export function parseOperationalEvidenceRecordInput(
  value: unknown,
): OperationalEvidenceRecordInput {
  const root = objectAt("operational evidence record", value);
  exact(root, "operational evidence record", [
    "schemaVersion",
    "subject",
    "recordedAt",
    "window",
    "environment",
    "observations",
    "outcome",
    "producerProvenance",
  ]);
  literal(root.schemaVersion, "schemaVersion", [
    "operational-evidence-record-v1",
  ] as const);
  const recordedAt = stringValue("recordedAt", root.recordedAt);
  instant("recordedAt", recordedAt);
  const window = objectAt("window", root.window);
  exact(window, "window", ["startedAt", "endedAt"]);
  const startedAt = stringValue("window.startedAt", window.startedAt);
  const endedAt = stringValue("window.endedAt", window.endedAt);
  if (
    instant("window.endedAt", endedAt) <= instant("window.startedAt", startedAt)
  ) {
    throw new Error("window.endedAt must be after window.startedAt");
  }
  if (!Array.isArray(root.observations) || root.observations.length === 0) {
    throw new Error("observations must be a non-empty array");
  }
  const observations = root.observations.map((value, index) => {
    const observation = objectAt(`observations[${index}]`, value);
    exact(
      observation,
      `observations[${index}]`,
      ["signal", "value", "unit", "interpretation", "evidenceRefs"],
      ["unit"],
    );
    const unit = optionalString("unit", observation.unit);
    return {
      signal: stringValue("signal", observation.signal),
      value: stringValue("value", observation.value),
      ...(unit ? { unit } : {}),
      interpretation: stringValue("interpretation", observation.interpretation),
      evidenceRefs: strings("evidenceRefs", observation.evidenceRefs, true),
    };
  });
  return {
    schemaVersion: "operational-evidence-record-v1",
    subject: parseSubject(root.subject),
    recordedAt,
    window: { startedAt, endedAt },
    environment: stringValue("environment", root.environment),
    observations,
    outcome: literal(root.outcome, "outcome", [
      "within_bounds",
      "outside_bounds",
      "inconclusive",
    ] as const),
    producerProvenance: parseProducerProvenance(root.producerProvenance),
  };
}

export function parseProducerProvenance(value: unknown): ProducerProvenance {
  const root = objectAt("producerProvenance", value);
  exact(root, "producerProvenance", [
    "schemaVersion",
    "identity",
    "version",
    "sourceRevision",
    "sourceState",
    "executableDigest",
    "configurationDigest",
    "capabilities",
    "artifacts",
  ]);
  literal(root.schemaVersion, "producerProvenance.schemaVersion", [
    "producer-provenance-v1",
  ] as const);
  const artifactsValue = root.artifacts;
  if (!Array.isArray(artifactsValue)) {
    throw new Error("producerProvenance.artifacts must be an array");
  }
  const artifacts = artifactsValue.map((value, index) => {
    const artifact = objectAt(`artifacts[${index}]`, value);
    exact(artifact, `artifacts[${index}]`, ["name", "digest"]);
    const digest = stringValue("artifact.digest", artifact.digest);
    sha256("artifact.digest", digest);
    return { name: stringValue("artifact.name", artifact.name), digest };
  });
  const executableDigest = stringValue(
    "producerProvenance.executableDigest",
    root.executableDigest,
  );
  const configurationDigest = stringValue(
    "producerProvenance.configurationDigest",
    root.configurationDigest,
  );
  sha256("producerProvenance.executableDigest", executableDigest);
  sha256("producerProvenance.configurationDigest", configurationDigest);
  return {
    schemaVersion: "producer-provenance-v1",
    identity: stringValue("producerProvenance.identity", root.identity),
    version: stringValue("producerProvenance.version", root.version),
    sourceRevision: stringValue(
      "producerProvenance.sourceRevision",
      root.sourceRevision,
    ),
    sourceState: literal(root.sourceState, "producerProvenance.sourceState", [
      "clean",
      "dirty",
    ] as const),
    executableDigest,
    configurationDigest,
    capabilities: strings(
      "producerProvenance.capabilities",
      root.capabilities,
      false,
    ),
    artifacts,
  };
}

function parseSubject(value: unknown): EvidenceSubject {
  const subject = objectAt("subject", value);
  exact(subject, "subject", ["kind", "id", "sourceRevision"]);
  return {
    kind: stringValue("subject.kind", subject.kind),
    id: stringValue("subject.id", subject.id),
    sourceRevision: stringValue(
      "subject.sourceRevision",
      subject.sourceRevision,
    ),
  };
}

function identify<
  T extends ExperimentRecordInput | OperationalEvidenceRecordInput,
>(input: T): T & { recordId: string } {
  const digest = createHash("sha256")
    .update(canonicalJson(input))
    .digest("hex");
  return { ...input, recordId: `sha256:${digest}` };
}

function publish(
  path: string,
  record: AssuranceEvidenceRecord,
): { path: string; created: boolean } {
  const bytes = canonicalJson(record);
  mkdirSync(dirname(path), { recursive: true });
  if (existsSync(path)) return compareExisting(path, bytes);
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  writeFileSync(temporary, bytes, { encoding: "utf8", flag: "wx" });
  try {
    try {
      linkSync(temporary, path);
      return { path, created: true };
    } catch (error) {
      if (existsSync(path)) return compareExisting(path, bytes);
      throw error;
    }
  } finally {
    if (existsSync(temporary)) unlinkSync(temporary);
  }
}

function compareExisting(
  path: string,
  expected: string,
): { path: string; created: false } {
  const existing = readFileSync(path, "utf8");
  if (existing !== expected) {
    throw new Error(
      `content-address collision or corrupt immutable record at ${path}; existing bytes were not overwritten`,
    );
  }
  return { path, created: false };
}

function readRecord<T extends AssuranceEvidenceRecord>(
  path: string,
  expectedId: string,
  parse: (value: unknown) => T,
): T | null {
  if (!existsSync(path)) return null;
  const record = parse(JSON.parse(readFileSync(path, "utf8")) as unknown);
  if (record.recordId !== expectedId) {
    throw new Error(`record id does not match path at ${path}`);
  }
  const input = Object.fromEntries(
    Object.entries(record).filter(([key]) => key !== "recordId"),
  );
  const digest = createHash("sha256")
    .update(canonicalJson(input))
    .digest("hex");
  if (`sha256:${digest}` !== record.recordId) {
    throw new Error(`record digest does not match content at ${path}`);
  }
  return record;
}

function recordPath(repo: string, directory: string, recordId: string): string {
  sha256("recordId", recordId);
  return join(
    storeRoot(repo),
    directory,
    `sha256-${recordId.slice("sha256:".length)}.json`,
  );
}

function listRecords(repo: string, directory: string): string[] {
  const path = join(storeRoot(repo), directory);
  if (!existsSync(path)) return [];
  return readdirSync(path)
    .filter((name) => /^sha256-[0-9a-f]{64}\.json$/.test(name))
    .sort()
    .map((name) => `sha256:${name.slice(7, -5)}`);
}

function objectAt(name: string, value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${name} must be an object`);
  }
  return value as Record<string, unknown>;
}

function exact(
  value: Record<string, unknown>,
  name: string,
  allowed: string[],
  optional: string[] = [],
): void {
  const keys = new Set(allowed);
  for (const key of Object.keys(value)) {
    if (!keys.has(key)) throw new Error(`${name} has unknown field ${key}`);
  }
  const optionalKeys = new Set(optional);
  for (const key of allowed) {
    if (!optionalKeys.has(key) && !(key in value)) {
      throw new Error(`${name} is missing ${key}`);
    }
  }
}

function stringValue(name: string, value: unknown): string {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${name} must be a non-empty string`);
  }
  return value;
}

function optionalString(name: string, value: unknown): string | undefined {
  if (value === undefined) return undefined;
  return stringValue(name, value);
}

function strings(name: string, value: unknown, nonEmpty: boolean): string[] {
  if (!Array.isArray(value) || (nonEmpty && value.length === 0)) {
    throw new Error(
      `${name} must be ${nonEmpty ? "a non-empty " : "an "}array`,
    );
  }
  const result = value.map((item) => stringValue(name, item));
  if (new Set(result).size !== result.length) {
    throw new Error(`${name} must contain unique values`);
  }
  return result;
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

function sha256(name: string, value: string): void {
  if (!/^sha256:[0-9a-f]{64}$/.test(value)) {
    throw new Error(`${name} must be sha256:<64 lowercase hex>`);
  }
}
