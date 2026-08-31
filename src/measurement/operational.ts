import { createHash, randomUUID } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";

import { Ajv2020 } from "ajv/dist/2020.js";

import { canonicalJson, storeRoot } from "../evidence/store.js";
import {
  assertGoverningDefinition,
  verifyRawEvidenceReferences,
} from "./intervention.js";
import { InterventionIntakeError } from "./intervention-types.js";
import { operationalEvidenceSchema } from "./operational-schema.js";
import type {
  OperationalEvidenceRecord,
  OperationalExerciseRecord,
  OperationalObligation,
  StandingCapabilityRecord,
} from "./operational-types.js";

const ajv = new Ajv2020({ allErrors: true, strict: false });
const validateSchema = ajv.compile(operationalEvidenceSchema as object);
const PIN_KINDS = new Set(["policy", "prompt", "model", "tool", "data"]);

export function operationalRoot(repo: string): string {
  return join(storeRoot(repo), "operational");
}

export function operationalPath(repo: string, recordId: string): string {
  return join(operationalRoot(repo), `${safeId(recordId)}.json`);
}

export function validateOperationalRecord(
  value: unknown,
): asserts value is OperationalEvidenceRecord {
  const findings: string[] = [];
  if (!validateSchema(value)) {
    findings.push(
      ...(validateSchema.errors ?? []).map(
        (error) =>
          `${error.instancePath || "/"}: ${error.message ?? "invalid"}`,
      ),
    );
  }
  if (isRecord(value)) semanticFindings(value, findings);
  if (findings.length > 0) {
    throw new InterventionIntakeError(
      "invalid_record",
      [...new Set(findings)].sort(compare),
    );
  }
}

export function writeOperationalRecord(
  repo: string,
  candidate: unknown,
): string {
  validateOperationalRecord(candidate);
  validateForIntake(repo, candidate, readOperationalRecords(repo));
  return writeOne(repo, candidate);
}

/** Persist a linked capability/exercise pair as one canonical file and rename. */
export function writeOperationalPair(
  repo: string,
  capability: unknown,
  exercise: unknown,
): string {
  validateOperationalRecord(capability);
  validateOperationalRecord(exercise);
  if (
    capability.record_shape !== "standing_capability" ||
    exercise.record_shape !== "exercise"
  ) {
    throw new InterventionIntakeError("invalid_record", [
      "operational pair requires one standing capability followed by one exercise",
    ]);
  }
  const existing = readOperationalRecords(repo);
  const pairId = createHash("sha256")
    .update(`${capability.record_id}\0${exercise.record_id}`)
    .digest("hex");
  const path = join(operationalRoot(repo), "pairs", `${pairId}.json`);
  const bytes = canonicalJson({
    schema_version: 1,
    records: [capability, exercise],
  });
  validateForIntake(repo, capability, existing);
  validateForIntake(repo, exercise, [...existing, capability]);
  if (existsSync(path)) {
    if (readFileSync(path, "utf8") === bytes) return path;
    throw new InterventionIntakeError("record_id_collision", [path]);
  }
  for (const record of [capability, exercise]) {
    if (existing.some((item) => item.record_id === record.record_id)) {
      throw new InterventionIntakeError("record_id_collision", [
        record.record_id,
      ]);
    }
  }
  return atomicWrite(path, bytes, "record_id_collision");
}

export function readOperationalRecords(
  repo: string,
): OperationalEvidenceRecord[] {
  const root = operationalRoot(repo);
  if (!existsSync(root)) return [];
  const values: OperationalEvidenceRecord[] = [];
  for (const name of readdirSync(root)
    .filter((item) => item.endsWith(".json"))
    .sort(compare)) {
    values.push(readRecord(join(root, name)));
  }
  const pairs = join(root, "pairs");
  if (existsSync(pairs)) {
    for (const name of readdirSync(pairs)
      .filter((item) => item.endsWith(".json"))
      .sort(compare)) {
      const path = join(pairs, name);
      const envelope = JSON.parse(readFileSync(path, "utf8")) as {
        records?: unknown;
      };
      if (!Array.isArray(envelope.records) || envelope.records.length !== 2) {
        throw new Error(
          `${path}: unreadable operational pair: expected two records`,
        );
      }
      for (const value of envelope.records) {
        validateOperationalRecord(value);
        values.push(value);
      }
    }
  }
  const seen = new Set<string>();
  for (const value of values) {
    if (seen.has(value.record_id)) {
      throw new Error(
        `duplicate retained operational record id ${value.record_id}`,
      );
    }
    seen.add(value.record_id);
  }
  return values.sort(
    (a, b) =>
      compare(a.observed_at, b.observed_at) ||
      compare(a.record_id, b.record_id),
  );
}

export function operationalDischarge(
  exercise: OperationalExerciseRecord,
  obligation: OperationalObligation,
): { discharged: boolean; reason: string } {
  if (exercise.control_kind !== obligation.control_kind)
    return no("control_kind mismatch");
  if (canonicalJson(exercise.subject) !== canonicalJson(obligation.subject))
    return no("subject mismatch");
  if (canonicalJson(exercise.scope) !== canonicalJson(obligation.scope))
    return no("scope mismatch");
  if (!obligation.accepted_modes.includes(exercise.exercise.mode))
    return no("exercise mode mismatch");
  if (exercise.exercise.outcome !== "succeeded")
    return no(`exercise outcome ${exercise.exercise.outcome}`);
  if (
    exercise.exercise.clock.applicability !== "operational_with_clock" ||
    exercise.exercise.clock.status !== "met"
  ) {
    return no(`clock status ${exercise.exercise.clock.status}`);
  }
  return {
    discharged: true,
    reason: "matched succeeded exercise completed within clock",
  };
}

function validateForIntake(
  repo: string,
  record: OperationalEvidenceRecord,
  retained: OperationalEvidenceRecord[],
): void {
  assertGoverningDefinition(repo, record.producer.definition_version);
  verifyRawEvidenceReferences(repo, record.raw_evidence);
  const sameId = retained.find((item) => item.record_id === record.record_id);
  if (sameId && canonicalJson(sameId) !== canonicalJson(record)) {
    throw new InterventionIntakeError("record_id_collision", [
      record.record_id,
    ]);
  }
  if (
    record.record_shape === "exercise" &&
    record.exercise.capability_record_id
  ) {
    const capability = retained.find(
      (item): item is StandingCapabilityRecord =>
        item.record_shape === "standing_capability" &&
        item.record_id === record.exercise.capability_record_id,
    );
    if (!capability || !linked(capability, record)) {
      throw new InterventionIntakeError("invalid_record", [
        "/exercise/capability_record_id: linked capability does not match control, kind, subject, and scope",
      ]);
    }
  }
}

function writeOne(repo: string, record: OperationalEvidenceRecord): string {
  const path = operationalPath(repo, record.record_id);
  return atomicWrite(path, canonicalJson(record), "record_id_collision");
}

function atomicWrite(
  path: string,
  bytes: string,
  collisionCode: "record_id_collision",
): string {
  if (existsSync(path)) {
    if (readFileSync(path, "utf8") === bytes) return path;
    throw new InterventionIntakeError(collisionCode, [
      `${path}: retained bytes differ`,
    ]);
  }
  mkdirSync(dirname(path), { recursive: true });
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  try {
    writeFileSync(temporary, bytes, { encoding: "utf8", flag: "wx" });
    renameSync(temporary, path);
  } finally {
    if (existsSync(temporary)) unlinkSync(temporary);
  }
  return path;
}

function readRecord(path: string): OperationalEvidenceRecord {
  try {
    const value = JSON.parse(readFileSync(path, "utf8")) as unknown;
    validateOperationalRecord(value);
    return value;
  } catch (error) {
    throw new Error(
      `${path}: unreadable operational record: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

function semanticFindings(
  value: Record<string, unknown>,
  findings: string[],
): void {
  const observed = date(value.observed_at, "/observed_at", findings);
  const capability = asRecord(value.capability);
  const exercise = asRecord(value.exercise);
  if (value.record_shape === "standing_capability") {
    if (!capability || exercise)
      findings.push("/: standing_capability requires only capability payload");
    const support = asRecord(capability?.clock_support);
    if (support?.supported === true) {
      if (
        !support.start_event ||
        !support.completion_event ||
        !positiveInteger(support.deadline_seconds)
      ) {
        findings.push(
          "/capability/clock_support: supported clock requires events and positive deadline",
        );
      }
    } else if (
      support?.supported === false &&
      Object.keys(support).length !== 1
    ) {
      findings.push(
        "/capability/clock_support: unsupported clock excludes event/deadline fields",
      );
    }
  } else if (value.record_shape === "exercise") {
    if (!exercise || capability)
      findings.push("/: exercise requires only exercise payload");
    const started = date(
      exercise?.started_at,
      "/exercise/started_at",
      findings,
    );
    const completed = date(
      exercise?.completed_at,
      "/exercise/completed_at",
      findings,
    );
    if (started !== null && completed !== null && completed < started) {
      findings.push("/exercise/completed_at: precedes exercise start");
    }
    if (observed !== null && completed !== null && observed < completed) {
      findings.push("/observed_at: precedes exercise completion");
    }
    if (exercise) checkClock(value, exercise, observed, findings);
  }

  const configuration = asRecord(value.configuration);
  const pins = records(configuration?.version_pins);
  const seen = new Set<string>();
  for (const [index, pin] of pins.entries()) {
    const key = `${String(pin.kind)}\0${String(pin.identity)}`;
    if (seen.has(key))
      findings.push(
        `/configuration/version_pins/${index}: duplicate kind/identity`,
      );
    seen.add(key);
  }
  if (
    typeof value.control_kind === "string" &&
    value.control_kind.endsWith("_pin")
  ) {
    const expected = value.control_kind.replace(/_pin$/, "");
    if (
      !PIN_KINDS.has(expected) ||
      !pins.some((pin) => pin.kind === expected)
    ) {
      findings.push(
        `/configuration/version_pins: ${value.control_kind} requires a matching pin kind`,
      );
    }
  }
}

function checkClock(
  record: Record<string, unknown>,
  exercise: Record<string, unknown>,
  observed: number | null,
  findings: string[],
): void {
  const clock = asRecord(exercise.clock);
  if (!clock) return;
  if (clock.applicability === "not_applicable") {
    if (clock.status !== "not_applicable" || Object.keys(clock).length !== 2) {
      findings.push(
        "/exercise/clock: not_applicable excludes timestamps and requires matching status",
      );
    }
    return;
  }
  if (clock.applicability !== "operational_with_clock") return;
  const started = date(
    clock.started_at,
    "/exercise/clock/started_at",
    findings,
  );
  const deadline = date(
    clock.deadline_at,
    "/exercise/clock/deadline_at",
    findings,
  );
  const completed =
    clock.completed_at === undefined
      ? null
      : date(clock.completed_at, "/exercise/clock/completed_at", findings);
  if (started !== null && deadline !== null && started > deadline) {
    findings.push("/exercise/clock/deadline_at: precedes clock start");
  }
  if (started !== null && completed !== null && completed < started) {
    findings.push("/exercise/clock/completed_at: precedes clock start");
  }
  const derived =
    completed !== null && deadline !== null
      ? completed <= deadline
        ? "met"
        : "missed"
      : completed === null && observed !== null && deadline !== null
        ? observed <= deadline
          ? "open"
          : "missed"
        : "unknown";
  if (
    clock.status !== derived &&
    !(
      clock.status === "unknown" &&
      Array.isArray(record.gaps) &&
      record.gaps.length > 0
    )
  ) {
    findings.push(
      `/exercise/clock/status: ${String(clock.status)} disagrees with derived ${derived}`,
    );
  }
  if (
    clock.status === "unknown" &&
    (!Array.isArray(record.gaps) || record.gaps.length === 0)
  ) {
    findings.push("/gaps: unknown clock status requires a declared gap");
  }
}

function linked(
  capability: StandingCapabilityRecord,
  exercise: OperationalExerciseRecord,
): boolean {
  return (
    capability.capability.control_id === exercise.exercise.control_id &&
    capability.control_kind === exercise.control_kind &&
    canonicalJson(capability.subject) === canonicalJson(exercise.subject) &&
    canonicalJson(capability.scope) === canonicalJson(exercise.scope)
  );
}

function safeId(value: string): string {
  if (!/^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$/.test(value)) {
    throw new InterventionIntakeError("invalid_record", [
      `unsafe record id ${value}`,
    ]);
  }
  return value.replaceAll("/", "%2F");
}

function date(value: unknown, path: string, findings: string[]): number | null {
  const parsed = typeof value === "string" ? Date.parse(value) : Number.NaN;
  if (!Number.isFinite(parsed)) {
    findings.push(`${path}: must be a valid date-time`);
    return null;
  }
  return parsed;
}

function positiveInteger(value: unknown): boolean {
  return Number.isInteger(value) && Number(value) > 0;
}

function records(value: unknown): Array<Record<string, unknown>> {
  return Array.isArray(value) ? value.filter(isRecord) : [];
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return isRecord(value) ? value : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function no(reason: string): { discharged: false; reason: string } {
  return { discharged: false, reason };
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
