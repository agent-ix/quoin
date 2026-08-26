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
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { delimiter, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const ANSWER_KEY = join(ROOT, "bench", "answer-key.json");
const BASELINE = join(ROOT, "bench", "battletest-baseline.json");
const QUOIN = join(ROOT, "bin", "quoin.js");
const COVERAGE_SOURCE = "quire.coverage";

/** `quire coverage --json` over one scope, or a stated reason it could not run. */
function coverage(quire, scope, module) {
  const args = ["coverage", "--scope", scope, "--json"];
  if (module) args.push("--module", module);
  try {
    return {
      ok: true,
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
    return { ok: false, reason: stderr?.slice(0, 200) ?? "unknown" };
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
  const detected = [];
  const missed = [];
  const notMechanized = [];
  const notEvaluated = [];

  for (const finding of key.findings) {
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
    if (!source?.ok) {
      notMechanized.push(finding.id);
      notEvaluated.push({
        id: finding.id,
        source: sourceName,
        reason: source?.reason ?? "the answer key names no runnable source",
      });
      continue;
    }

    const payload = source.payload;
    const reasons = new Set((payload.diagnostics ?? []).map((d) => d.reason));
    const suspicions = new Set((payload.suspicions ?? []).map((s) => s.kind));
    const metrics = new Map((payload.metrics ?? []).map((m) => [m.name, m]));
    const findings = new Set((payload.findings ?? []).map((f) => f.kind));

    if (finding.expect_reason) {
      (reasons.has(finding.expect_reason) ? detected : missed).push(finding.id);
    } else if (finding.expect_suspicion) {
      (suspicions.has(finding.expect_suspicion) ? detected : missed).push(
        finding.id,
      );
    } else if (finding.expect_metric) {
      const metric = metrics.get(finding.expect_metric);
      const hit =
        metric && Number(metric.value) === Number(finding.expect_value);
      (hit ? detected : missed).push(finding.id);
    } else if (finding.expect_finding) {
      (findings.has(finding.expect_finding) ? detected : missed).push(
        finding.id,
      );
    } else {
      notMechanized.push(finding.id);
      notEvaluated.push({
        id: finding.id,
        source: sourceName,
        reason: "the answer key declares no signal to score",
      });
    }
  }
  const denominator = detected.length + missed.length;
  return {
    detected: detected.sort(),
    missed: missed.sort(),
    notMechanized: notMechanized.sort(),
    notEvaluated: notEvaluated.sort((a, b) => compare(a.id, b.id)),
    // `null`, not 0, when nothing is mechanized — 0/0 is not 0% recall.
    recall:
      denominator === 0
        ? null
        : Number((detected.length / denominator).toFixed(3)),
  };
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
  const lines = [
    `answer-key findings: ${score.detected.length + score.missed.length + score.notMechanized.length}`,
    `  detected      ${score.detected.length} ${fmt(score.detected)}`,
    `  missed        ${score.missed.length} ${fmt(score.missed)}`,
    `  not evaluated  ${score.notMechanized.length} ${fmt(score.notMechanized)} — source premise unavailable; not counted as a miss`,
    `  recall        ${score.recall === null ? "n/a" : `${Math.round(score.recall * 100)}%`} (over the mechanized set)`,
  ];
  for (const item of notEvaluated) {
    lines.push(`  NOT EVALUATED ${item.id} via ${item.source}: ${item.reason}`);
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
  const head = execFileSync(
    "git",
    ["-C", corpus, "rev-parse", "--short", "HEAD"],
    {
      encoding: "utf8",
    },
  ).trim();
  if (!head.startsWith(key.pinned_sha) && !key.pinned_sha.startsWith(head)) {
    console.error(
      `battletest: the answer key is pinned at ${key.pinned_sha} and the corpus ` +
        `is at ${head}. Refusing to score — the findings were adjudicated against ` +
        `the pinned tree, and carrying them to another one asserts something ` +
        `nobody checked. Check the corpus out at the pin, or re-adjudicate with ` +
        `\`make answer-key-repin\`.`,
    );
    return 2;
  }

  const sources = collectTier2Sources({ quire, corpus, module });
  const score = scoreAgainstSources(sources, key);
  const previous = existsSync(BASELINE)
    ? JSON.parse(readFileSync(BASELINE, "utf8"))
    : null;
  const delta = diff(previous, score);
  console.log(render(score, delta));

  if (update) {
    if (score.notEvaluated.length > 0) {
      console.error(
        "\nrefusing to baseline a Tier-2 run with unevaluated answer-key entries",
      );
      return 2;
    }
    mkdirSync(dirname(BASELINE), { recursive: true });
    writeFileSync(
      BASELINE,
      JSON.stringify({ ...score, pinned_sha: head }, null, 2) + "\n",
    );
    console.log(`\nbaseline rewritten: bench/battletest-baseline.json`);
    return 0;
  }
  return delta.lost.length > 0 || score.notEvaluated.length > 0 ? 1 : 0;
}

export function collectTier2Sources({
  quire,
  corpus,
  module = null,
  quoin = QUOIN,
}) {
  return {
    [COVERAGE_SOURCE]: coverage(quire, corpus, module),
    "quoin.validate": quoinValidate(quoin, corpus),
    "quoin.evidence-audit": evidenceAudit(quoin, quire, corpus, module),
  };
}

function quoinValidate(quoin, corpus) {
  return jsonCommand(
    process.execPath,
    [quoin, "validate", "--repo", corpus, "--json"],
    "quoin validate",
  );
}

function evidenceAudit(quoin, quire, corpus, module) {
  const bindings = join(corpus, "spec", "evidence", "bindings.json");
  if (!existsSync(bindings)) {
    return {
      ok: false,
      reason:
        "spec/evidence/bindings.json is absent, so no suite-to-obligation join exists",
    };
  }
  const args = [quoin, "evidence", "audit", "--repo", corpus, "--json"];
  if (module) args.push("--module", module);
  return jsonCommand(process.execPath, args, "quoin evidence audit", {
    ...process.env,
    PATH: `${dirname(resolve(quire))}${delimiter}${process.env.PATH ?? ""}`,
  });
}

function jsonCommand(executable, args, label, env = process.env) {
  try {
    return {
      ok: true,
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
      reason: `${label} failed: ${detail?.slice(0, 200) ?? "unknown"}`,
    };
  }
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
