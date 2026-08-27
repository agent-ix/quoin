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
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
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
    if (source?.state !== "evaluated") {
      notMechanized.push(finding.id);
      notEvaluated.push({
        id: finding.id,
        source: sourceName,
        reason: source?.reason ?? "the answer key names no runnable source",
      });
      continue;
    }

    const normalizedFindings = source.normalized?.findings ?? [];
    for (const record of normalizedFindings) validateFindingEnvelope(record);
    const reasons = findingFamilies(normalizedFindings, "coverage.diagnostics");
    const suspicions = findingFamilies(
      normalizedFindings,
      "coverage.suspicions",
    );
    const metrics = new Map(
      (source.normalized?.metrics ?? []).map((metric) => [metric.name, metric]),
    );
    const findings = findingFamilies(normalizedFindings);

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

function findingFamilies(findings, channel = null) {
  return new Set(
    findings
      .filter(
        (finding) => channel === null || finding.source.channel === channel,
      )
      .map((finding) => finding.identity?.family ?? finding.kind),
  );
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
  const head = execFileSync("git", ["-C", corpus, "rev-parse", "HEAD"], {
    encoding: "utf8",
  }).trim();
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
  const dirty = execFileSync("git", ["-C", corpus, "status", "--porcelain"], {
    encoding: "utf8",
  }).trim();
  if (dirty) {
    console.error(
      "battletest: the pinned corpus checkout is dirty — refusing to retain or compare output from bytes the recorded SHA does not identify.",
    );
    return 2;
  }

  const sources = collectTier2Sources({ quire, corpus, module });
  const score = scoreAgainstSources(sources, key);
  const candidate = createTier2Baseline({
    provenance: tier2Provenance({ quire, corpus, module, head }),
    sources,
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
    const failed = Object.entries(sources).filter(
      ([, source]) => source.state === "failed",
    );
    if (failed.length > 0) {
      console.error(
        `\nrefusing to baseline failed Tier-2 source(s): ${failed.map(([name]) => name).join(", ")}`,
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
    Object.values(sources).some((source) => source.state === "failed")
    ? 1
    : 0;
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

function tier2Provenance({ quire, corpus, module, head }) {
  const quirePath = resolve(quire);
  const moduleRoot = module ? resolve(module) : join(ROOT, "corpus", "modules");
  return {
    answer_key_digest: fileDigest(ANSWER_KEY),
    corpus: {
      repository: "agent-ix/filament-ide-rs",
      revision: head,
      checkout: "isolated-clean-worktree",
    },
    declaration: gitProvenance(moduleRoot),
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
        `  SOURCE REGRESSION ${item.source}: ${item.before} -> ${item.after} (${item.reason})`,
      );
    }
  }
  if (comparison.source_changes.length) {
    lines.push(
      `  source output changed: ${comparison.source_changes.map((item) => item.source).join(", ")}`,
    );
  } else {
    lines.push("  source outputs byte-identical within v1 canonical ordering");
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
