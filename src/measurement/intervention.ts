import { createHash, randomUUID } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  renameSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname, isAbsolute, join, posix, relative, resolve } from "node:path";

import { Ajv2020 } from "ajv/dist/2020.js";

import { canonicalJson, storeRoot } from "../evidence/store.js";
import { interventionExperimentSchema } from "./intervention-schema.js";
import type { InterventionExperimentRecord } from "./intervention-types.js";
import { InterventionIntakeError } from "./intervention-types.js";
import { loadMeasurementPlans } from "./plans.js";

const ajv = new Ajv2020({
  allErrors: true,
  strict: false,
  formats: { "date-time": isRfc3339DateTime },
});
const validateSchema = ajv.compile(interventionExperimentSchema as object);

export function interventionsRoot(repo: string): string {
  return join(storeRoot(repo), "interventions");
}

export function interventionPath(repo: string, recordId: string): string {
  if (!/^[A-Za-z0-9._:-]+$/.test(recordId)) {
    throw new InterventionIntakeError("invalid_record", [
      `/record_id: unsafe record id ${JSON.stringify(recordId)}`,
    ]);
  }
  return join(interventionsRoot(repo), `${recordId}.json`);
}

export function validateInterventionRecord(
  value: unknown,
): asserts value is InterventionExperimentRecord {
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

/** Validate governance/raw bytes, then persist with one same-directory rename. */
export function writeInterventionRecord(
  repo: string,
  candidate: unknown,
): string {
  validateInterventionRecord(candidate);
  validateDefinition(repo, candidate.producer.definition_version);
  validateRawEvidence(repo, candidate);
  const path = interventionPath(repo, candidate.record_id);
  const bytes = canonicalJson(candidate);
  if (existsSync(path)) {
    if (readFileSync(path, "utf8") === bytes) return path;
    throw new InterventionIntakeError("record_id_collision", [
      `${path}: record id already exists with different canonical bytes`,
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

export function readInterventionRecords(
  repo: string,
): InterventionExperimentRecord[] {
  const root = interventionsRoot(repo);
  if (!existsSync(root)) return [];
  return readdirSync(root)
    .filter((name) => name.endsWith(".json"))
    .sort(compare)
    .map((name) => {
      const path = join(root, name);
      let value: unknown;
      try {
        value = JSON.parse(readFileSync(path, "utf8")) as unknown;
        validateInterventionRecord(value);
      } catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        throw new Error(`${path}: unreadable intervention record: ${detail}`);
      }
      return value;
    })
    .sort(
      (a, b) =>
        compare(a.observed_at, b.observed_at) ||
        compare(a.record_id, b.record_id),
    );
}

export function rawEvidenceFor(repo: string, path: string, mediaType: string) {
  const resolved = resolveRawPath(repo, path);
  const bytes = readFileSync(resolved);
  return {
    path,
    media_type: mediaType,
    size_bytes: bytes.byteLength,
    digest: `sha256:${createHash("sha256").update(bytes).digest("hex")}`,
  };
}

function validateDefinition(repo: string, observed: string): void {
  const active = loadMeasurementPlans(repo).filter(
    (plan) => plan.status === "active",
  );
  if (active.length === 0) {
    throw new InterventionIntakeError("governing_plan_absent", [
      `requested definition ${observed}; no active MeasurementPlan exists`,
    ]);
  }
  if (!active.some((plan) => plan.definitionVersion === observed)) {
    throw new InterventionIntakeError("definition_mismatch", [
      `requested definition ${observed}; expected one of ${active
        .map((plan) => plan.definitionVersion)
        .sort(compare)
        .join(", ")}; observed ${observed}`,
    ]);
  }
}

function validateRawEvidence(
  repo: string,
  record: InterventionExperimentRecord,
): void {
  const findings: string[] = [];
  for (const [index, reference] of record.raw_evidence.entries()) {
    try {
      const path = resolveRawPath(repo, reference.path);
      const bytes = readFileSync(path);
      const digest = `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
      if (bytes.byteLength !== reference.size_bytes) {
        findings.push(
          `/raw_evidence/${index}/size_bytes: expected ${reference.size_bytes}, observed ${bytes.byteLength}`,
        );
      }
      if (digest !== reference.digest) {
        findings.push(
          `/raw_evidence/${index}/digest: expected ${reference.digest}, observed ${digest}`,
        );
      }
    } catch (error) {
      findings.push(
        `/raw_evidence/${index}/path: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }
  if (findings.length > 0) {
    throw new InterventionIntakeError("raw_evidence_mismatch", findings);
  }
}

function resolveRawPath(repo: string, path: string): string {
  if (
    path.length === 0 ||
    isAbsolute(path) ||
    path.includes("\\") ||
    path.split("/").includes("..") ||
    posix.normalize(path) !== path ||
    path === "." ||
    path.endsWith("/")
  ) {
    throw new Error(`unsafe relative path ${JSON.stringify(path)}`);
  }
  const root = storeRoot(repo);
  const target = resolve(root, path);
  const rel = relative(resolve(root), target);
  if (
    rel === ".." ||
    rel.startsWith(`..${process.platform === "win32" ? "\\" : "/"}`) ||
    isAbsolute(rel)
  ) {
    throw new Error(`path escapes evidence store: ${path}`);
  }
  if (!existsSync(target) || !statSync(target).isFile()) {
    throw new Error(`retained evidence file is absent: ${path}`);
  }
  const realRoot = realpathSync(root);
  const realTarget = realpathSync(target);
  const realRel = relative(realRoot, realTarget);
  if (
    realRel === ".." ||
    realRel.startsWith(`..${process.platform === "win32" ? "\\" : "/"}`) ||
    isAbsolute(realRel)
  ) {
    throw new Error(`path resolves outside evidence store: ${path}`);
  }
  return target;
}

function semanticFindings(
  value: Record<string, unknown>,
  findings: string[],
): void {
  if (
    typeof value.observed_at === "string" &&
    !isRfc3339DateTime(value.observed_at)
  ) {
    findings.push("/observed_at: must be a valid date-time");
  }
  const design = asRecord(value.design);
  const assignment = asRecord(design?.assignment);
  if (
    design?.kind === "randomized" &&
    assignment?.method !== "randomized" &&
    assignment?.method !== "blocked_randomized"
  ) {
    findings.push(
      "/design/assignment/method: randomized design requires randomized assignment",
    );
  }
  if (
    (assignment?.method === "randomized" ||
      assignment?.method === "blocked_randomized") &&
    (typeof assignment.seed !== "string" || assignment.seed.length === 0)
  ) {
    findings.push(
      "/design/assignment/seed: randomized assignment requires a seed",
    );
  }

  const baseline = asRecord(value.baseline);
  const treatments = records(value.treatments);
  const treatmentIds = new Set<string>();
  if (typeof baseline?.id === "string") treatmentIds.add(baseline.id);
  for (const [index, treatment] of treatments.entries()) {
    if (typeof treatment.id !== "string") continue;
    if (treatmentIds.has(treatment.id)) {
      findings.push(
        `/treatments/${index}/id: duplicate arm id ${treatment.id}`,
      );
    }
    treatmentIds.add(treatment.id);
  }
  const actualTreatmentIds = new Set(
    treatments
      .map((item) => item.id)
      .filter((id): id is string => typeof id === "string"),
  );
  checkLinkedKeys(
    value.changed_variables,
    "name",
    "/changed_variables",
    actualTreatmentIds,
    findings,
  );
  checkLinkedKeys(
    value.measured_effects,
    "metric",
    "/measured_effects",
    actualTreatmentIds,
    findings,
  );

  const conclusion = asRecord(value.conclusion);
  if (
    (value.status === "failed" || value.status === "inconclusive") &&
    conclusion?.kind !== "cause_not_established"
  ) {
    findings.push(
      "/conclusion/kind: failed or inconclusive records require cause_not_established",
    );
  }
  if (
    conclusion?.kind === "causal_effect_established" ||
    conclusion?.kind === "no_effect_observed"
  ) {
    const arms = [baseline, ...treatments];
    if (
      arms.some(
        (arm) => typeof arm?.sample_size !== "number" || arm.sample_size < 1,
      )
    ) {
      findings.push(
        "/conclusion/kind: observed-effect conclusions require positive arm samples",
      );
    }
    const effects = records(value.measured_effects);
    if (
      effects.length === 0 ||
      effects.some(
        (effect) =>
          effect.baseline_value === null || effect.treatment_value === null,
      )
    ) {
      findings.push(
        "/measured_effects: observed-effect conclusions require observed comparisons",
      );
    }
    if (conclusion.kind === "causal_effect_established") {
      if (effects.some((effect) => effect.effect === null)) {
        findings.push(
          "/measured_effects: causal conclusions require non-null effects",
        );
      }
      if (conclusion.attribution_confidence === "none") {
        findings.push(
          "/conclusion/attribution_confidence: causal conclusion requires attributed confidence",
        );
      }
      const qualifiers = [
        ...records(value.interactions),
        ...records(value.confounders),
      ];
      if (
        qualifiers.some(
          (item) =>
            item.disposition === "uncontrolled" ||
            item.disposition === "unknown",
        )
      ) {
        findings.push(
          "/conclusion/kind: causal conclusion conflicts with uncontrolled or unknown qualifier",
        );
      }
    }
  }
}

function isRfc3339DateTime(value: string): boolean {
  const match =
    /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:Z|([+-])(\d{2}):(\d{2}))$/.exec(
      value,
    );
  if (!match) return false;
  const [, yearText, monthText, dayText, hourText, minuteText, secondText] =
    match;
  const year = Number(yearText);
  const month = Number(monthText);
  const day = Number(dayText);
  const hour = Number(hourText);
  const minute = Number(minuteText);
  const second = Number(secondText);
  const offsetHour = Number(match[8] ?? 0);
  const offsetMinute = Number(match[9] ?? 0);
  return (
    month >= 1 &&
    month <= 12 &&
    day >= 1 &&
    day <= daysInMonth(year, month) &&
    hour <= 23 &&
    minute <= 59 &&
    second <= 59 &&
    offsetHour <= 23 &&
    offsetMinute <= 59
  );
}

function daysInMonth(year: number, month: number): number {
  if (month === 2) {
    const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
    return leap ? 29 : 28;
  }
  return [4, 6, 9, 11].includes(month) ? 30 : 31;
}

function checkLinkedKeys(
  value: unknown,
  field: string,
  path: string,
  treatments: Set<string>,
  findings: string[],
): void {
  const seen = new Set<string>();
  for (const [index, item] of records(value).entries()) {
    if (
      typeof item.treatment_id !== "string" ||
      !treatments.has(item.treatment_id)
    ) {
      findings.push(
        `${path}/${index}/treatment_id: does not resolve to one treatment`,
      );
      continue;
    }
    const key = `${item.treatment_id}\0${String(item[field])}`;
    if (seen.has(key))
      findings.push(`${path}/${index}: duplicate treatment/${field} key`);
    seen.add(key);
  }
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

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
