import {
  copyFileSync,
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
  InterventionIntakeError,
  produceAgentEvalIntervention,
  rawEvidenceFor,
  readInterventionRecords,
  renderMeasurementReport,
  renderMeasurementReportJson,
  validateInterventionRecord,
  writeInterventionRecord,
  type AgentEvalInterventionDefinition,
  type InterventionExperimentRecord,
} from "../src/measurement/index.js";

const DEFINITION = "intervention-fixture-v1";
const PLAN = `---
id: MP-900
title: Intervention fixture
type: MeasurementPlan
status: active
owner: test
stage: branch-comparison
metric: intervention.fixture
definition_version: ${DEFINITION}
---

# Fixture
`;

describe("intervention-experiment evidence", () => {
  const roots: string[] = [];

  afterEach(() => {
    for (const root of roots.splice(0))
      rmSync(root, { recursive: true, force: true });
  });

  function repo(withPlan = true): string {
    const root = mkdtempSync(join(tmpdir(), "quoin-intervention-"));
    roots.push(root);
    if (withPlan) {
      mkdirSync(join(root, "spec", "assurance"), { recursive: true });
      writeFileSync(join(root, "spec", "assurance", "MP-900.md"), PLAN);
    }
    writeRaw(root, "raw/baseline.json", '{"baseline":true}\n');
    writeRaw(root, "raw/treatment.json", '{"treatment":true}\n');
    return root;
  }

  function record(root: string): InterventionExperimentRecord {
    return {
      schema_version: 1,
      record_type: "intervention_experiment",
      record_id: "experiment-001",
      observed_at: "2026-08-30T12:00:00.000Z",
      subject: { id: "quoin", revision: "a".repeat(40) },
      producer: {
        tool_identity: "quoin",
        tool_version: "1.2.3",
        configuration_digest: `sha256:${"1".repeat(64)}`,
        source_revision: "b".repeat(40),
        environment: { node: "22.15.0" },
        definition_version: DEFINITION,
      },
      design: {
        kind: "repeated",
        repetitions: 2,
        assignment: { method: "deterministic" },
        sampling_conditions: ["same public fixture"],
      },
      baseline: {
        id: "baseline",
        population: "fixture scenarios",
        sample_size: 2,
        configuration: { variant: "before" },
      },
      treatments: [
        {
          id: "treatment",
          population: "fixture scenarios",
          sample_size: 2,
          configuration: { variant: "after" },
        },
      ],
      changed_variables: [
        {
          name: "prompt",
          treatment_id: "treatment",
          baseline_value: "before",
          treatment_value: "after",
        },
      ],
      held_constant: [{ name: "model", value: "fixture" }],
      measured_effects: [
        {
          treatment_id: "treatment",
          metric: "pass-rate",
          baseline_value: 0.5,
          treatment_value: 1,
          effect: 0.5,
          unit: "fraction",
        },
      ],
      interactions: [{ description: "none known", disposition: "controlled" }],
      confounders: [
        { description: "fixture only", disposition: "not_applicable" },
      ],
      status: "completed",
      conclusion: {
        kind: "cause_not_established",
        statement:
          "An observed difference exists; causality is not established.",
        attribution_confidence: "none",
      },
      gaps: ["fictional fixture"],
      owner: "test-owner",
      actions: ["capture a real repeated run"],
      raw_evidence: [
        rawEvidenceFor(root, "raw/baseline.json", "application/json"),
        rawEvidenceFor(root, "raw/treatment.json", "application/json"),
      ],
    };
  }

  // Trace: FR-056-AC-1, FR-056-AC-2 (TC-1195, TC-1196)
  test("requires the envelope and immutable producer tuple", () => {
    const root = repo();
    const valid = record(root);
    expect(() => validateInterventionRecord(valid)).not.toThrow();
    for (const key of [
      "record_id",
      "observed_at",
      "subject",
      "producer",
    ] as const) {
      const invalid = structuredClone(valid) as unknown as Record<
        string,
        unknown
      >;
      delete invalid[key];
      expect(() => validateInterventionRecord(invalid)).toThrow(
        InterventionIntakeError,
      );
    }
    for (const key of [
      "tool_identity",
      "tool_version",
      "configuration_digest",
      "source_revision",
      "environment",
      "definition_version",
    ] as const) {
      const invalid = structuredClone(valid);
      delete (invalid.producer as unknown as Record<string, unknown>)[key];
      expect(() => validateInterventionRecord(invalid)).toThrow(
        /invalid_record/,
      );
    }
    const mutable = structuredClone(valid);
    mutable.producer.tool_version = "latest";
    expect(() => validateInterventionRecord(mutable)).toThrow(/tool_version/);
  });

  // Trace: FR-056-AC-3 (TC-1197)
  test("TC-1197 validates every design and assignment boundary", () => {
    for (const [kind, method] of [
      ["repeated", "deterministic"],
      ["randomized", "randomized"],
      ["factorial", "blocked_randomized"],
    ] as const) {
      const value = record(repo());
      value.design = {
        kind,
        repetitions: 1,
        assignment: {
          method,
          ...(method.includes("randomized") ? { seed: "seed-1" } : {}),
        },
        sampling_conditions: ["fixed"],
      };
      expect(() => validateInterventionRecord(value)).not.toThrow();
      value.design.repetitions = 0;
      expect(() => validateInterventionRecord(value)).toThrow(/repetitions/);
    }
  });

  // Trace: FR-056-AC-4, FR-056-AC-5 (TC-1198, TC-1199)
  test("requires unique linked arms, variables, and effects without null coercion", () => {
    const value = record(repo());
    value.measured_effects[0].effect = null;
    expect(() => validateInterventionRecord(value)).not.toThrow();
    value.changed_variables.push({ ...value.changed_variables[0] });
    value.measured_effects.push({ ...value.measured_effects[0] });
    expect(() => validateInterventionRecord(value)).toThrow(/duplicate/);
    value.changed_variables = [
      { ...value.changed_variables[0], treatment_id: "missing" },
    ];
    expect(() => validateInterventionRecord(value)).toThrow(/does not resolve/);
  });

  // Trace: FR-056-AC-6, FR-056-AC-7 (TC-1200, TC-1201)
  test("preserves qualifier collections and terminal causal safety", () => {
    const value = record(repo());
    value.interactions = [
      { description: "interaction", disposition: "uncontrolled" },
    ];
    value.confounders = [{ description: "confounder", disposition: "unknown" }];
    expect(() => validateInterventionRecord(value)).not.toThrow();
    for (const status of ["failed", "inconclusive"] as const) {
      const invalid = structuredClone(value);
      invalid.status = status;
      invalid.conclusion.kind = "no_effect_observed";
      expect(() => validateInterventionRecord(invalid)).toThrow(
        /cause_not_established/,
      );
    }
  });

  // Trace: FR-056-AC-8 (TC-1202)
  test("causal conclusions require samples, effects, confidence, and controlled qualifiers", () => {
    const value = record(repo());
    value.conclusion = {
      kind: "causal_effect_established",
      statement: "fixture causal claim",
      attribution_confidence: "moderate",
    };
    expect(() => validateInterventionRecord(value)).not.toThrow();
    value.interactions[0].disposition = "unknown";
    value.measured_effects[0].effect = null;
    value.conclusion.attribution_confidence = "none";
    expect(() => validateInterventionRecord(value)).toThrow(/causal/);
  });

  // Trace: FR-056-AC-9, FR-057-AC-2, FR-057-AC-11 (TC-1203, TC-1205, TC-1216)
  test("refuses undeclared fields, unsafe paths, and raw-byte mismatch without writing", () => {
    const root = repo();
    const extra = record(root) as unknown as Record<string, unknown>;
    extra.unowned = true;
    expect(() => writeInterventionRecord(root, extra)).toThrow(
      /invalid_record/,
    );
    expect(readInterventionRecords(root)).toEqual([]);
    const unsafe = record(root);
    unsafe.raw_evidence[0].path = "../outside.json";
    expect(() => writeInterventionRecord(root, unsafe)).toThrow(
      /raw_evidence_mismatch/,
    );
    const mismatch = record(root);
    mismatch.raw_evidence[0].size_bytes += 1;
    mismatch.raw_evidence[1].digest = `sha256:${"f".repeat(64)}`;
    expect(() => writeInterventionRecord(root, mismatch)).toThrow(
      /raw_evidence_mismatch.*size_bytes.*digest/,
    );
    expect(readInterventionRecords(root)).toEqual([]);
  });

  // Trace: FR-057-AC-1, FR-057-AC-4, FR-057-AC-5 (TC-1204, TC-1207, TC-1208)
  test("writes atomically, repeats byte-identically, and refuses collisions", () => {
    const root = repo();
    const value = record(root);
    const path = writeInterventionRecord(root, value);
    const before = readFileSync(path, "utf8");
    expect(writeInterventionRecord(root, value)).toBe(path);
    expect(readFileSync(path, "utf8")).toBe(before);
    const collision = structuredClone(value);
    collision.owner = "different-owner";
    expect(() => writeInterventionRecord(root, collision)).toThrow(
      /record_id_collision/,
    );
    expect(readFileSync(path, "utf8")).toBe(before);
  });

  // Trace: FR-057-AC-3 (TC-1206)
  test("distinguishes absent and mismatched governing definitions", () => {
    const absent = repo(false);
    expect(() => writeInterventionRecord(absent, record(absent))).toThrow(
      /governing_plan_absent.*requested definition/,
    );
    const mismatch = repo();
    const value = record(mismatch);
    value.producer.definition_version = "different-v1";
    expect(() => writeInterventionRecord(mismatch, value)).toThrow(
      /definition_mismatch.*expected.*observed/,
    );
  });

  // Trace: FR-057-AC-6 (TC-1209)
  test("keeps every terminal result independently queryable", () => {
    const root = repo();
    for (const [index, status] of (
      ["completed", "failed", "inconclusive"] as const
    ).entries()) {
      const value = record(root);
      value.record_id = `experiment-${index}`;
      value.status = status;
      writeInterventionRecord(root, value);
    }
    expect(readInterventionRecords(root).map((item) => item.status)).toEqual([
      "completed",
      "failed",
      "inconclusive",
    ]);
  });

  // Trace: FR-057-AC-7, FR-057-AC-8, FR-057-AC-9, FR-057-AC-10, FR-057-CON-2 (TC-1210..TC-1213)
  test("renders one deterministic claim-centered object with no aggregate score", () => {
    const root = repo();
    const value = record(root);
    value.interactions[0] = {
      description: "uncontrolled ordering",
      disposition: "uncontrolled",
    };
    writeInterventionRecord(root, value);
    const report = buildMeasurementReport(root);
    const human = renderMeasurementReport(report);
    const json = renderMeasurementReportJson(report);
    expect(human).toContain("#### Claims");
    expect(human).toContain("#### Evidence");
    expect(human).toContain("#### Counterevidence");
    expect(human).toContain("uncontrolled ordering");
    expect(human).toContain("#### Gaps");
    expect(human).toContain("#### Owner");
    expect(human).toContain("#### Actions");
    expect(renderMeasurementReport(buildMeasurementReport(root))).toBe(human);
    expect(json).not.toMatch(/trust.score|quality.score|overall.score/i);
    expect(JSON.parse(json).interventions[0]).toMatchObject({
      claims: value.conclusion.statement ? [value.conclusion.statement] : [],
      owner: value.owner,
      actions: value.actions,
    });
  });

  // Trace: FR-057-CON-1, FR-057-CON-3 (TC-1214, TC-1215)
  test("intervention modules have no process/network import and an empty store preserves measurement reporting", () => {
    const sources = [
      "src/measurement/intervention.ts",
      "src/measurement/agent-eval-intervention.ts",
      "src/measurement/intervention-report.ts",
    ].map((path) => readFileSync(join(process.cwd(), path), "utf8"));
    expect(sources.join("\n")).not.toMatch(/node:child_process|\bfetch\s*\(/);
    const root = repo();
    expect(buildMeasurementReport(root).interventions).toEqual([]);
  });

  // Trace: FR-058-AC-2, FR-058-AC-3, FR-058-AC-4 (TC-1218..TC-1220)
  test("agent-eval producer derives effects/raw metadata and refuses mismatches", () => {
    const root = repo();
    const baseline = reportJson([
      { id: "TC-EV-1", passed: 1, total: 2 },
      { id: "TC-EV-2", passed: 0, total: 2 },
    ]);
    const treatment = reportJson([
      { id: "TC-EV-1", passed: 2, total: 2 },
      { id: "TC-EV-2", passed: 1, total: 2 },
    ]);
    writeRaw(root, "raw/eval-before.json", baseline);
    writeRaw(root, "raw/eval-after.json", treatment);
    const definition = producerDefinition();
    const { record: produced } = produceAgentEvalIntervention(root, definition);
    expect(produced.measured_effects.map((item) => item.effect)).toEqual([
      0.5, 0.5,
    ]);
    expect(produced.conclusion).toMatchObject({
      kind: "cause_not_established",
      attribution_confidence: "none",
    });
    expect(produced.raw_evidence).toEqual([
      rawEvidenceFor(
        root,
        definition.baseline_evidence_path,
        "application/json",
      ),
      rawEvidenceFor(
        root,
        definition.treatment_evidence_path,
        "application/json",
      ),
    ]);
    const callerSubstitutes = {
      ...definition,
      measured_effects: [{ effect: 999 }],
      raw_evidence: [
        {
          path: "caller-selected.json",
          media_type: "text/plain",
          size_bytes: 1,
          digest: `sha256:${"f".repeat(64)}`,
        },
      ],
    } as unknown as AgentEvalInterventionDefinition;
    expect(
      produceAgentEvalIntervention(root, callerSubstitutes).record.raw_evidence,
    ).toEqual(produced.raw_evidence);

    const mismatchRoot = repo();
    writeRaw(mismatchRoot, "raw/eval-before.json", baseline);
    writeRaw(
      mismatchRoot,
      "raw/eval-after.json",
      reportJson([{ id: "different", passed: 1, total: 2 }]),
    );
    expect(() =>
      produceAgentEvalIntervention(mismatchRoot, definition),
    ).toThrow(/scenario mismatch/);
    expect(readInterventionRecords(mismatchRoot)).toEqual([]);

    const invalidReports = [
      [
        "empty",
        JSON.stringify({
          ok: true,
          suite: "empty",
          repeats: 1,
          results: [],
        }),
      ],
      ["malformed", "{invalid\n"],
      [
        "unversioned",
        JSON.stringify({
          ok: true,
          suite: "unversioned",
          results: [{ id: "TC-EV-1", ok: true, passRate: "1/1" }],
        }),
      ],
      [
        "duplicate",
        JSON.stringify({
          ok: false,
          suite: "duplicate",
          repeats: 2,
          results: [
            { id: "TC-EV-1", ok: false, passRate: "1/2" },
            { id: "TC-EV-1", ok: false, passRate: "1/2" },
          ],
        }),
      ],
    ] as const;
    for (const [name, invalid] of invalidReports) {
      const invalidRoot = repo();
      writeRaw(invalidRoot, "raw/eval-before.json", invalid);
      writeRaw(invalidRoot, "raw/eval-after.json", treatment);
      expect(() =>
        produceAgentEvalIntervention(invalidRoot, producerDefinition()),
      ).toThrow();
      expect(readInterventionRecords(invalidRoot), name).toEqual([]);
    }

    const inadequateRoot = repo();
    const single = reportJson([{ id: "TC-EV-1", passed: 1, total: 1 }]);
    writeRaw(inadequateRoot, "raw/eval-before.json", single);
    writeRaw(inadequateRoot, "raw/eval-after.json", single);
    const inadequate = producerDefinition();
    inadequate.design.repetitions = 1;
    inadequate.interactions = [
      { description: "uncontrolled fixture", disposition: "uncontrolled" },
    ];
    inadequate.confounders = [
      { description: "unknown fixture", disposition: "unknown" },
    ];
    const inadequateRecord = produceAgentEvalIntervention(
      inadequateRoot,
      inadequate,
    ).record;
    expect(inadequateRecord.conclusion).toMatchObject({
      kind: "cause_not_established",
      attribution_confidence: "none",
    });
    expect(inadequateRecord.gaps).toEqual(
      expect.arrayContaining([
        "agent-eval reports do not contain repeated samples for every scenario",
        "one or more declared interactions or confounders are uncontrolled or unknown",
        "no justified attribution method was supplied",
      ]),
    );
  });

  // Trace: FR-058-AC-1, FR-058-AC-5 (TC-1217, TC-1221)
  test("real retained runner reports persist an honest zero-effect intervention", () => {
    const root = repo();
    const source = join(process.cwd(), "spec", "evidence", "agent-evals");
    for (const name of [
      "quoin-270-sentinel-baseline.json",
      "quoin-270-sentinel-treatment.json",
    ]) {
      const target = join(root, "spec", "evidence", "agent-evals", name);
      mkdirSync(dirname(target), { recursive: true });
      copyFileSync(join(source, name), target);
    }
    writeFileSync(
      join(root, "spec", "assurance", "MP-901.md"),
      PLAN.replace(DEFINITION, "cli-agent-evals.sentinel-contract-v1"),
    );
    const definition = JSON.parse(
      readFileSync(join(source, "quoin-270-sentinel-definition.json"), "utf8"),
    ) as AgentEvalInterventionDefinition;

    const { record: produced } = produceAgentEvalIntervention(root, definition);

    expect(produced.baseline.sample_size).toBe(2);
    expect(produced.treatments[0].sample_size).toBe(2);
    expect(produced.measured_effects).toEqual([
      expect.objectContaining({
        baseline_value: 0.5,
        treatment_value: 0.5,
        effect: 0,
      }),
    ]);
    expect(produced.conclusion).toEqual({
      kind: "cause_not_established",
      statement:
        "The retained agent-evaluation runs show no observed pass-rate difference; this adapter does not establish causality.",
      attribution_confidence: "none",
    });
    expect(produced.raw_evidence.map((item) => item.digest)).toEqual([
      "sha256:741ab150c107d1f5551a9be2092081cc0439f85e804d23f81e3923dfab8fe076",
      "sha256:baade4ab1c2e8f3447b4c585c95634884cb2ed1c69a9cf8819d0b495c7f77963",
    ]);
    expect(renderMeasurementReport(buildMeasurementReport(root))).toContain(
      "no observed pass-rate difference",
    );
  });

  function producerDefinition(): AgentEvalInterventionDefinition {
    const base = record(repo());
    // The throwaway root above is cleaned by afterEach; the definition itself is
    // path-relative and carries no bytes from it.
    return {
      report_schema_version: "cli-agent-evals-report-v1",
      cli_agent_evals_version: "0.4.0",
      record_id: "agent-eval-experiment",
      observed_at: base.observed_at,
      subject: base.subject,
      producer: base.producer,
      design: base.design,
      baseline: {
        id: base.baseline.id,
        population: base.baseline.population,
        configuration: base.baseline.configuration,
      },
      treatment: {
        id: base.treatments[0].id,
        population: base.treatments[0].population,
        configuration: base.treatments[0].configuration,
      },
      changed_variables: base.changed_variables,
      held_constant: base.held_constant,
      interactions: base.interactions,
      confounders: base.confounders,
      owner: base.owner,
      gaps: [],
      actions: base.actions,
      baseline_evidence_path: "raw/eval-before.json",
      treatment_evidence_path: "raw/eval-after.json",
    };
  }
});

function writeRaw(root: string, path: string, bytes: string): void {
  const target = join(root, "spec", "evidence", path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, bytes);
}

function reportJson(
  scenarios: Array<{ id: string; passed: number; total: number }>,
): string {
  const total = scenarios[0]?.total ?? 0;
  return JSON.stringify({
    ok: scenarios.every((item) => item.passed === item.total),
    suite: "constructed-fixture",
    repeats: total,
    results: scenarios.map((item) => ({
      id: item.id,
      ok: item.passed === item.total,
      passRate: `${item.passed}/${item.total}`,
    })),
  });
}
