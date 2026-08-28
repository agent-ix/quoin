#!/usr/bin/env node
// The committed battletest runner (agent-ix/quoin#203).
//
// A battletest was a manual, human-driven session whose findings were
// transcribed into prose and ad-hoc frozen into unit tests. This makes pass 3
// a command.
//
// It runs the tool suite against the corpora the benchmark declares, scores
// the run against the tier-1 labels and the tier-2 adjudicated answer key, and
// diffs the result against the checked-in baseline.
//
//   node scripts/battletest.mjs               # score and diff
//   node scripts/battletest.mjs --update      # deliberate re-baseline
//
// What it does NOT do is replace the human pass. Every conclusion-changing
// finding of pass 2 came from somebody reading code, and a runner that claimed
// otherwise would be the overclaim this whole programme exists to end. What it
// replaces is the RE-RUN: checking whether what was found before is still
// found, cheaply enough to do every time.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  compareTier2Baseline,
  createTier2Baseline,
  retainTier2Sources,
} from "./lib/tier2-baseline.mjs";
import { validateFindingEnvelope } from "../evals/lib/finding-envelope.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const ANSWER_KEY = join(ROOT, "bench", "answer-key.json");
const BASELINE = join(ROOT, "bench", "battletest-baseline.json");
const QUOIN = join(ROOT, "bin", "quoin.js");
const COVERAGE_SOURCE = "quire.coverage";

/** `quire coverage --json` over one scope, or a stated reason it could not run. */
function coverage(quire, scope, module) {
  const args = ["coverage", "--scope", scope, "--json"];
  if (module) args.push("--module", module);
  const command = canonicalCommand("QUIRE", args, { scope, module });
  try {
    return {
      ok: true,
      state: "evaluated",
      command,
      payload: JSON.parse(
        execFileSync(quire, args, {
          encoding: "utf8",
          maxBuffer: 256 * 1024 * 1024,
          stdio: ["ignore", "pipe", "pipe"],
        }),
      ),
    };
  } catch (error) {
    const stderr = String(error.stderr ?? error.message)
      .trim()
      .split("\n")
      .pop();
    return {
      ok: false,
      state: "failed",
      command,
      reason: stderr?.slice(0, 200) ?? "unknown",
    };
  }
}

/**
 * Which answer-key findings this run would surface.
 *
 * A finding counts as detected when the payload carries the signal the key
 * says it should — `expect_reason` in diagnostics, `expect_suspicion` in
 * suspicions, `expect_metric` at its expected value. A key entry that declares
 * no signal is NOT EVALUATED BY THIS RUNNER and is reported as such, never
 * counted as a miss. That state says nothing about whether another production
 * command can detect the family (agent-ix/quoin#203).
 */
export function scoreAgainstSources(sources, key) {
  return scoreAgainstRetainedSources(retainTier2Sources(sources), key);
}

/** Score only the normalized cross-producer view retained in the baseline. */
export function scoreAgainstRetainedSources(sources, key) {
  const detected = [];
  const missed = [];
  const notMechanized = [];
  const notEvaluated = [];
  const unavailable = [];
  const invalidAnswerKey = [];

  for (const finding of key.findings) {
    if (finding.answer_key_state === "invalid") {
      invalidAnswerKey.push({
        id: finding.id,
        reason:
          finding.invalid_reason ??
          "the answer-key entry is explicitly invalid",
      });
      continue;
    }
    if (
      finding.expect_metric &&
      (finding.expect_value === undefined ||
        finding.expect_value === null ||
        Number.isNaN(Number(finding.expect_value)))
    ) {
      throw new Error(
        `answer key ${finding.id}: declares expect_metric ` +
          `"${finding.expect_metric}" with no usable expect_value ` +
          `(got ${JSON.stringify(finding.expect_value)}). A malformed entry ` +
          `must fail the run, never score as a miss.`,
      );
    }

    const sourceName = finding.source ?? COVERAGE_SOURCE;
    const source = sources[sourceName];
    if (source?.state !== "evaluated") {
      notMechanized.push(finding.id);
      const item = {
        id: finding.id,
        source: sourceName,
        reason: source?.reason ?? "the answer key names no runnable source",
      };
      (source?.state === "unavailable" ? unavailable : notEvaluated).push(item);
      continue;
    }

    const result = scoreFinding(source, finding);
    if (result === null) {
      notMechanized.push(finding.id);
      notEvaluated.push({
        id: finding.id,
        source: sourceName,
        reason: "the answer key declares no signal to score",
      });
    } else {
      (result ? detected : missed).push(finding.id);
    }
  }
  const denominator = detected.length + missed.length;
  return {
    detected: detected.sort(),
    missed: missed.sort(),
    notMechanized: notMechanized.sort(),
    notEvaluated: notEvaluated.sort((a, b) => compare(a.id, b.id)),
    unavailable: unavailable.sort((a, b) => compare(a.id, b.id)),
    invalidAnswerKey: invalidAnswerKey.sort((a, b) => compare(a.id, b.id)),
    // `null`, not 0, when nothing is mechanized — 0/0 is not 0% recall.
    recall:
      denominator === 0
        ? null
        : Number((detected.length / denominator).toFixed(3)),
  };
}

/** Score findings against their exact retained defect and healthy-control cohorts. */
export function scoreAgainstCohorts(cohorts, key) {
  const retained = Object.fromEntries(
    Object.entries(cohorts).map(([id, cohort]) => [
      id,
      { ...cohort, sources: retainTier2Sources(cohort.sources ?? {}) },
    ]),
  );
  return scoreAgainstRetainedCohorts(retained, key);
}

export function scoreAgainstRetainedCohorts(cohorts, key) {
  const detected = [];
  const missed = [];
  const unavailable = [];
  const notEvaluated = [];
  const invalidAnswerKey = [];
  const controlFailures = [];

  for (const finding of key.findings) {
    if (finding.answer_key_state === "invalid") {
      invalidAnswerKey.push({
        id: finding.id,
        reason:
          finding.invalid_reason ??
          "the answer-key entry is explicitly invalid",
      });
      continue;
    }
    validateFindingDefinition(finding);
    const cohort = cohorts[finding.cohort];
    const source = cohort?.sources?.[finding.source ?? COVERAGE_SOURCE];
    if (source?.state !== "evaluated") {
      const item = {
        id: finding.id,
        cohort: finding.cohort,
        source: finding.source ?? COVERAGE_SOURCE,
        reason:
          source?.reason ?? "the pinned defect cohort/source is unavailable",
      };
      (source?.state === "unavailable" ? unavailable : notEvaluated).push(item);
      continue;
    }
    const hit = scoreFinding(source, finding);
    if (hit === null) {
      throw new Error(
        `answer key ${finding.id}: valid entries must declare an exact signal`,
      );
    }
    if (!hit) {
      missed.push(finding.id);
      continue;
    }

    const control = finding.healthy_control;
    if (control?.state === "pinned") {
      const controlSource =
        cohorts[control.cohort]?.sources?.[
          control.source ?? finding.source ?? COVERAGE_SOURCE
        ];
      if (controlSource?.state !== "evaluated") {
        const item = {
          id: finding.id,
          cohort: control.cohort,
          source: control.source ?? finding.source ?? COVERAGE_SOURCE,
          reason:
            controlSource?.reason ??
            "the pinned healthy-control source is unavailable",
        };
        (controlSource?.state === "unavailable"
          ? unavailable
          : notEvaluated
        ).push(item);
        continue;
      }
      if (scoreFinding(controlSource, finding)) {
        missed.push(finding.id);
        controlFailures.push({
          id: finding.id,
          cohort: control.cohort,
          reason: "the finding also fires on its pinned healthy control",
        });
        continue;
      }
    }
    detected.push(finding.id);
  }

  const denominator = detected.length + missed.length;
  const sortedUnavailable = unavailable.sort((a, b) => compare(a.id, b.id));
  const sortedNotEvaluated = notEvaluated.sort((a, b) => compare(a.id, b.id));
  return {
    detected: detected.sort(),
    missed: missed.sort(),
    unavailable: sortedUnavailable,
    invalidAnswerKey: invalidAnswerKey.sort((a, b) => compare(a.id, b.id)),
    controlFailures: controlFailures.sort((a, b) => compare(a.id, b.id)),
    // Compatibility names remain while v2 exposes the distinct states directly.
    notMechanized: [...sortedUnavailable, ...sortedNotEvaluated]
      .map((item) => item.id)
      .sort(),
    notEvaluated: sortedNotEvaluated,
    recall:
      denominator === 0
        ? null
        : Number((detected.length / denominator).toFixed(3)),
  };
}

function scoreFinding(source, finding) {
  const normalizedFindings = source.normalized?.findings ?? [];
  for (const record of normalizedFindings) validateFindingEnvelope(record);
  const metrics = new Map(
    (source.normalized?.metrics ?? []).map((metric) => [metric.name, metric]),
  );
  if (finding.expect_metric) {
    const metric = metrics.get(finding.expect_metric);
    return Boolean(
      metric && Number(metric.value) === Number(finding.expect_value),
    );
  }
  const expected =
    finding.expect_reason ?? finding.expect_suspicion ?? finding.expect_finding;
  if (!expected) return null;
  const channel = finding.expect_reason
    ? "coverage.diagnostics"
    : finding.expect_suspicion
      ? "coverage.suspicions"
      : null;
  return normalizedFindings.some(
    (record) =>
      (channel === null || record.source.channel === channel) &&
      (record.identity?.family ?? record.kind) === expected &&
      matchesExpectedLocus(record, finding.expected_locus),
  );
}

function matchesExpectedLocus(record, expected) {
  if (!expected) return true;
  if (record.locus?.state !== "available") return false;
  return Object.entries(expected).every(
    ([field, value]) => record.locus.value?.[field] === value,
  );
}

function validateFindingDefinition(finding) {
  if (!finding.cohort) {
    throw new Error(`answer key ${finding.id}: valid entry has no cohort pin`);
  }
  if (!finding.source) {
    throw new Error(`answer key ${finding.id}: valid entry has no source`);
  }
  if (!finding.declaration?.state) {
    throw new Error(
      `answer key ${finding.id}: valid entry has no declaration state`,
    );
  }
  if (!finding.evidence_sidecar?.state) {
    throw new Error(
      `answer key ${finding.id}: valid entry has no evidence-sidecar state`,
    );
  }
  if (!finding.reproduction_command) {
    throw new Error(
      `answer key ${finding.id}: valid entry has no reproduction command`,
    );
  }
  if (!finding.locus) {
    throw new Error(`answer key ${finding.id}: valid entry has no locus state`);
  }
  if (!finding.healthy_control?.state) {
    throw new Error(
      `answer key ${finding.id}: valid entry has no healthy-control state`,
    );
  }
}

/** Backward-compatible coverage-only scorer used by focused unit tests. */
export function scoreAgainstKey(payload, key) {
  return scoreAgainstSources({ [COVERAGE_SOURCE]: { ok: true, payload } }, key);
}

/** Compare a run against the baseline. A regression is a finding LOST. */
export function diff(previous, current) {
  const before = new Set(previous?.detected ?? []);
  const after = new Set(current.detected ?? []);
  return {
    gained: [...after].filter((id) => !before.has(id)).sort(),
    lost: [...before].filter((id) => !after.has(id)).sort(),
    recallBefore: previous?.recall ?? null,
    recallAfter: current.recall,
  };
}

export function render(score, delta) {
  const notEvaluated = score.notEvaluated ?? [];
  const unavailable = score.unavailable ?? [];
  const invalidAnswerKey = score.invalidAnswerKey ?? [];
  const lines = [
    `answer-key findings: ${score.detected.length + score.missed.length + unavailable.length + notEvaluated.length + invalidAnswerKey.length}`,
    `  detected      ${score.detected.length} ${fmt(score.detected)}`,
    `  missed        ${score.missed.length} ${fmt(score.missed)}`,
    `  unavailable   ${unavailable.length} ${fmt(unavailable.map((item) => item.id))} — required immutable input absent; outside recall denominator`,
    `  not evaluated ${notEvaluated.length} ${fmt(notEvaluated.map((item) => item.id))} — runnable evaluation did not complete; outside recall denominator`,
    `  invalid key   ${invalidAnswerKey.length} ${fmt(invalidAnswerKey.map((item) => item.id))} — exact defect-bearing snapshot/locus absent; outside recall denominator`,
    `  recall        ${score.recall === null ? "n/a" : `${Math.round(score.recall * 100)}%`} (over the mechanized set)`,
  ];
  for (const item of unavailable) {
    lines.push(`  UNAVAILABLE ${item.id} via ${item.source}: ${item.reason}`);
  }
  for (const item of notEvaluated) {
    lines.push(`  NOT EVALUATED ${item.id} via ${item.source}: ${item.reason}`);
  }
  for (const item of invalidAnswerKey) {
    lines.push(`  INVALID ANSWER KEY ${item.id}: ${item.reason}`);
  }
  for (const item of score.controlFailures ?? []) {
    lines.push(
      `  CONTROL FAILURE ${item.id} via ${item.cohort}: ${item.reason}`,
    );
  }
  if (delta) {
    if (delta.gained.length) lines.push(`  gained: ${fmt(delta.gained)}`);
    if (delta.lost.length) {
      lines.push(`  LOST:   ${fmt(delta.lost)}`);
      lines.push("  ^ a finding the tools used to surface and no longer do.");
    }
    if (!delta.gained.length && !delta.lost.length)
      lines.push("  no change against the baseline");
  }
  return lines.join("\n");
}

const fmt = (ids) => (ids.length ? `(${ids.join(", ")})` : "");

function main() {
  const update = process.argv.includes("--update");
  const quire = argOf("--quire") ?? "quire";
  const module = argOf("--module") ?? null;
  const key = JSON.parse(readFileSync(ANSWER_KEY, "utf8"));

  const corpus = resolve(ROOT, argOf("--corpus") ?? "../filament-ide-rs");
  if (!existsSync(corpus)) {
    console.error(
      `battletest: tier-2 corpus absent at ${corpus} — refusing to score.`,
    );
    return 2;
  }
  const dirty = execFileSync("git", ["-C", corpus, "status", "--porcelain"], {
    encoding: "utf8",
  }).trim();
  if (dirty) {
    console.error(
      "battletest: the pinned corpus checkout is dirty — refusing to retain or compare output from bytes the recorded SHA does not identify.",
    );
    return 2;
  }

  validateCohortManifest(key, module);
  const cohorts = collectTier2Cohorts({ quire, corpus, module, key });
  const score = scoreAgainstCohorts(cohorts, key);
  const candidate = createTier2Baseline({
    provenance: tier2Provenance({ quire, module, key }),
    cohorts,
    score,
  });
  const previous = existsSync(BASELINE)
    ? JSON.parse(readFileSync(BASELINE, "utf8"))
    : null;
  const delta = diff(previous?.score ?? previous, score);
  const comparison = compareTier2Baseline(previous, candidate);
  console.log(render(score, delta));
  console.log(renderBaselineComparison(comparison));

  if (update) {
    const failed = Object.entries(cohorts).flatMap(([cohort, record]) =>
      Object.entries(record.sources).flatMap(([source, result]) =>
        result.state === "failed" ? [{ cohort, source }] : [],
      ),
    );
    if (failed.length > 0) {
      console.error(
        `\nrefusing to baseline failed Tier-2 source(s): ${failed.map((item) => `${item.cohort}/${item.source}`).join(", ")}`,
      );
      return 2;
    }
    mkdirSync(dirname(BASELINE), { recursive: true });
    writeFileSync(BASELINE, JSON.stringify(candidate, null, 2) + "\n");
    console.log(`\nbaseline rewritten: bench/battletest-baseline.json`);
    return 0;
  }
  return !comparison.comparable ||
    comparison.lost.length > 0 ||
    comparison.source_regressions.length > 0 ||
    Object.values(cohorts).some((cohort) =>
      Object.values(cohort.sources).some((source) => source.state === "failed"),
    )
    ? 1
    : 0;
}

/** Materialize every immutable cohort as an isolated worktree and retain its sources. */
export function collectTier2Cohorts({
  quire,
  corpus,
  module = null,
  quoin = QUOIN,
  key,
}) {
  const root = mkdtempSync(join(tmpdir(), "quoin-tier2-cohorts-"));
  const materialized = [];
  const out = {};
  try {
    for (const [id, cohort] of Object.entries(key.cohorts).sort(([a], [b]) =>
      a.localeCompare(b),
    )) {
      verifyCommitExists(corpus, cohort.revision, id);
      const checkout = join(root, safePathSegment(id));
      execFileSync(
        "git",
        [
          "-C",
          corpus,
          "worktree",
          "add",
          "--detach",
          checkout,
          cohort.revision,
        ],
        { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
      );
      materialized.push(checkout);
      const revision = execFileSync(
        "git",
        ["-C", checkout, "rev-parse", "HEAD"],
        { encoding: "utf8" },
      ).trim();
      if (revision !== cohort.revision) {
        throw new Error(
          `answer-key cohort ${id}: expected ${cohort.revision}, materialized ${revision}`,
        );
      }
      verifyEvidenceSidecar(checkout, cohort.evidence_sidecar, id);
      const sourceNames = sourcesForCohort(key, id);
      out[id] = {
        provenance: {
          corpus: {
            repository: cohort.repository ?? key.corpus,
            revision,
            checkout: "isolated-clean-worktree",
          },
          declaration: cohort.declaration,
          evidence_sidecar: cohort.evidence_sidecar,
          role: cohort.role,
        },
        sources: collectTier2Sources({
          quire,
          quoin,
          corpus: checkout,
          module,
          sourceNames,
        }),
      };
    }
    validateReproductionCommands(out, key);
    return out;
  } finally {
    for (const checkout of materialized.reverse()) {
      try {
        execFileSync(
          "git",
          ["-C", corpus, "worktree", "remove", "--force", checkout],
          {
            stdio: ["ignore", "pipe", "pipe"],
          },
        );
      } catch {
        // The parent temp directory is runner-owned; cleanup below is safe and
        // git's next worktree operation prunes any stale administrative entry.
      }
    }
    rmSync(root, { recursive: true, force: true });
  }
}

export function collectTier2Sources({
  quire,
  corpus,
  module = null,
  quoin = QUOIN,
  sourceNames = [COVERAGE_SOURCE, "quoin.validate", "quoin.evidence-audit"],
}) {
  const registry = {
    [COVERAGE_SOURCE]: () => coverage(quire, corpus, module),
    "quoin.validate": () => quoinValidate(quoin, corpus),
    "quoin.evidence-audit": () => evidenceAudit(quoin, quire, corpus, module),
  };
  return Object.fromEntries(
    [...new Set(sourceNames)].sort().map((name) => {
      if (!registry[name]) throw new Error(`unsupported Tier-2 source ${name}`);
      return [name, registry[name]()];
    }),
  );
}

function quoinValidate(quoin, corpus) {
  return jsonCommand(
    process.execPath,
    [quoin, "validate", "--repo", corpus, "--json"],
    "quoin validate",
    process.env,
    canonicalCommand("NODE", [
      "QUOIN",
      "validate",
      "--repo",
      "CORPUS",
      "--json",
    ]),
  );
}

function evidenceAudit(quoin, quire, corpus, module) {
  const bindings = join(corpus, "spec", "evidence", "bindings.json");
  const command = canonicalCommand(
    "NODE",
    [
      "QUOIN",
      "evidence",
      "audit",
      "--repo",
      "CORPUS",
      "--json",
      ...(module ? ["--module", "MODULE"] : []),
    ],
    { pathPrepend: "QUIRE_DIR" },
  );
  if (!existsSync(bindings)) {
    return {
      ok: false,
      state: "unavailable",
      command,
      reason:
        "spec/evidence/bindings.json is absent, so no suite-to-obligation join exists",
    };
  }
  const args = [quoin, "evidence", "audit", "--repo", corpus, "--json"];
  if (module) args.push("--module", module);
  return jsonCommand(
    process.execPath,
    args,
    "quoin evidence audit",
    {
      ...process.env,
      PATH: `${dirname(resolve(quire))}${delimiter}${process.env.PATH ?? ""}`,
    },
    command,
  );
}

function jsonCommand(
  executable,
  args,
  label,
  env = process.env,
  command = { executable, args },
) {
  try {
    return {
      ok: true,
      state: "evaluated",
      command,
      payload: JSON.parse(
        execFileSync(executable, args, {
          encoding: "utf8",
          env,
          maxBuffer: 256 * 1024 * 1024,
          stdio: ["ignore", "pipe", "pipe"],
        }),
      ),
    };
  } catch (error) {
    const detail = String(error.stderr ?? error.message)
      .trim()
      .split("\n")
      .pop();
    return {
      ok: false,
      state: "failed",
      command,
      reason: `${label} failed: ${detail?.slice(0, 200) ?? "unknown"}`,
    };
  }
}

function tier2Provenance({ quire, module, key }) {
  const quirePath = resolve(quire);
  const moduleRoot = module ? resolve(module) : join(ROOT, "corpus", "modules");
  return {
    answer_key_digest: fileDigest(ANSWER_KEY),
    cohort_manifest_digest: valueDigest(key.cohorts),
    declaration_checkout: gitProvenance(moduleRoot),
    tools: {
      quire: {
        version: execFileSync(quirePath, ["--version"], {
          encoding: "utf8",
        }).trim(),
        digest: fileDigest(quirePath),
      },
      quoin: {
        version: execFileSync(
          process.execPath,
          [join(ROOT, "bin", "quoin.js"), "--version"],
          { encoding: "utf8" },
        ).trim(),
        revision: gitProvenance(ROOT).revision,
        digest: treeDigest(join(ROOT, "dist")),
      },
    },
    environment: {
      node: process.version,
      platform: process.platform,
      arch: process.arch,
    },
  };
}

function validateCohortManifest(key, module) {
  if (key.schema_version !== "tier2-answer-key-v2") {
    throw new Error(
      `battletest: unsupported answer-key schema ${JSON.stringify(key.schema_version)}`,
    );
  }
  if (!key.cohorts || Object.keys(key.cohorts).length === 0) {
    throw new Error("battletest: answer key declares no immutable cohorts");
  }
  const moduleRoot = module ? resolve(module) : join(ROOT, "corpus", "modules");
  const declaration = gitProvenance(moduleRoot);
  if (declaration.dirty) {
    throw new Error(
      "battletest: declaration checkout is dirty; its bytes are not identified by the answer key",
    );
  }
  for (const [id, cohort] of Object.entries(key.cohorts)) {
    if (!/^[0-9a-f]{40}$/.test(cohort.revision ?? "")) {
      throw new Error(
        `answer-key cohort ${id}: revision must be a full immutable commit SHA`,
      );
    }
    if (cohort.declaration?.revision !== declaration.revision) {
      throw new Error(
        `answer-key cohort ${id}: declaration is pinned at ${cohort.declaration?.revision ?? "nothing"}, checkout is ${declaration.revision}`,
      );
    }
    if (!cohort.evidence_sidecar?.state) {
      throw new Error(
        `answer-key cohort ${id}: evidence-sidecar availability is undeclared`,
      );
    }
  }
  for (const finding of key.findings) {
    if (finding.answer_key_state === "invalid") continue;
    validateFindingDefinition(finding);
    if (!key.cohorts[finding.cohort]) {
      throw new Error(
        `answer key ${finding.id}: unknown defect cohort ${finding.cohort}`,
      );
    }
    if (
      finding.healthy_control?.state === "pinned" &&
      !key.cohorts[finding.healthy_control.cohort]
    ) {
      throw new Error(
        `answer key ${finding.id}: unknown control cohort ${finding.healthy_control.cohort}`,
      );
    }
  }
}

function sourcesForCohort(key, cohortId) {
  const names = new Set(key.cohorts[cohortId]?.retained_sources ?? []);
  for (const finding of key.findings) {
    if (finding.answer_key_state === "invalid") continue;
    if (finding.cohort === cohortId) names.add(finding.source);
    if (
      finding.healthy_control?.state === "pinned" &&
      finding.healthy_control.cohort === cohortId
    ) {
      names.add(finding.healthy_control.source ?? finding.source);
    }
  }
  return [...names];
}

function validateReproductionCommands(cohorts, key) {
  for (const finding of key.findings) {
    if (finding.answer_key_state === "invalid") continue;
    const source = cohorts[finding.cohort]?.sources?.[finding.source];
    if (
      canonicalJson(source?.command) !==
      canonicalJson(finding.reproduction_command)
    ) {
      throw new Error(
        `answer key ${finding.id}: reproduction command does not match the retained production source command`,
      );
    }
    if (finding.healthy_control?.state === "pinned") {
      const controlSource =
        cohorts[finding.healthy_control.cohort]?.sources?.[
          finding.healthy_control.source ?? finding.source
        ];
      if (
        canonicalJson(controlSource?.command) !==
        canonicalJson(finding.reproduction_command)
      ) {
        throw new Error(
          `answer key ${finding.id}: healthy-control command differs from its defect reproduction command`,
        );
      }
    }
  }
}

function verifyCommitExists(repository, revision, cohortId) {
  try {
    execFileSync(
      "git",
      ["-C", repository, "cat-file", "-e", `${revision}^{commit}`],
      {
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
  } catch {
    throw new Error(
      `answer-key cohort ${cohortId}: commit ${revision} is unavailable in the supplied repository history`,
    );
  }
}

function verifyEvidenceSidecar(checkout, sidecar, cohortId) {
  const path = join(checkout, sidecar?.path ?? "spec/evidence/bindings.json");
  if (sidecar?.state === "unavailable") {
    if (existsSync(path)) {
      throw new Error(
        `answer-key cohort ${cohortId}: evidence sidecar is declared unavailable but exists at ${sidecar.path}`,
      );
    }
    return;
  }
  if (sidecar?.state !== "bound" || !sidecar.digest) {
    throw new Error(
      `answer-key cohort ${cohortId}: sidecar must be unavailable or bound with a digest`,
    );
  }
  if (!existsSync(path) || fileDigest(path) !== sidecar.digest) {
    throw new Error(
      `answer-key cohort ${cohortId}: immutable evidence sidecar is absent or its digest changed`,
    );
  }
}

function safePathSegment(value) {
  if (!/^[a-z0-9][a-z0-9._-]*$/i.test(value)) {
    throw new Error(`unsafe answer-key cohort id ${JSON.stringify(value)}`);
  }
  return value;
}

function canonicalCommand(executable, args, options = {}) {
  const replacements = new Map([
    [options.scope, "CORPUS"],
    [options.module, "MODULE"],
  ]);
  return {
    executable,
    args: args.map((arg) => replacements.get(arg) ?? arg),
    ...(options.pathPrepend
      ? { environment: { PATH_prepend: options.pathPrepend } }
      : {}),
  };
}

function gitProvenance(path) {
  const revision = execFileSync("git", ["-C", path, "rev-parse", "HEAD"], {
    encoding: "utf8",
  }).trim();
  const dirty = execFileSync("git", ["-C", path, "status", "--porcelain"], {
    encoding: "utf8",
  }).trim();
  return { revision, dirty: dirty !== "" };
}

function fileDigest(path) {
  return `sha256:${createHash("sha256").update(readFileSync(path)).digest("hex")}`;
}

function valueDigest(value) {
  return `sha256:${createHash("sha256").update(canonicalJson(value)).digest("hex")}`;
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function treeDigest(path) {
  const output = execFileSync("find", [path, "-type", "f", "-print0"], {
    encoding: "buffer",
  });
  const files = output.toString("utf8").split("\0").filter(Boolean).sort();
  const hash = createHash("sha256");
  for (const file of files) {
    hash.update(file.slice(path.length));
    hash.update("\0");
    hash.update(readFileSync(file));
    hash.update("\n");
  }
  return `sha256:${hash.digest("hex")}`;
}

function renderBaselineComparison(comparison) {
  const lines = [
    `baseline comparable: ${comparison.comparable ? "yes" : "no"}`,
  ];
  if (comparison.input_mismatches.length) {
    lines.push(
      `  incomparable inputs: ${comparison.input_mismatches.join(", ")}`,
    );
  }
  if (comparison.source_regressions.length) {
    for (const item of comparison.source_regressions) {
      lines.push(
        `  SOURCE REGRESSION ${item.cohort}/${item.source}: ${item.before} -> ${item.after} (${item.reason})`,
      );
    }
  }
  if (comparison.source_changes.length) {
    lines.push(
      `  source output changed: ${comparison.source_changes.map((item) => `${item.cohort}/${item.source}`).join(", ")}`,
    );
  } else {
    lines.push("  source outputs byte-identical within v2 canonical ordering");
  }
  return lines.join("\n");
}

function compare(a, b) {
  return a === b ? 0 : a < b ? -1 : 1;
}

function argOf(flag) {
  const i = process.argv.indexOf(flag);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

if (process.argv[1] && process.argv[1].endsWith("battletest.mjs")) {
  process.exit(main());
}
