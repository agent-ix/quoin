import { execFile } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { promisify } from "node:util";

import { publishFileNoReplace } from "../src/measurement/atomic-file.js";

import {
  buildMeasurementReport,
  operationalDischarge,
  operationalEvidenceSchema,
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
const execFileAsync = promisify(execFile);
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

  // Trace: FR-059-AC-1 (TC-1223)
  test("TC-1223 requires the complete operational envelope", () => {
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
    for (const observedAt of ["2026-08-30", "2026-02-30T12:05:00Z"]) {
      const invalid = structuredClone(value);
      invalid.observed_at = observedAt;
      expect(() => validateOperationalRecord(invalid)).toThrow(/date-time/);
    }
    const authored = readFileSync(
      join(
        process.cwd(),
        "spec",
        "functional",
        "FR-059-operational-evidence-records.md",
      ),
      "utf8",
    ).match(/```json\n([\s\S]*?)\n```/)?.[1];
    expect(authored).toBeDefined();
    expect(operationalEvidenceSchema).toEqual(JSON.parse(authored as string));
  });

  // Trace: FR-059-AC-2 (TC-1224)
  test("TC-1224 admits exactly the declared control vocabulary", () => {
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

  // Trace: FR-059-AC-3, FR-059-AC-5 (TC-1225, TC-1227)
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

  // Trace: FR-059-AC-4, FR-059-AC-6, FR-059-AC-8 (TC-1226, TC-1228, TC-1230)
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
    (invalid.exercise.clock as { status: string }).status = "unknown";
    invalid.gaps = ["producer uncertainty must not hide deterministic time"];
    expect(() => validateOperationalRecord(invalid)).toThrow(/derived met/);
    invalid.exercise.clock.status = "met";
    invalid.exercise.clock.completed_at = "2026-08-30T12:11:00.000Z";
    expect(() => validateOperationalRecord(invalid)).toThrow(/derived missed/);
    invalid.exercise.clock.completed_at = "2026-08-30";
    expect(() => validateOperationalRecord(invalid)).toThrow(/RFC 3339/);
    invalid.exercise.clock = {
      applicability: "not_applicable",
      status: "not_applicable",
    };
    expect(() => validateOperationalRecord(invalid)).not.toThrow();
  });

  // Trace: FR-059-AC-7 (TC-1229)
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

  // Trace: FR-059-AC-9, FR-060-AC-1, FR-060-AC-2 (TC-1231..TC-1233)
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

    const badPath = exercise(root);
    badPath.raw_evidence[0].path = "raw/./control.json";
    expect(() => writeOperationalRecord(root, badPath)).toThrow(
      /raw_evidence_mismatch/,
    );
    expect(readOperationalRecords(root)).toHaveLength(1);

    const slashIdRoot = repo();
    const slashId = capability(slashIdRoot);
    slashId.record_id = `a${"/".repeat(127)}`;
    const slashIdPath = writeOperationalRecord(slashIdRoot, slashId);
    expect(basename(slashIdPath)).toMatch(/^[a-f0-9]{64}[.]json$/);
    expect(basename(slashIdPath).length).toBeLessThanOrEqual(255);
    expect(readOperationalRecords(slashIdRoot)).toEqual([slashId]);
  });

  // Trace: FR-060-AC-3, FR-060-AC-4, FR-060-AC-5, FR-060-AC-6 (TC-1234..TC-1237)
  test("definition, idempotency, collision, shape, and outcome behavior remain explicit", async () => {
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

    const pairRoot = repo();
    const pairCapability = capability(pairRoot);
    const pairExercise = exercise(pairRoot);
    const pairPath = writeOperationalPair(
      pairRoot,
      pairCapability,
      pairExercise,
    );
    expect(writeOperationalRecord(pairRoot, pairCapability)).toBe(pairPath);
    expect(writeOperationalRecord(pairRoot, pairExercise)).toBe(pairPath);
    expect(readOperationalRecords(pairRoot)).toEqual([
      pairCapability,
      pairExercise,
    ]);

    const raceRoot = repo();
    const target = join(raceRoot, "retained.json");
    const temporary = join(raceRoot, "candidate.tmp");
    writeFileSync(target, "first");
    writeFileSync(temporary, "second");
    expect(() =>
      publishFileNoReplace(
        temporary,
        target,
        "second",
        () => new Error("record_id_collision"),
      ),
    ).toThrow(/record_id_collision/);
    expect(readFileSync(target, "utf8")).toBe("first");

    const concurrentRoot = repo();
    const standalone = capability(concurrentRoot);
    const concurrentPairCapability = capability(concurrentRoot);
    concurrentPairCapability.owner = "pair-owner";
    const concurrentPairExercise = exercise(concurrentRoot);
    concurrentPairExercise.owner = "pair-owner";
    const standalonePath = join(concurrentRoot, "standalone.json");
    const pairCapabilityPath = join(concurrentRoot, "pair-capability.json");
    const pairExercisePath = join(concurrentRoot, "pair-exercise.json");
    writeFileSync(standalonePath, JSON.stringify(standalone));
    writeFileSync(pairCapabilityPath, JSON.stringify(concurrentPairCapability));
    writeFileSync(pairExercisePath, JSON.stringify(concurrentPairExercise));
    const lock = join(
      concurrentRoot,
      "spec",
      "evidence",
      ".operational-write.lock",
    );
    mkdirSync(lock, { recursive: true });
    const worker = join(
      process.cwd(),
      "tests",
      "fixtures",
      "operational-race-writer.ts",
    );
    const command = ["--loader", "ts-node/esm", worker, concurrentRoot];
    const writes = [
      execFileAsync(process.execPath, [
        ...command,
        "standalone",
        standalonePath,
        pairExercisePath,
      ]),
      execFileAsync(process.execPath, [
        ...command,
        "pair",
        pairCapabilityPath,
        pairExercisePath,
      ]),
    ];
    const readyBy = Date.now() + 10_000;
    while (
      (!existsSync(join(concurrentRoot, "standalone.ready")) ||
        !existsSync(join(concurrentRoot, "pair.ready"))) &&
      Date.now() < readyBy
    ) {
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
    expect(existsSync(join(concurrentRoot, "standalone.ready"))).toBe(true);
    expect(existsSync(join(concurrentRoot, "pair.ready"))).toBe(true);
    rmSync(lock, { recursive: true });
    const results = await Promise.allSettled(writes);
    expect(
      results.filter((result) => result.status === "fulfilled"),
    ).toHaveLength(1);
    expect(
      results.filter((result) => result.status === "rejected"),
    ).toHaveLength(1);
    const retained = readOperationalRecords(concurrentRoot);
    expect(new Set(retained.map((record) => record.record_id)).size).toBe(
      retained.length,
    );
    expect([1, 2]).toContain(retained.length);

    const staleRoot = repo();
    const staleLock = join(
      staleRoot,
      "spec",
      "evidence",
      ".operational-write.lock",
    );
    mkdirSync(staleLock, { recursive: true });
    const now = Date.now();
    const clock = vi
      .spyOn(Date, "now")
      .mockReturnValueOnce(now)
      .mockReturnValue(now + 10_001);
    try {
      expect(() =>
        writeOperationalRecord(staleRoot, capability(staleRoot)),
      ).toThrow(/intake_busy.*fail closed/);
    } finally {
      clock.mockRestore();
    }
    expect(readOperationalRecords(staleRoot)).toEqual([]);
  }, 20_000);

  // Trace: FR-060-AC-7 (TC-1238)
  test("clocked discharge requires full identity, mode, success, and met-clock match", () => {
    const root = repo();
    const value = exercise(root);
    const obligation = {
      control_kind: value.control_kind,
      subject: value.subject,
      scope: value.scope,
      accepted_modes: ["actual" as const],
      clock: {
        applicability: "operational_with_clock" as const,
        started_at: value.exercise.clock.started_at,
        deadline_at: value.exercise.clock.deadline_at,
      },
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
    const forged = structuredClone(value);
    forged.exercise.clock.completed_at = "2026-08-30T12:11:00.000Z";
    forged.exercise.clock.status = "met";
    expect(operationalDischarge(forged, obligation)).toMatchObject({
      discharged: false,
      reason: expect.stringMatching(
        /invalid operational exercise.*derived missed/,
      ),
    });
    expect(
      operationalDischarge(value, {
        ...obligation,
        clock: {
          ...obligation.clock,
          deadline_at: "2026-08-30T12:03:00.000Z",
        },
      }),
    ).toMatchObject({
      discharged: false,
      reason: "obligation clock condition mismatch",
    });
  });

  // Trace: FR-060-AC-8, FR-060-AC-9, FR-060-AC-10 (TC-1239..TC-1241)
  test("report keeps adverse states separate and has no aggregate trust score", () => {
    const root = repo();
    writeOperationalRecord(root, capability(root));
    const adverse = exercise(root);
    adverse.exercise.outcome = "failed";
    writeOperationalRecord(root, adverse);
    const unknown = capability(root);
    unknown.record_id = "unknown-capability";
    unknown.capability.control_id = "unknown-control";
    unknown.capability.status = "unknown";
    unknown.gaps = ["availability has not been observed"];
    unknown.actions = ["run a capability probe"];
    writeOperationalRecord(root, unknown);
    const open = exercise(root);
    open.record_id = "open-exercise";
    open.exercise.capability_record_id = undefined;
    open.exercise.control_id = "open-control";
    open.exercise.outcome = "partial";
    open.exercise.clock.completed_at = undefined;
    open.exercise.clock.status = "open";
    open.observed_at = "2026-08-30T12:05:00.000Z";
    open.gaps = ["exercise still open"];
    open.actions = ["wait for the deadline"];
    writeOperationalRecord(root, open);
    const report = buildMeasurementReport(root);
    const human = renderMeasurementReport(report);
    const json = renderMeasurementReportJson(report);
    const parsed = JSON.parse(json) as { operational: unknown };
    expect(human).toContain("## Operational evidence");
    expect(human).toContain("#### Claims");
    expect(human).toContain("#### Evidence");
    expect(human).toContain("#### Counterevidence");
    expect(human).toContain("#### Gaps");
    expect(human).toContain("#### Owner");
    expect(human).toContain("#### Actions");
    expect(human).toContain("exercise is failed");
    expect(human).toContain("availability has not been observed");
    expect(human).toContain("run a capability probe");
    expect(human).toContain("exercise still open");
    expect(human).toContain("wait for the deadline");
    expect(parsed.operational).toEqual(report.operational);
    expect(report.operational).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          record_id: "release-capability",
          claims: expect.arrayContaining([expect.stringMatching(/available/)]),
          evidence: expect.arrayContaining([expect.stringMatching(/surface/)]),
          owner: "release-owner",
        }),
        expect.objectContaining({
          record_id: "release-exercise",
          counterevidence: expect.arrayContaining([
            expect.stringMatching(/exercise is failed/),
          ]),
        }),
        expect.objectContaining({
          record_id: "open-exercise",
          gaps: expect.arrayContaining([
            "exercise still open",
            expect.stringMatching(/clock open/),
          ]),
          actions: ["wait for the deadline"],
        }),
      ]),
    );
    expect(renderMeasurementReport(buildMeasurementReport(root))).toBe(human);
    expect(json).not.toMatch(/trust.score|confidence.score|quality.score/i);
  });

  // Trace: FR-060-CON-1, FR-060-CON-2 (TC-1242, TC-1243)
  test("operational modules have no control path and coexist with legacy evidence", () => {
    const sources = [
      "src/measurement/operational.ts",
      "src/measurement/operational-report.ts",
      "src/measurement/github-release-operational.ts",
    ].map((path) => readFileSync(join(process.cwd(), path), "utf8"));
    expect(sources.join("\n")).not.toMatch(/node:child_process|\bfetch\s*\(/);
    const root = repo();
    const source = join(process.cwd(), "spec", "evidence");
    const measurement = "tier1-20260829194758755-8b2c0c3ba9c4.json";
    const measurementTarget = join(
      root,
      "spec",
      "evidence",
      "measurements",
      measurement,
    );
    mkdirSync(dirname(measurementTarget), { recursive: true });
    copyFileSync(join(source, "measurements", measurement), measurementTarget);
    const intervention = "quoin-270-cli-eval-sentinel-contract.json";
    const interventionTarget = join(
      root,
      "spec",
      "evidence",
      "interventions",
      intervention,
    );
    mkdirSync(dirname(interventionTarget), { recursive: true });
    copyFileSync(
      join(source, "interventions", intervention),
      interventionTarget,
    );
    writeOperationalRecord(root, capability(root));
    const report = buildMeasurementReport(root);
    expect(report.corpusGaps).not.toBeNull();
    expect(report.interventions).toHaveLength(1);
    expect(report.operational).toHaveLength(1);
    expect(renderMeasurementReport(report)).toContain(
      "## Intervention experiments",
    );
    expect(renderMeasurementReport(report)).toContain(
      "## Operational evidence",
    );
  });

  // Trace: FR-061-AC-2, FR-061-AC-3, FR-061-AC-4 (TC-1245..TC-1247)
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
    Object.assign(definition as unknown as Record<string, unknown>, {
      observed_at: "1900-01-01T00:00:00Z",
      conclusion: "failure",
      clock_status: "missed",
      raw_evidence: [{ digest: `sha256:${"f".repeat(64)}` }],
    });
    expect(result.exercise.observed_at).toBe("2026-08-30T12:05:00.000Z");
    expect(writeOperationalPair(root, result.capability, result.exercise)).toBe(
      result.path,
    );

    for (const [conclusion, outcome] of [
      ["failure", "failed"],
      ["timed_out", "failed"],
      ["action_required", "failed"],
      ["cancelled", "aborted"],
      ["skipped", "partial"],
    ] as const) {
      const adverseRoot = repo();
      writeGitHubArtifacts(adverseRoot, conclusion);
      const adverse = produceGitHubReleaseOperational(
        adverseRoot,
        githubDefinition(),
      );
      expect(adverse.exercise.exercise.outcome).toBe(outcome);
      expect(
        operationalDischarge(adverse.exercise, {
          control_kind: "release",
          subject: adverse.exercise.subject,
          scope: adverse.exercise.scope,
          accepted_modes: ["actual"],
          clock: {
            applicability: "operational_with_clock",
            started_at: "2026-08-30T12:00:00.000Z",
            deadline_at: "2026-08-30T12:10:00.000Z",
          },
        }).discharged,
      ).toBe(false);
    }

    const mismatchRoot = repo();
    writeGitHubArtifacts(mismatchRoot, "success", "wrong/path.yml");
    expect(() =>
      produceGitHubReleaseOperational(mismatchRoot, githubDefinition()),
    ).toThrow(/path, event, or immutable source revision mismatch/);
    expect(readOperationalRecords(mismatchRoot)).toEqual([]);

    const malformed = repo();
    writeGitHubArtifacts(malformed, "success");
    writeRaw(malformed, "raw/run.json", "{");
    expect(() =>
      produceGitHubReleaseOperational(malformed, githubDefinition()),
    ).toThrow(/workflow-run export is malformed/);
    expect(readOperationalRecords(malformed)).toEqual([]);

    for (const [path, bytes, message] of [
      ["raw/release.yml", "jobs: [", /Flow sequence/],
      ["raw/jobs.json", "{", /workflow-jobs export is malformed/],
      [
        "raw/release.yml",
        "name: Release\non:\n  push:\njobs:\n  publish:\n    name: Publish\n",
        /accepted event/,
      ],
      [
        "raw/release.yml",
        "name: Release\non:\n  workflow_dispatch:\njobs:\n  test:\n    name: Test\n",
        /configured release job/,
      ],
    ] as const) {
      const invalid = repo();
      writeGitHubArtifacts(invalid, "success");
      writeRaw(invalid, path, bytes);
      expect(() =>
        produceGitHubReleaseOperational(invalid, githubDefinition()),
      ).toThrow(message);
      expect(readOperationalRecords(invalid)).toEqual([]);
    }

    for (const changed of [{ event: "push" }, { head_sha: "b".repeat(40) }]) {
      const invalid = repo();
      writeGitHubArtifacts(invalid, "success");
      const run = JSON.parse(
        readFileSync(
          join(invalid, "spec", "evidence", "raw", "run.json"),
          "utf8",
        ),
      ) as Record<string, unknown>;
      writeRaw(invalid, "raw/run.json", JSON.stringify({ ...run, ...changed }));
      expect(() =>
        produceGitHubReleaseOperational(invalid, githubDefinition()),
      ).toThrow(/path, event, or immutable source revision mismatch/);
      expect(readOperationalRecords(invalid)).toEqual([]);
    }

    for (const jobs of [
      [],
      [githubJob("success"), githubJob("success")],
      [{ ...githubJob("success"), started_at: undefined }],
      [{ ...githubJob("success"), status: "in_progress" }],
      [{ ...githubJob("success"), completed_at: undefined }],
    ]) {
      const invalid = repo();
      writeGitHubArtifacts(invalid, "success");
      writeRaw(invalid, "raw/jobs.json", JSON.stringify({ jobs }));
      expect(() =>
        produceGitHubReleaseOperational(invalid, githubDefinition()),
      ).toThrow(/exactly one Publish job|unstarted or incomplete/);
      expect(readOperationalRecords(invalid)).toEqual([]);
    }

    for (const changed of [
      { run_id: 41 },
      { head_sha: "b".repeat(40) },
      { run_attempt: 2 },
    ]) {
      const invalid = repo();
      writeGitHubArtifacts(invalid, "success");
      writeRaw(
        invalid,
        "raw/jobs.json",
        JSON.stringify({ jobs: [{ ...githubJob("success"), ...changed }] }),
      );
      expect(() =>
        produceGitHubReleaseOperational(invalid, githubDefinition()),
      ).toThrow(/does not match workflow run/);
      expect(readOperationalRecords(invalid)).toEqual([]);
    }

    for (const identityCase of [
      { run: { id: undefined }, job: { run_id: undefined } },
      {
        run: { run_attempt: undefined },
        job: { run_attempt: undefined },
      },
      { run: { id: 0 }, job: { run_id: 0 } },
      { run: { run_attempt: 0 }, job: { run_attempt: 0 } },
    ]) {
      const invalid = repo();
      writeGitHubArtifacts(invalid, "success");
      const run = JSON.parse(
        readFileSync(
          join(invalid, "spec", "evidence", "raw", "run.json"),
          "utf8",
        ),
      ) as Record<string, unknown>;
      writeRaw(
        invalid,
        "raw/run.json",
        JSON.stringify({ ...run, ...identityCase.run }),
      );
      writeRaw(
        invalid,
        "raw/jobs.json",
        JSON.stringify({
          jobs: [{ ...githubJob("success"), ...identityCase.job }],
        }),
      );
      expect(() =>
        produceGitHubReleaseOperational(invalid, githubDefinition()),
      ).toThrow(/positive/);
      expect(readOperationalRecords(invalid)).toEqual([]);
    }

    const inheritedTrigger = repo();
    writeGitHubArtifacts(inheritedTrigger, "success");
    const inheritedRun = JSON.parse(
      readFileSync(
        join(inheritedTrigger, "spec", "evidence", "raw", "run.json"),
        "utf8",
      ),
    ) as Record<string, unknown>;
    writeRaw(
      inheritedTrigger,
      "raw/run.json",
      JSON.stringify({ ...inheritedRun, event: "toString" }),
    );
    expect(() =>
      produceGitHubReleaseOperational(inheritedTrigger, {
        ...githubDefinition(),
        accepted_event: "toString",
      }),
    ).toThrow(/does not declare accepted event/);
  });

  // Trace: FR-061-AC-1, FR-061-AC-5 (TC-1244, TC-1248)
  test("retained real release evidence persists one exact linked pair offline", () => {
    const root = repo();
    const source = join(process.cwd(), "spec", "evidence", "github-actions");
    const definition = JSON.parse(
      readFileSync(
        join(source, "quoin-271-release-v0.22.5-definition.json"),
        "utf8",
      ),
    ) as GitHubReleaseProducerDefinition;
    writeFileSync(
      join(root, "spec", "assurance", "MP-901.md"),
      PLAN.replace(DEFINITION, definition.producer.definition_version),
    );
    for (const name of [
      "quoin-271-release-v0.22.5-workflow.yml",
      "quoin-271-release-v0.22.5-run.json",
      "quoin-271-release-v0.22.5-jobs.json",
    ]) {
      const target = join(root, "spec", "evidence", "github-actions", name);
      mkdirSync(dirname(target), { recursive: true });
      copyFileSync(join(source, name), target);
    }

    const result = produceGitHubReleaseOperational(root, definition);
    expect(readOperationalRecords(root)).toEqual([
      result.capability,
      result.exercise,
    ]);
    expect(result.capability).toMatchObject({
      record_shape: "standing_capability",
      subject: definition.subject,
      capability: {
        status: "available",
        surface: ".github/workflows/release.yml",
      },
    });
    expect(result.exercise.exercise).toMatchObject({
      actor: "kreneskyp",
      trigger: "workflow_dispatch",
      outcome: "succeeded",
      started_at: "2026-08-29T23:11:00Z",
      completed_at: "2026-08-29T23:11:37Z",
      clock: { status: "met" },
      state_before: {
        source_revision: "a9808be18b61f8e4d44e3b74de27e90f17c5c76b",
      },
    });
    expect(result.exercise.raw_evidence).toEqual([
      expect.objectContaining({
        digest:
          "sha256:5a867277a071c2dd8fe1ab86e22b4b3e580fff0b269756c3911c4dde2ed1cc78",
        size_bytes: 6651,
      }),
      expect.objectContaining({
        digest:
          "sha256:a3a3eea43f6f17fc9851a50aa6853aa1d67a77ad91c6b4ddc1922aa03c4acaaf",
        size_bytes: 11898,
      }),
      expect.objectContaining({
        digest:
          "sha256:adb0e51b5212d9ff4f01238a26040f8ea893c75f6bc02cfbc4b7d98552182e92",
        size_bytes: 13166,
      }),
    ]);

    const invalidRoot = repo();
    const invalidExercise = exercise(invalidRoot);
    invalidExercise.exercise.control_id = "different-control";
    expect(() =>
      writeOperationalPair(
        invalidRoot,
        capability(invalidRoot),
        invalidExercise,
      ),
    ).toThrow(/invalid_record/);
    expect(readOperationalRecords(invalidRoot)).toEqual([]);
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
      run_attempt: 1,
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
      jobs: [githubJob(conclusion)],
    }),
  );
}

function githubJob(conclusion: string): Record<string, unknown> {
  return {
    run_id: 42,
    run_attempt: 1,
    head_sha: "a".repeat(40),
    name: "Publish",
    status: "completed",
    conclusion,
    started_at: "2026-08-30T12:00:00.000Z",
    completed_at: "2026-08-30T12:04:00.000Z",
  };
}
