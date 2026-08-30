import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import {
  buildMeasurementReport,
  operationalDischarge,
  produceGitHubReleaseOperational,
  rawEvidenceFor,
  readOperationalRecords,
  renderMeasurementReport,
  renderMeasurementReportJson,
  validateOperationalRecord,
  writeOperationalPair,
  writeOperationalRecord,
  type GitHubReleaseProducerDefinition,
  type OperationalControlKind,
  type OperationalExerciseRecord,
  type StandingCapabilityRecord,
} from "../src/measurement/index.js";

const DEFINITION = "operational-fixture-v1";
const PLAN = `---
id: MP-901
title: Operational fixture
type: MeasurementPlan
status: active
owner: test
stage: gate
metric: operational.fixture
definition_version: ${DEFINITION}
---

# Fixture
`;

describe("operational evidence", () => {
  const roots: string[] = [];

  afterEach(() => {
    for (const root of roots.splice(0))
      rmSync(root, { recursive: true, force: true });
  });

  function repo(): string {
    const root = mkdtempSync(join(tmpdir(), "quoin-operational-"));
    roots.push(root);
    mkdirSync(join(root, "spec", "assurance"), { recursive: true });
    writeFileSync(join(root, "spec", "assurance", "MP-901.md"), PLAN);
    writeRaw(root, "raw/control.json", '{"control":true}\n');
    return root;
  }

  function base(root: string) {
    return {
      schema_version: 1 as const,
      record_type: "operational_evidence" as const,
      observed_at: "2026-08-30T12:05:00.000Z",
      control_kind: "release" as const,
      subject: { id: "quoin", revision: "a".repeat(40) },
      producer: {
        tool_identity: "quoin",
        tool_version: "1.2.3",
        configuration_digest: `sha256:${"1".repeat(64)}`,
        source_revision: "b".repeat(40),
        environment: { runner: "fixture" },
        definition_version: DEFINITION,
      },
      scope: {
        service: "quoin",
        environment: "test",
        population: "fixture release",
      },
      configuration: { version_pins: [] },
      owner: "release-owner",
      gaps: [] as string[],
      actions: ["retain next release run"],
      raw_evidence: [
        rawEvidenceFor(root, "raw/control.json", "application/json"),
      ],
    };
  }

  function capability(root: string): StandingCapabilityRecord {
    return {
      ...base(root),
      record_id: "release-capability",
      record_shape: "standing_capability",
      capability: {
        control_id: "release-control",
        status: "available",
        surface: ".github/workflows/release.yml",
        authorized_roles: ["maintainer"],
        coverage: "tagged package release",
        limitations: ["manual dispatch"],
        supported_transitions: ["tagged-to-published"],
        clock_support: {
          supported: true,
          start_event: "publish_started",
          completion_event: "publish_completed",
          deadline_seconds: 600,
        },
      },
    };
  }

  function exercise(root: string): OperationalExerciseRecord {
    return {
      ...base(root),
      record_id: "release-exercise",
      record_shape: "exercise",
      exercise: {
        control_id: "release-control",
        capability_record_id: "release-capability",
        mode: "actual",
        started_at: "2026-08-30T12:00:00.000Z",
        completed_at: "2026-08-30T12:04:00.000Z",
        actor: "fixture-user",
        trigger: "workflow_dispatch",
        outcome: "succeeded",
        state_before: { published: false },
        state_after: { published: true },
        observations: ["publish job completed"],
        clock: {
          applicability: "operational_with_clock",
          started_at: "2026-08-30T12:00:00.000Z",
          deadline_at: "2026-08-30T12:10:00.000Z",
          completed_at: "2026-08-30T12:04:00.000Z",
          status: "met",
        },
      },
    };
  }

  // Trace: FR-048-AC-1 (TC-1147)
  test("TC-1147 requires the complete operational envelope", () => {
    const value = capability(repo());
    expect(() => validateOperationalRecord(value)).not.toThrow();
    for (const key of [
      "record_id",
      "observed_at",
      "record_shape",
      "control_kind",
      "subject",
      "producer",
      "scope",
      "configuration",
      "owner",
      "gaps",
      "actions",
      "raw_evidence",
    ] as const) {
      const invalid = structuredClone(value) as unknown as Record<
        string,
        unknown
      >;
      delete invalid[key];
      expect(() => validateOperationalRecord(invalid)).toThrow(
        /invalid_record/,
      );
    }
  });

  // Trace: FR-048-AC-2 (TC-1148)
  test("TC-1148 admits exactly the declared control vocabulary", () => {
    const kinds: OperationalControlKind[] = [
      "release",
      "feature_flag",
      "canary_deployment",
      "shadow_deployment",
      "rollback",
      "kill_switch",
      "human_override",
      "appeal",
      "abstention",
      "safe_fallback",
      "policy_pin",
      "prompt_pin",
      "model_pin",
      "tool_pin",
      "data_pin",
      "reporting",
    ];
    for (const kind of kinds) {
      const value = capability(repo());
      value.control_kind = kind;
      if (kind.endsWith("_pin")) {
        const pinKind = kind.replace("_pin", "") as
          "policy" | "prompt" | "model" | "tool" | "data";
        value.configuration.version_pins = [
          {
            kind: pinKind,
            identity: "fixture",
            revision: "v1",
            digest: `sha256:${"2".repeat(64)}`,
          },
        ];
      }
      expect(() => validateOperationalRecord(value)).not.toThrow();
    }
    const invalid = capability(repo()) as unknown as Record<string, unknown>;
    invalid.control_kind = "unknown-control";
    expect(() => validateOperationalRecord(invalid)).toThrow(/control_kind/);
  });

  // Trace: FR-048-AC-3, FR-048-AC-5 (TC-1149, TC-1151)
  test("capability shape requires complete clock support and excludes exercise", () => {
    const value = capability(repo());
    (value as unknown as Record<string, unknown>).exercise = {};
    expect(() => validateOperationalRecord(value)).toThrow(/only capability/);
    delete (value as unknown as Record<string, unknown>).exercise;
    value.capability.clock_support = { supported: false };
    expect(() => validateOperationalRecord(value)).not.toThrow();
    (
      value.capability.clock_support as unknown as Record<string, unknown>
    ).deadline_seconds = 10;
    expect(() => validateOperationalRecord(value)).toThrow(/unsupported clock/);
  });

  // Trace: FR-048-AC-4, FR-048-AC-6, FR-048-AC-8 (TC-1150, TC-1152, TC-1154)
  test("exercise outcomes round-trip only with ordered and derived clock state", () => {
    for (const outcome of [
      "succeeded",
      "failed",
      "partial",
      "aborted",
    ] as const) {
      const value = exercise(repo());
      value.exercise.outcome = outcome;
      expect(() => validateOperationalRecord(value)).not.toThrow();
    }
    const invalid = exercise(repo());
    invalid.exercise.clock.status = "missed";
    expect(() => validateOperationalRecord(invalid)).toThrow(/derived met/);
    invalid.exercise.clock = {
      applicability: "not_applicable",
      status: "not_applicable",
    };
    expect(() => validateOperationalRecord(invalid)).not.toThrow();
  });

  // Trace: FR-048-AC-7 (TC-1153)
  test("pin controls require one unique matching pin", () => {
    const value = capability(repo());
    value.control_kind = "model_pin";
    expect(() => validateOperationalRecord(value)).toThrow(/matching pin/);
    const pin = {
      kind: "model" as const,
      identity: "fixture-model",
      revision: "v1",
      digest: `sha256:${"3".repeat(64)}`,
    };
    value.configuration.version_pins = [pin];
    expect(() => validateOperationalRecord(value)).not.toThrow();
    value.configuration.version_pins.push(pin);
    expect(() => validateOperationalRecord(value)).toThrow(/duplicate/);
  });

  // Trace: FR-048-AC-9, FR-049-AC-1, FR-049-AC-2 (TC-1155..TC-1157)
  test("valid intake is atomic while invalid links and raw bytes write nothing", () => {
    const root = repo();
    const cap = capability(root);
    const capPath = writeOperationalRecord(root, cap);
    expect(readFileSync(capPath, "utf8")).toContain(
      '"record_shape": "standing_capability"',
    );
    const badLink = exercise(root);
    badLink.exercise.control_id = "different";
    expect(() => writeOperationalRecord(root, badLink)).toThrow(
      /invalid_record/,
    );
    expect(readOperationalRecords(root)).toHaveLength(1);
    const badRaw = exercise(root);
    badRaw.raw_evidence[0].digest = `sha256:${"f".repeat(64)}`;
    expect(() => writeOperationalRecord(root, badRaw)).toThrow(
      /raw_evidence_mismatch/,
    );
    expect(readOperationalRecords(root)).toHaveLength(1);
  });

  // Trace: FR-049-AC-3, FR-049-AC-4, FR-049-AC-5, FR-049-AC-6 (TC-1158..TC-1161)
  test("definition, idempotency, collision, shape, and outcome behavior remain explicit", () => {
    const root = repo();
    const cap = capability(root);
    const path = writeOperationalRecord(root, cap);
    const bytes = readFileSync(path, "utf8");
    expect(writeOperationalRecord(root, cap)).toBe(path);
    expect(readFileSync(path, "utf8")).toBe(bytes);
    const collision = structuredClone(cap);
    collision.owner = "other";
    expect(() => writeOperationalRecord(root, collision)).toThrow(
      /record_id_collision/,
    );
    for (const [index, outcome] of (
      ["succeeded", "failed", "partial", "aborted"] as const
    ).entries()) {
      const value = exercise(root);
      value.record_id = `exercise-${index}`;
      value.exercise.outcome = outcome;
      writeOperationalRecord(root, value);
    }
    expect(
      readOperationalRecords(root).map((item) => item.record_shape),
    ).toEqual([
      "exercise",
      "exercise",
      "exercise",
      "exercise",
      "standing_capability",
    ]);
    const mismatch = capability(repo());
    mismatch.producer.definition_version = "other-v1";
    expect(() =>
      writeOperationalRecord(roots.at(-1) as string, mismatch),
    ).toThrow(/definition_mismatch/);
  });

  // Trace: FR-049-AC-7 (TC-1162)
  test("clocked discharge requires full identity, mode, success, and met-clock match", () => {
    const root = repo();
    const value = exercise(root);
    const obligation = {
      control_kind: value.control_kind,
      subject: value.subject,
      scope: value.scope,
      accepted_modes: ["actual" as const],
    };
    expect(operationalDischarge(value, obligation).discharged).toBe(true);
    for (const outcome of ["failed", "partial", "aborted"] as const) {
      const adverse = structuredClone(value);
      adverse.exercise.outcome = outcome;
      expect(operationalDischarge(adverse, obligation).discharged).toBe(false);
    }
    expect(
      operationalDischarge(value, { ...obligation, accepted_modes: ["drill"] })
        .reason,
    ).toMatch(/mode mismatch/);
  });

  // Trace: FR-049-AC-8, FR-049-AC-9, FR-049-AC-10 (TC-1163..TC-1165)
  test("report keeps adverse states separate and has no aggregate trust score", () => {
    const root = repo();
    writeOperationalRecord(root, capability(root));
    const adverse = exercise(root);
    adverse.exercise.outcome = "failed";
    writeOperationalRecord(root, adverse);
    const report = buildMeasurementReport(root);
    const human = renderMeasurementReport(report);
    const json = renderMeasurementReportJson(report);
    expect(human).toContain("## Operational evidence");
    expect(human).toContain("#### Claims");
    expect(human).toContain("#### Counterevidence");
    expect(human).toContain("exercise is failed");
    expect(renderMeasurementReport(buildMeasurementReport(root))).toBe(human);
    expect(json).not.toMatch(/trust.score|confidence.score|quality.score/i);
  });

  // Trace: FR-049-CON-1, FR-049-CON-2 (TC-1166, TC-1167)
  test("operational modules have no control-execution path and preserve empty-store reporting", () => {
    const sources = [
      "src/measurement/operational.ts",
      "src/measurement/operational-report.ts",
      "src/measurement/github-release-operational.ts",
    ].map((path) => readFileSync(join(process.cwd(), path), "utf8"));
    expect(sources.join("\n")).not.toMatch(/node:child_process|\bfetch\s*\(/);
    expect(buildMeasurementReport(repo()).operational).toEqual([]);
  });

  // Trace: FR-051-AC-2, FR-051-AC-3, FR-051-AC-4 (TC-1174..TC-1176)
  test("GitHub producer derives a linked pair and refuses mismatched/adverse inputs", () => {
    const root = repo();
    writeGitHubArtifacts(root, "success");
    const definition = githubDefinition();
    const result = produceGitHubReleaseOperational(root, definition);
    expect(readOperationalRecords(root)).toHaveLength(2);
    expect(result.capability.capability.surface).toBe(definition.workflow_path);
    expect(result.exercise.exercise).toMatchObject({
      actor: "release-owner",
      trigger: "workflow_dispatch",
      outcome: "succeeded",
      clock: { status: "met" },
    });
    expect(writeOperationalPair(root, result.capability, result.exercise)).toBe(
      result.path,
    );

    const adverseRoot = repo();
    writeGitHubArtifacts(adverseRoot, "failure");
    const adverse = produceGitHubReleaseOperational(adverseRoot, definition);
    expect(adverse.exercise.exercise.outcome).toBe("failed");
    expect(
      operationalDischarge(adverse.exercise, {
        control_kind: "release",
        subject: adverse.exercise.subject,
        scope: adverse.exercise.scope,
        accepted_modes: ["actual"],
      }).discharged,
    ).toBe(false);

    const mismatchRoot = repo();
    writeGitHubArtifacts(mismatchRoot, "success", "wrong/path.yml");
    expect(() =>
      produceGitHubReleaseOperational(mismatchRoot, definition),
    ).toThrow(/path, event, or immutable source revision mismatch/);
    expect(readOperationalRecords(mismatchRoot)).toEqual([]);
  });

  function githubDefinition(): GitHubReleaseProducerDefinition {
    const root = repo();
    const common = base(root);
    return {
      record_prefix: "github-release-1",
      workflow_path: ".github/workflows/release.yml",
      release_job: "Publish",
      accepted_event: "workflow_dispatch",
      control_id: "release-control",
      subject: common.subject,
      producer: common.producer,
      scope: common.scope,
      configuration: common.configuration,
      supported_transition: "tagged-to-published",
      authorized_roles: ["maintainer"],
      coverage: "npm package release",
      limitations: ["manual dispatch"],
      owner: common.owner,
      gaps: [],
      actions: common.actions,
      clock_deadline_seconds: 600,
      workflow_evidence_path: "raw/release.yml",
      run_evidence_path: "raw/run.json",
      jobs_evidence_path: "raw/jobs.json",
    };
  }
});

function writeRaw(root: string, path: string, bytes: string): void {
  const target = join(root, "spec", "evidence", path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, bytes);
}

function writeGitHubArtifacts(
  root: string,
  conclusion: string,
  path = ".github/workflows/release.yml",
): void {
  writeRaw(
    root,
    "raw/release.yml",
    "name: Release\non:\n  workflow_dispatch:\njobs:\n  publish:\n    name: Publish\n    runs-on: ubuntu-latest\n    steps: []\n",
  );
  writeRaw(
    root,
    "raw/run.json",
    JSON.stringify({
      id: 42,
      path,
      event: "workflow_dispatch",
      head_sha: "a".repeat(40),
      status: "completed",
      conclusion,
      run_started_at: "2026-08-30T12:00:00.000Z",
      updated_at: "2026-08-30T12:05:00.000Z",
      html_url: "https://example.invalid/run/42",
      actor: { login: "release-owner" },
    }),
  );
  writeRaw(
    root,
    "raw/jobs.json",
    JSON.stringify({
      jobs: [
        {
          name: "Publish",
          status: "completed",
          conclusion,
          started_at: "2026-08-30T12:00:00.000Z",
          completed_at: "2026-08-30T12:04:00.000Z",
        },
      ],
    }),
  );
}
