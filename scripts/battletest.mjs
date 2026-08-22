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
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const ANSWER_KEY = join(ROOT, "bench", "answer-key.json");
const BASELINE = join(ROOT, "bench", "battletest-baseline.json");

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
 * no signal is UNDETECTABLE BY CONSTRUCTION and is reported as such, never
 * counted as a miss: AK-006 and AK-007 have no mechanized detector yet, and
 * scoring them as failures would make the number move when nothing changed.
 */
export function scoreAgainstKey(payload, key) {
  const reasons = new Set((payload.diagnostics ?? []).map((d) => d.reason));
  const suspicions = new Set((payload.suspicions ?? []).map((s) => s.kind));
  const metrics = new Map((payload.metrics ?? []).map((m) => [m.name, m]));

  const detected = [];
  const missed = [];
  const notMechanized = [];

  for (const finding of key.findings) {
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
    } else {
      notMechanized.push(finding.id);
    }
  }
  const denominator = detected.length + missed.length;
  return {
    detected: detected.sort(),
    missed: missed.sort(),
    notMechanized: notMechanized.sort(),
    // `null`, not 0, when nothing is mechanized — 0/0 is not 0% recall.
    recall:
      denominator === 0
        ? null
        : Number((detected.length / denominator).toFixed(3)),
  };
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
  const lines = [
    `answer-key findings: ${score.detected.length + score.missed.length + score.notMechanized.length}`,
    `  detected      ${score.detected.length} ${fmt(score.detected)}`,
    `  missed        ${score.missed.length} ${fmt(score.missed)}`,
    `  not mechanized ${score.notMechanized.length} ${fmt(score.notMechanized)} — no detector exists; not counted as a miss`,
    `  recall        ${score.recall === null ? "n/a" : `${Math.round(score.recall * 100)}%`} (over the mechanized set)`,
  ];
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

  const run = coverage(quire, corpus, module);
  if (!run.ok) {
    console.error(`battletest: ${run.reason}`);
    return 2;
  }
  const score = scoreAgainstKey(run.payload, key);
  const previous = existsSync(BASELINE)
    ? JSON.parse(readFileSync(BASELINE, "utf8"))
    : null;
  const delta = diff(previous, score);
  console.log(render(score, delta));

  if (update) {
    mkdirSync(dirname(BASELINE), { recursive: true });
    writeFileSync(
      BASELINE,
      JSON.stringify({ ...score, pinned_sha: head }, null, 2) + "\n",
    );
    console.log(`\nbaseline rewritten: bench/battletest-baseline.json`);
    return 0;
  }
  return delta.lost.length > 0 ? 1 : 0;
}

function argOf(flag) {
  const i = process.argv.indexOf(flag);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

if (process.argv[1] && process.argv[1].endsWith("battletest.mjs")) {
  process.exit(main());
}
