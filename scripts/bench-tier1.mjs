#!/usr/bin/env node
// The tier-1 benchmark runner (agent-ix/quoin#199, FR-043-AC-2/AC-7).
//
// `buildBenchCorpora` had no production caller. The corpora were built only by
// a unit test, into a temp directory, and scored by nobody — so every metric in
// `bench/metrics.json` that depends on them carried `baseline: null` with the
// note "No tier-1 run has been scored against a toolchain yet."
//
// This is that runner. It materializes the seeded corpora, runs the real tools
// over each, maps their payloads to findings through the committed table in
// `bench/tier1-mapping.json`, and scores them against the labels the corpora
// carry.
//
//   node scripts/bench-tier1.mjs                  # score and diff
//   node scripts/bench-tier1.mjs --update         # deliberate re-baseline
//   node scripts/bench-tier1.mjs --json           # the score, machine-readable
//
// Ratchet semantics are quire-rs `scripts/bench.py`'s, deliberately: a closed
// metric dictionary that refuses an undeclared name, a one-way compare where a
// regression keeps the OLD baseline so a bad run can never lower the bar, and
// an unreadable metric OMITTED rather than reported as 0.

import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { buildBenchCorpora } from "../evals/fixtures/bench/build.mjs";
import { crossCheckFamilies, loadMetrics } from "../evals/lib/dictionary.mjs";
import { scoreActionability, scoreFindings } from "../evals/lib/quality.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const MAPPING = join(ROOT, "bench", "tier1-mapping.json");
const METRICS = join(ROOT, "bench", "metrics.json");
const BASELINE = join(ROOT, "bench", "tier1-baseline.json");

/**
 * The bracketed reason a `validate` finding ends with, and the path and line it
 * opens with.
 *
 * Parsed rather than read from a field because `validate` has no JSON payload —
 * `--diagnostics-format json` wraps the whole human string in `message`. The
 * fragility is declared in `bench/tier1-mapping.json` and tracked as
 * agent-ix/quire-cli#65; a message this cannot parse is an ERROR here, never a
 * skip, because a family that silently stops scoring looks identical to a
 * family with nothing to report.
 */
const VALIDATE_LINE =
  /^(?<path>.+?): line (?<line>\d+): (?<rest>.*) \[(?<reason>[a-z-]+)\]$/;

/** Run a command, returning stdout, stderr and whether it exited zero. */
function run(bin, args) {
  try {
    const stdout = execFileSync(bin, args, {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
      stdio: ["ignore", "pipe", "pipe"],
    });
    return { ok: true, stdout, stderr: "" };
  } catch (error) {
    // A non-zero exit is normal here: `validate` exits 1 when a document fails,
    // which is exactly the case being measured. The output is what matters.
    return {
      ok: false,
      stdout: String(error.stdout ?? ""),
      stderr: String(error.stderr ?? error.message ?? ""),
    };
  }
}

/** Every finding one corpus produced, already mapped to a family. */
function findingsFor(quire, corpusRoot, mapping) {
  const module = join(corpusRoot, "module");
  const out = [];
  const bySource = (name) =>
    Object.entries(mapping.families).filter(([, m]) => m.source === name);

  // ── quire coverage ──
  const cov = run(quire, [
    "coverage",
    "--scope",
    corpusRoot,
    "--module",
    module,
    "--json",
  ]);
  let payload = null;
  try {
    payload = JSON.parse(cov.stdout);
  } catch {
    throw new Error(
      `bench-tier1: \`quire coverage\` produced no JSON for ${corpusRoot}. ` +
        `Refusing to score a corpus whose payload could not be read — an ` +
        `unreadable run and a clean run are the same zero.\n${cov.stderr.trim()}`,
    );
  }

  for (const [family, m] of bySource("coverage.diagnostics")) {
    for (const d of payload.diagnostics ?? []) {
      if (d.reason !== m.key) continue;
      out.push({ family, reason: d.reason, path: d.path ?? null, line: null });
    }
  }
  for (const [family, m] of bySource("coverage.suspicions")) {
    for (const s of payload.suspicions ?? []) {
      if (s.kind !== m.key) continue;
      out.push({
        family,
        reason: s.kind,
        path: s.path ?? null,
        line: typeof s.line === "number" ? s.line : null,
      });
    }
  }
  // A metric is a finding only when it sits at the value a label expects. The
  // metric MOVING is the signal, so the expectation lives on the label and this
  // stage carries the observed value for the caller to compare.
  const metrics = new Map((payload.metrics ?? []).map((m) => [m.name, m]));
  for (const [family, m] of bySource("coverage.metrics")) {
    const metric = metrics.get(m.key);
    if (!metric || metric.state !== "measured") continue;
    out.push({
      family,
      reason: m.key,
      path: null,
      line: null,
      metric: m.key,
      value: Number(metric.value),
    });
  }

  // ── quire validate ──
  const val = run(quire, [
    "validate",
    "--diagnostics-format",
    "json",
    "--scope",
    corpusRoot,
    "--module",
    module,
    "spec/*.md",
  ]);
  const wanted = new Set(bySource("validate.findings").map(([, m]) => m.key));
  const familyOfReason = new Map(
    bySource("validate.findings").map(([family, m]) => [m.key, family]),
  );
  for (const raw of val.stderr.split("\n")) {
    const text = raw.trim();
    if (!text.startsWith("{")) continue;
    let record;
    try {
      record = JSON.parse(text);
    } catch {
      continue;
    }
    if (record.kind !== "ValidationError") continue;
    const parsed = VALIDATE_LINE.exec(record.message);
    if (!parsed) {
      // Not a locatable finding (the run summary is a ValidationError too).
      // Only refuse when it LOOKS like a finding: `<path>: line N:`.
      if (/^.+: line \d+:/.test(record.message)) {
        throw new Error(
          `bench-tier1: could not parse a validate finding's path, line and ` +
            `reason from:\n  ${record.message}\n` +
            `The message format is the only place they exist ` +
            `(agent-ix/quire-cli#65). Refusing to skip it — a family that ` +
            `silently stops scoring reads as a family with nothing to report.`,
        );
      }
      continue;
    }
    const { path, line, reason } = parsed.groups;
    if (!wanted.has(reason)) continue;
    out.push({
      family: familyOfReason.get(reason),
      reason,
      path,
      line: Number(line),
    });
  }

  return out;
}

/** Flatten `labels.json`'s `{corpora:[{defects}]}` into the flat array scoring takes. */
export function flattenLabels(labels) {
  // The shape mismatch that kept `buildBenchCorpora` and `scoreFindings` from
  // ever meeting: the builder writes a wrapper keyed by corpus, the scorer
  // consumes a flat list. Converted in ONE place so the two cannot drift.
  return labels.corpora.flatMap((c) =>
    c.defects.map((d) => ({ ...d, corpus: c.name })),
  );
}

/**
 * The fraction of true positives whose reported location matches the label's.
 *
 * The metric that encodes the actual requirement: an alert must say WHERE.
 * `scoreFindings` already counts positional pairings; this is their share of
 * the confirmed findings, and `null` — never 0 — when nothing was confirmed,
 * because 0/0 is not 0%.
 */
export function localisationRate(score) {
  const truePositives = score.families.reduce((n, f) => n + f.truePositives, 0);
  if (truePositives === 0) return null;
  return Number((score.positional / truePositives).toFixed(3));
}

/**
 * quire-rs `bench.py`'s `compare`, in JS and with its semantics intact.
 *
 * A regression keeps the OLD baseline, so a bad run can never implicitly lower
 * the bar; `gate-zero` carries no baseline and no tolerance, so `--update`
 * cannot launder it; a missing baseline is `new`, never a pass by default.
 */
export function compare(direction, observed, baseline) {
  if (direction === "gate-zero") {
    return [observed === 0 ? "held" : "regressed", 0];
  }
  if (baseline === null || baseline === undefined) return ["new", observed];
  const better =
    direction === "higher-is-better"
      ? observed > baseline
      : observed < baseline;
  if (better) return ["improved", observed];
  if (observed === baseline) return ["held", baseline];
  return ["regressed", baseline];
}

function main() {
  const update = process.argv.includes("--update");
  const asJson = process.argv.includes("--json");
  const quire = argOf("--quire") ?? "quire";

  const mapping = JSON.parse(readFileSync(MAPPING, "utf8"));
  const dictionary = loadMetrics(METRICS);

  const root = mkdtempSync(join(tmpdir(), "quoin-tier1-"));
  let report;
  try {
    const labels = buildBenchCorpora(root);
    const flat = flattenLabels(labels);

    // Both directions, before anything is scored: a declared family with no
    // corpus and a corpus family no metric governs are each a hole in the
    // score, and finding them after a run wastes the run.
    crossCheckFamilies(
      dictionary.families,
      labels.corpora.map((c) => c.family),
      { path: "bench/metrics.json" },
    );

    const found = [];
    for (const corpus of labels.corpora) {
      found.push(...findingsFor(quire, join(root, corpus.name), mapping));
    }

    // A metric-sourced finding counts only at the value its label expects; the
    // metric merely EXISTING says nothing. Filtered here rather than inside
    // `findingsFor` so the mapping stage stays a pure payload read.
    const expectedValues = new Map(
      flat
        .filter((l) => l.expect_metric !== undefined)
        .map((l) => [l.expect_metric, Number(l.expect_value)]),
    );
    const scoredFindings = found.filter(
      (f) => f.metric === undefined || expectedValues.get(f.metric) === f.value,
    );

    const score = scoreFindings(scoredFindings, flat);
    report = {
      families: score.families,
      excluded: score.excluded,
      collateral: score.collateral,
      positional: score.positional,
      finding_localisation_rate: localisationRate(score),
      actionability: scoreActionability(scoredFindings),
      corpora: labels.corpora.length,
      findings: scoredFindings.length,
    };
  } finally {
    rmSync(root, { recursive: true, force: true });
  }

  const previous = existsSync(BASELINE)
    ? JSON.parse(readFileSync(BASELINE, "utf8"))
    : null;
  const verdicts = ratchet(report, previous, dictionary);

  if (asJson) {
    console.log(JSON.stringify({ ...report, verdicts }, null, 2));
  } else {
    console.log(render(report, verdicts));
  }

  if (update) {
    writeFileSync(BASELINE, JSON.stringify(report, null, 2) + "\n");
    console.error(`bench-tier1: baseline rewritten at ${BASELINE}`);
    return 0;
  }
  return verdicts.some((v) => v.verdict === "regressed") ? 1 : 0;
}

/** Per-family precision and recall against the baseline, one-way. */
export function ratchet(report, previous, dictionary) {
  const out = [];
  const before = new Map((previous?.families ?? []).map((f) => [f.family, f]));
  for (const family of report.families) {
    for (const metric of ["precision", "recall"]) {
      const declared = dictionary.metrics[`finding_${metric}`];
      // A metric this run could not read is OMITTED, never reported as 0 —
      // `null` precision means no denominator, and calling that a regression
      // would fail the build for a family nothing fired on.
      if (family[metric] === null) continue;
      const [verdict, kept] = compare(
        declared.direction,
        family[metric],
        before.get(family.family)?.[metric] ?? null,
      );
      out.push({
        metric: `finding_${metric}`,
        family: family.family,
        observed: family[metric],
        baseline: kept,
        verdict,
      });
    }
  }
  // A family the baseline scored and this run does not report AT ALL — its
  // corpus deleted, its mapping dropped, its label removed. Without this the
  // family simply vanishes from the table and the ratchet says nothing, so
  // deleting a corpus reads as a clean run. That is the same shape as a check
  // that cannot fail: the absence of a row is indistinguishable from an
  // absence of news.
  const now = new Set(report.families.map((f) => f.family));
  for (const f of previous?.families ?? []) {
    if (now.has(f.family)) continue;
    out.push({
      metric: "finding_recall",
      family: f.family,
      observed: null,
      baseline: f.recall,
      verdict: "regressed",
      why: "the baseline scored this family and this run did not report it at all",
    });
  }

  if (report.finding_localisation_rate !== null) {
    const [verdict, kept] = compare(
      "higher-is-better",
      report.finding_localisation_rate,
      previous?.finding_localisation_rate ?? null,
    );
    out.push({
      metric: "finding_localisation_rate",
      family: null,
      observed: report.finding_localisation_rate,
      baseline: kept,
      verdict,
    });
  }
  return out;
}

const MARK = { improved: "++", held: "ok", new: "**", regressed: "!!" };

function render(report, verdicts) {
  const pct = (v) =>
    v === null ? "  n/a" : `${Math.round(v * 100)}%`.padStart(5);
  const lines = [
    `tier-1: ${report.corpora} corpora, ${report.findings} findings mapped`,
    "",
    "family                     TP  FP  miss   prec  recall",
  ];
  for (const f of report.families) {
    lines.push(
      `  ${f.family.padEnd(24)} ${String(f.truePositives).padStart(2)}  ` +
        `${String(f.falsePositives).padStart(2)}  ${String(f.misses).padStart(4)}  ` +
        `${pct(f.precision)}  ${pct(f.recall)}`,
    );
  }
  lines.push(
    "",
    `finding_localisation_rate  ${pct(report.finding_localisation_rate)} ` +
      `(${report.positional} of ${report.families.reduce((n, f) => n + f.truePositives, 0)} true positives named where)`,
    `actionability_rate         ${pct(report.actionability.rate)} ` +
      `(${report.actionability.actionable} of ${report.actionability.total} findings name a row or a line)`,
  );
  if (report.collateral.length) {
    lines.push(
      "",
      "declared collateral, set aside from scoring:",
      ...report.collateral.map((c) => `  ${c.family} (${c.reason})`),
    );
  }
  if (report.excluded.length) {
    lines.push(`excluded as not findable: ${report.excluded.join(", ")}`);
  }
  lines.push("", "ratchet:");
  for (const v of verdicts) {
    const name = v.family ? `${v.metric}[${v.family}]` : v.metric;
    lines.push(
      `  ${MARK[v.verdict]} ${name.padEnd(40)} ${v.observed} (baseline ${v.baseline})` +
        (v.why ? ` — ${v.why}` : ""),
    );
  }
  if (verdicts.some((v) => v.verdict === "regressed")) {
    lines.push(
      "",
      "REGRESSED — the baseline is kept, never lowered by a bad run.",
    );
  } else if (verdicts.some((v) => v.verdict === "improved")) {
    lines.push(
      "",
      "improved — run `make bench-tier1-update` to move the baseline.",
    );
  }
  return lines.join("\n");
}

function argOf(flag) {
  const i = process.argv.indexOf(flag);
  return i === -1 ? null : process.argv[i + 1];
}

if (
  resolve(process.argv[1] ?? "") === resolve(fileURLToPath(import.meta.url))
) {
  process.exit(main());
}
