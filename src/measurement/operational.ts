import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmdirSync,
} from "node:fs";
import { join } from "node:path";

import { Ajv2020 } from "ajv/dist/2020.js";

import { canonicalJson, storeRoot } from "../evidence/store.js";
import { writeFileAtomicNoReplace } from "./atomic-file.js";
import { parseRfc3339DateTime } from "./date-time.js";
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
ajv.addFormat("date-time", {
  type: "string",
  validate: (value: string) => parseRfc3339DateTime(value) !== null,
});
const validateSchema = ajv.compile(operationalEvidenceSchema as object);
const PIN_KINDS = new Set(["policy", "prompt", "model", "tool", "data"]);

interface RetainedOperationalEntry {
  record: OperationalEvidenceRecord;
  path: string;
}

export function operationalRoot(repo: string): string {
  return join(storeRoot(repo), "operational");
}

export function operationalPath(repo: string, recordId: string): string {
  return join(operationalRoot(repo), `${recordFileId(recordId)}.json`);
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
  return withOperationalWriteLock(repo, () => {
    const entries = readOperationalEntries(repo);
    const retained = entries.map((entry) => entry.record);
    validateForIntake(repo, candidate, retained);
    const identical = entries.find(
      (entry) =>
        entry.record.record_id === candidate.record_id &&
        canonicalJson(entry.record) === canonicalJson(candidate),
    );
    if (identical) return identical.path;
    return writeOne(repo, candidate);
  });
}

/** Persist a linked capability/exercise pair as one atomic no-replace file. */
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
  return withOperationalWriteLock(repo, () => {
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
    return writeAtomic(path, bytes);
  });
}

export function readOperationalRecords(
  repo: string,
): OperationalEvidenceRecord[] {
  return readOperationalEntries(repo).map((entry) => entry.record);
}

function readOperationalEntries(repo: string): RetainedOperationalEntry[] {
  const root = operationalRoot(repo);
  if (!existsSync(root)) return [];
  const entries: RetainedOperationalEntry[] = [];
  for (const name of readdirSync(root)
    .filter((item) => item.endsWith(".json"))
    .sort(compare)) {
    const path = join(root, name);
    entries.push({ path, record: readRecord(path) });
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
        entries.push({ path, record: value });
      }
    }
  }
  const seen = new Set<string>();
  for (const entry of entries) {
    if (seen.has(entry.record.record_id)) {
      throw new Error(
        `duplicate retained operational record id ${entry.record.record_id}`,
      );
    }
    seen.add(entry.record.record_id);
  }
  return entries.sort(
    (a, b) =>
      compare(a.record.observed_at, b.record.observed_at) ||
      compare(a.record.record_id, b.record.record_id),
  );
}

export function operationalDischarge(
  exercise: OperationalExerciseRecord,
  obligation: OperationalObligation,
): { discharged: boolean; reason: string } {
  try {
    validateOperationalRecord(exercise);
  } catch (error) {
    return no(
      `invalid operational exercise: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
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
  const obligationStart = parseRfc3339DateTime(obligation.clock?.started_at);
  const obligationDeadline = parseRfc3339DateTime(
    obligation.clock?.deadline_at,
  );
  if (
    obligation.clock?.applicability !== "operational_with_clock" ||
    obligationStart === null ||
    obligationDeadline === null ||
    obligationStart > obligationDeadline
  ) {
    return no("invalid obligation clock condition");
  }
  const clock = exercise.exercise.clock;
  if (clock.applicability !== "operational_with_clock") {
    return no(`clock status ${clock.status}`);
  }
  const clockStart = parseRfc3339DateTime(clock.started_at);
  const clockDeadline = parseRfc3339DateTime(clock.deadline_at);
  const clockCompletion = parseRfc3339DateTime(clock.completed_at);
  if (clockStart !== obligationStart || clockDeadline !== obligationDeadline) {
    return no("obligation clock condition mismatch");
  }
  if (
    clockCompletion === null ||
    clockCompletion < obligationStart ||
    clockCompletion > obligationDeadline
  ) {
    return no("exercise did not complete within obligation clock");
  }
  const derived = deriveClockStatus(
    clock,
    parseRfc3339DateTime(exercise.observed_at),
  );
  if (derived !== "met") {
    return no(`derived clock status ${derived}`);
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
  return writeAtomic(path, canonicalJson(record));
}

function writeAtomic(path: string, bytes: string): string {
  return writeFileAtomicNoReplace(
    path,
    bytes,
    (collisionPath) =>
      new InterventionIntakeError("record_id_collision", [
        `${collisionPath}: retained bytes differ`,
      ]),
  );
}

function withOperationalWriteLock<T>(repo: string, operation: () => T): T {
  const lock = join(storeRoot(repo), ".operational-write.lock");
  mkdirSync(storeRoot(repo), { recursive: true });
  const deadline = Date.now() + 10_000;
  while (true) {
    try {
      mkdirSync(lock);
      break;
    } catch (error) {
      if (!isNodeError(error) || error.code !== "EEXIST") throw error;
      if (Date.now() >= deadline) {
        throw new InterventionIntakeError("intake_busy", [
          `${lock}: timed out waiting for the store-wide operational intake lock; fail closed and remove it only after confirming no writer is active`,
        ]);
      }
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 5);
    }
  }
  try {
    return operation();
  } finally {
    rmdirSync(lock);
  }
}

function isNodeError(value: unknown): value is NodeJS.ErrnoException {
  return value instanceof Error && "code" in value;
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
  const derived = deriveClockStatus(clock, observed);
  if (clock.status !== derived) {
    findings.push(
      `/exercise/clock/status: ${String(clock.status)} disagrees with derived ${derived}`,
    );
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

function recordFileId(value: string): string {
  if (!/^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$/.test(value)) {
    throw new InterventionIntakeError("invalid_record", [
      `unsafe record id ${value}`,
    ]);
  }
  return createHash("sha256").update(value).digest("hex");
}

function date(value: unknown, path: string, findings: string[]): number | null {
  const parsed = parseRfc3339DateTime(value);
  if (parsed === null) {
    findings.push(`${path}: must be an RFC 3339 date-time`);
    return null;
  }
  return parsed;
}

function deriveClockStatus(
  clock: Record<string, unknown>,
  observed: number | null,
): "open" | "met" | "missed" | "unknown" {
  const deadline = parseRfc3339DateTime(clock.deadline_at);
  const completed =
    clock.completed_at === undefined
      ? null
      : parseRfc3339DateTime(clock.completed_at);
  return completed !== null && deadline !== null
    ? completed <= deadline
      ? "met"
      : "missed"
    : completed === null && observed !== null && deadline !== null
      ? observed <= deadline
        ? "open"
        : "missed"
      : "unknown";
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
