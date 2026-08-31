import { readFileSync } from "node:fs";
import { join } from "node:path";

import { parse as parseYaml } from "yaml";

import { rawEvidenceFor } from "./intervention.js";
import { writeOperationalPair } from "./operational.js";
import type {
  GitHubReleaseProducerDefinition,
  OperationalExerciseRecord,
  StandingCapabilityRecord,
} from "./operational-types.js";

export function produceGitHubReleaseOperational(
  repo: string,
  definition: GitHubReleaseProducerDefinition,
): {
  path: string;
  capability: StandingCapabilityRecord;
  exercise: OperationalExerciseRecord;
} {
  validateDefinition(definition);
  const workflowRaw = readRetained(repo, definition.workflow_evidence_path);
  const runRaw = readRetained(repo, definition.run_evidence_path);
  const jobsRaw = readRetained(repo, definition.jobs_evidence_path);
  const workflow = parseYaml(workflowRaw) as unknown;
  const run = parseJson(runRaw, "workflow-run");
  const jobs = parseJson(jobsRaw, "workflow-jobs");
  const workflowRoot = requireRecord(workflow, "workflow");
  const workflowJobs = requireRecord(workflowRoot.jobs, "workflow.jobs");
  const configuredJobs = Object.entries(workflowJobs).filter(([key, value]) => {
    const job = isRecord(value) ? value : {};
    return (
      job.name === definition.release_job || key === definition.release_job
    );
  });
  if (configuredJobs.length !== 1) {
    throw new Error(
      `workflow must contain exactly one configured release job ${definition.release_job}`,
    );
  }
  const triggers = requireRecord(workflowRoot.on, "workflow.on");
  if (!(definition.accepted_event in triggers)) {
    throw new Error(
      `workflow does not declare accepted event ${definition.accepted_event}`,
    );
  }
  if (
    run.path !== definition.workflow_path ||
    run.event !== definition.accepted_event ||
    run.head_sha !== definition.subject.revision
  ) {
    throw new Error(
      "workflow run path, event, or immutable source revision mismatch",
    );
  }
  if (run.status !== "completed" || typeof run.conclusion !== "string") {
    throw new Error("workflow run is not completed with a conclusion");
  }
  if (!Array.isArray(jobs.jobs))
    throw new Error("workflow-jobs export requires jobs array");
  const selected = jobs.jobs.filter(
    (value) => isRecord(value) && value.name === definition.release_job,
  );
  if (selected.length !== 1) {
    throw new Error(
      `jobs export must contain exactly one ${definition.release_job} job`,
    );
  }
  const job = selected[0] as Record<string, unknown>;
  if (
    job.status !== "completed" ||
    typeof job.conclusion !== "string" ||
    typeof job.started_at !== "string" ||
    typeof job.completed_at !== "string"
  ) {
    throw new Error("configured release job is unstarted or incomplete");
  }
  if (
    job.run_id !== run.id ||
    job.head_sha !== run.head_sha ||
    job.run_attempt !== run.run_attempt
  ) {
    throw new Error(
      "configured release job run, revision, or attempt does not match workflow run",
    );
  }
  const started = timestamp(job.started_at, "job.started_at");
  const completed = timestamp(job.completed_at, "job.completed_at");
  const observedAt = String(run.updated_at ?? job.completed_at);
  const observed = timestamp(observedAt, "run.updated_at");
  if (completed < started || observed < completed) {
    throw new Error("workflow job/run timestamps are not ordered");
  }
  const deadline = started + definition.clock_deadline_seconds * 1000;
  const outcome = mapConclusion(job.conclusion);
  const producer = {
    ...definition.producer,
    environment: {
      ...definition.producer.environment,
      github_run_id: scalar(run.id),
      github_run_url: scalar(run.html_url),
    },
  };
  const raw = [
    rawEvidenceFor(repo, definition.workflow_evidence_path, "application/yaml"),
    rawEvidenceFor(repo, definition.run_evidence_path, "application/json"),
    rawEvidenceFor(repo, definition.jobs_evidence_path, "application/json"),
  ];
  const capabilityId = `${definition.record_prefix}-capability`;
  const exerciseId = `${definition.record_prefix}-exercise`;
  const capability: StandingCapabilityRecord = {
    schema_version: 1,
    record_type: "operational_evidence",
    record_id: capabilityId,
    observed_at: observedAt,
    record_shape: "standing_capability",
    control_kind: "release",
    subject: definition.subject,
    producer,
    scope: definition.scope,
    configuration: definition.configuration,
    capability: {
      control_id: definition.control_id,
      status: "available",
      surface: definition.workflow_path,
      authorized_roles: definition.authorized_roles,
      coverage: definition.coverage,
      limitations: definition.limitations,
      supported_transitions: [definition.supported_transition],
      clock_support: {
        supported: true,
        start_event: "release_job_started",
        completion_event: "release_job_completed",
        deadline_seconds: definition.clock_deadline_seconds,
      },
    },
    owner: definition.owner,
    gaps: definition.gaps,
    actions: definition.actions,
    raw_evidence: raw,
  };
  const exercise: OperationalExerciseRecord = {
    schema_version: 1,
    record_type: "operational_evidence",
    record_id: exerciseId,
    observed_at: observedAt,
    record_shape: "exercise",
    control_kind: "release",
    subject: definition.subject,
    producer,
    scope: definition.scope,
    configuration: definition.configuration,
    exercise: {
      control_id: definition.control_id,
      capability_record_id: capabilityId,
      mode: "actual",
      started_at: String(job.started_at),
      completed_at: String(job.completed_at),
      actor: String(requireRecord(run.actor, "run.actor").login ?? ""),
      trigger: String(run.event),
      outcome,
      state_before: { source_revision: String(run.head_sha) },
      state_after: {
        conclusion: String(job.conclusion),
        run_id: scalar(run.id),
        url: scalar(run.html_url),
      },
      observations: [
        `workflow ${definition.workflow_path}`,
        `job ${definition.release_job} concluded ${String(job.conclusion)}`,
      ],
      clock: {
        applicability: "operational_with_clock",
        started_at: String(job.started_at),
        deadline_at: new Date(deadline).toISOString(),
        completed_at: String(job.completed_at),
        status: completed <= deadline ? "met" : "missed",
      },
    },
    owner: definition.owner,
    gaps: definition.gaps,
    actions: definition.actions,
    raw_evidence: raw,
  };
  const path = writeOperationalPair(repo, capability, exercise);
  return { path, capability, exercise };
}

function validateDefinition(value: GitHubReleaseProducerDefinition): void {
  if (!isRecord(value))
    throw new Error("producer definition must be an object");
  for (const key of [
    "record_prefix",
    "workflow_path",
    "release_job",
    "accepted_event",
    "control_id",
    "workflow_evidence_path",
    "run_evidence_path",
    "jobs_evidence_path",
  ]) {
    if (
      typeof value[key as keyof typeof value] !== "string" ||
      !value[key as keyof typeof value]
    ) {
      throw new Error(`producer definition requires ${key}`);
    }
  }
  if (
    !Number.isInteger(value.clock_deadline_seconds) ||
    value.clock_deadline_seconds < 1
  ) {
    throw new Error(
      "producer definition requires a positive clock_deadline_seconds",
    );
  }
}

function readRetained(repo: string, path: string): string {
  rawEvidenceFor(
    repo,
    path,
    path.endsWith(".json") ? "application/json" : "application/yaml",
  );
  return readFileSync(join(repo, "spec", "evidence", path), "utf8");
}

function parseJson(raw: string, label: string): Record<string, unknown> {
  try {
    return requireRecord(JSON.parse(raw), label);
  } catch (error) {
    throw new Error(
      `${label} export is malformed: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

function requireRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) throw new Error(`${label} must be an object`);
  return value;
}

function timestamp(value: unknown, label: string): number {
  const parsed = typeof value === "string" ? Date.parse(value) : Number.NaN;
  if (!Number.isFinite(parsed))
    throw new Error(`${label} must be a valid date-time`);
  return parsed;
}

function mapConclusion(
  value: unknown,
): OperationalExerciseRecord["exercise"]["outcome"] {
  if (value === "success") return "succeeded";
  if (
    value === "failure" ||
    value === "timed_out" ||
    value === "action_required"
  )
    return "failed";
  if (value === "cancelled") return "aborted";
  return "partial";
}

function scalar(value: unknown): string | number | boolean | null {
  return typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
    ? value
    : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
