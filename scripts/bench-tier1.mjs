#!/usr/bin/env node
// The tier-1 benchmark runner (agent-ix/quoin#199, FR-043-AC-2/AC-7).
//
// `buildBenchCorpora` had no production caller. The corpora were built only by
// a unit test, into a temp directory, and scored by nobody — so every metric in
// `bench/metrics.json` that depends on them carried `baseline: null` with the
// note "No tier-1 run has been scored against a toolchain yet."
//
// This is that runner. It reads STATIC cases from the `agent-ix/qa-corpus`
// submodule at `corpus/`, runs the real tools over each in place, maps their
// payloads to findings through the committed table in
// `bench/tier1-mapping.json`, and scores them against the labels in
// `corpus/labels/`.
//
// It used to GENERATE the corpora into a tmpdir from 550 lines of JavaScript
// (agent-ix/quoin#227). Nothing was on disk to inspect, diff or `cd` into, and
// the generator declared its own synthetic module — `section: Test Cases` where
// the ecosystem declares `Test Case Summary`. A corpus whose manifest heading
// always matches CANNOT EXHIBIT the defect that accounts for 3,514 unminted TC
// ids across 88 repositories, which is why tier 1 never caught the dominant
// ecosystem failure mode. That was a correctness defect in the benchmark.
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
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { parse as parseYaml } from "yaml";
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
function run(bin, args, extraEnv) {
  try {
    const stdout = execFileSync(bin, args, {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
      stdio: ["ignore", "pipe", "pipe"],
      env: extraEnv ? { ...process.env, ...extraEnv } : process.env,
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
function findingsFor(quire, corpusRoot, module, mapping) {
  const out = [];
  const bySource = (name) =>
    Object.entries(mapping.families).filter(([, m]) => m.source === name);

  // ── quire coverage ──
  //
  // A module id names either ONE module (`manifest.yaml` directly) or a module
  // PATH — a directory of module directories. The ecosystem declaration is the
  // second: `spec-artifacts-process` carries the traceability model and
  // `spec-artifacts-iso` declares FR/NFR/TestMatrix, and loading only the first
  // leaves criteria classification silently producing nothing while the totals
  // look identical (agent-ix/quire-rs#292). `--module` takes one directory, so
  // a path goes through the search-path env var.
  const single = existsSync(join(module, "manifest.yaml"));
  const args = single
    ? ["coverage", "--scope", corpusRoot, "--module", module, "--json"]
    : ["coverage", "--scope", corpusRoot, "--json"];
  const env = single ? undefined : { IX_FILAMENT_MODULES_PATH: module };
  const cov = run(quire, args, env);
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
  const val = run(
    quire,
    single
      ? [
          "validate",
          "--diagnostics-format",
          "json",
          "--scope",
          corpusRoot,
          "--module",
          module,
          "spec/*.md",
        ]
      : [
          "validate",
          "--diagnostics-format",
          "json",
          "--scope",
          corpusRoot,
          "spec/*.md",
        ],
    env,
  );
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
    // A family MAY narrow past the reason token with `contains`. `assert` is
    // the validator's CATEGORY and every ecosystem-bound fixture emits three
    // shared structural ones; without this the family absorbs all of them and
    // its precision measures this table rather than the detector.
    const narrow = mapping.families[familyOfReason.get(reason)]?.contains;
    if (narrow && !record.message.includes(narrow)) continue;
    out.push({
      family: familyOfReason.get(reason),
      reason,
      path,
      line: Number(line),
    });
  }

  return out;
}

/**
 * Every labelled case in the corpus: its `case.yaml`, its input directory, and
 * the adjudicated defects from `corpus/labels/<id>.yaml`.
 *
 * Read from disk, in place. A case with no label file is a case that seeds no
 * defect — the clean control — and is still SCORED, because a detector that
 * fires on healthy input is exactly what precision is for.
 */
export function loadCorpus(mapping = null, root = join(ROOT, "corpus")) {
  // The mapping resolves a diagnostic reason to the family it belongs to, so a
  // case's own `expect.yaml` can state ground truth without restating the
  // family. Optional so a caller that only wants the case list (a test
  // asserting corpus shape) need not load it.
  mapping ??= existsSync(MAPPING)
    ? JSON.parse(readFileSync(MAPPING, "utf8"))
    : { families: {} };
  const casesRoot = join(root, "cases");
  if (!existsSync(casesRoot)) {
    throw new Error(
      `bench-tier1: the corpus submodule is not checked out at ${root}. ` +
        `Run \`git submodule update --init\`.`,
    );
  }
  const corpora = [];
  for (const mode of readdirSync(casesRoot).sort()) {
    const modeDir = join(casesRoot, mode);
    for (const name of readdirSync(modeDir).sort()) {
      const dir = join(modeDir, name);
      const caseYaml = join(dir, "case.yaml");
      if (!existsSync(caseYaml)) continue;
      const meta = parseYaml(readFileSync(caseYaml, "utf8"));
      const labelPath = join(root, "labels", `${meta.id}.yaml`);
      const label = existsSync(labelPath)
        ? parseYaml(readFileSync(labelPath, "utf8"))
        : null;
      const expectPath = join(dir, "expect.yaml");
      const expect = existsSync(expectPath)
        ? (parseYaml(readFileSync(expectPath, "utf8")) ?? {})
        : {};

      corpora.push({
        name: meta.id,
        family: label?.family ?? familyOf(meta, expect, mapping),
        summary: label?.summary ?? meta.comment ?? "",
        defects: label?.defects ?? defectsFrom(meta, expect, mapping),
        input: join(dir, "input"),
        module: join(root, "modules", meta.module),
        pending: meta.pending ?? null,
      });
    }
  }
  return { corpora };
}

/**
 * The ground truth a case states about itself, for a case with no label file.
 *
 * THE DEFECT THIS FIXES: the first version treated "no label file" as "seeds no
 * defect", so every case quoin had not separately adjudicated was scored as
 * healthy input — and every CORRECT detection on it counted as a false
 * positive. Fourteen of twenty-two cases were unlabelled and **eight of those
 * declare `kind: failure`**, with an `expect.yaml` naming the exact diagnostics
 * quire emits. The reported precision collapse (1.0 -> 0.083) was the benchmark
 * penalising the engine for being right, against ground truth sitting unread in
 * the same submodule.
 *
 * `case.yaml`'s `kind`/`findable` and `expect.yaml`'s `diagnostic_reasons` are
 * the corpus's own statement of what should be found. A separate `labels/` file
 * adds quoin's finer adjudication — location, collateral, `expect_metric` — and
 * wins where it exists.
 */
function defectsFrom(meta, expect, mapping) {
  if (meta.kind !== "failure") return [];
  // ONE defect per case, not one per expected reason. A case isolates a single
  // family by construction (a mini-repo mixing three defects cannot tell you
  // which one a finding was about) — and the SECOND reason a seeded defect
  // produces is COLLATERAL, which the runner already models. Deriving both as
  // seeded defects broke the corpus's own isolation property, which is what
  // `every corpus isolates ONE defect family` caught.
  for (const reason of expect.diagnostic_reasons ?? []) {
    const entry = Object.entries(mapping?.families ?? {}).find(
      ([, m]) => m.key === reason,
    );
    if (!entry) continue;
    return [
      {
        // The corpus's id convention — two initials and an ordinal — made
        // unique across the whole set, which `defect ids are unique across the
        // whole corpus set` caught: `marker-form-mismatch` and
        // `marker-form-declared` both initialise to `MF`.
        id: `${initials(meta.id)}-${ordinal(meta.id)}`,
        family: entry[0],
        location: expect.diagnostic_paths?.[reason] ?? null,
        findable: meta.findable !== false,
        expect_reason: reason,
        confirmed_at: "derived from the case's own expect.yaml",
        // The REMAINING expected reasons are collateral, not second seeded
        // defects. Making one fire makes the others fire by construction — an
        // unreadable marker makes `coverage.backed` a ratio over an unread
        // population, so `no-symbol-bound` and `hollow-denominator` are one
        // situation seen through two lenses. Dropping them (the first fix for
        // the one-family-per-case violation) left each correct collateral
        // firing counted as a false positive.
        collateral: (expect.diagnostic_reasons ?? [])
          .filter((r) => r !== reason)
          .map((r) => {
            const e = Object.entries(mapping?.families ?? {}).find(
              ([, m]) => m.key === r,
            );
            return e
              ? {
                  family: e[0],
                  reason: r,
                  // Required by TC-950: collateral suppresses a finding from
                  // the precision denominator, so a declaration with no stated
                  // reason is a licence to launder false positives. This one is
                  // DERIVED rather than adjudicated, and says so — the case
                  // states both reasons in its own `expect.yaml`, so both are
                  // expected consequences of the one situation it seeds, but
                  // nobody has written down why they co-occur.
                  note:
                    `derived: the case's own expect.yaml expects both ` +
                    `\`${reason}\` and \`${r}\`, so this is a consequence of ` +
                    `the one defect it seeds, not a second seeded defect. Not ` +
                    `separately adjudicated.`,
                }
              : null;
          })
          .filter(Boolean),
        note:
          "Derived, not adjudicated: the corpus states in `expect.yaml` what " +
          "should be found, and an unlabelled failure case is NOT healthy " +
          "input. Treating it as such counted every correct detection on it " +
          "as a false positive (agent-ix/quoin#227 review).",
      },
    ];
  }
  return [];
}

/**
 * A stable ordinal for a case id, so two cases sharing initials do not share a
 * defect id. Derived from the id rather than a counter: a counter depends on
 * walk order, and two runners walking differently would disagree about which
 * defect is which.
 */
function ordinal(id) {
  let hash = 0;
  for (const ch of id) hash = (hash * 31 + ch.charCodeAt(0)) % 997;
  return hash;
}

/** `marker-form-mismatch` -> `MF`. The corpus's defect-id convention. */
function initials(id) {
  const parts = id.split(/[^a-z0-9]+/i).filter(Boolean);
  const letters =
    (parts[0]?.[0] ?? "X") + (parts[1]?.[0] ?? parts[0]?.[1] ?? "X");
  return letters.toUpperCase();
}

/**
 * The family a case belongs to, when quoin has no label file for it.
 *
 * Taken from the first of its own expected reasons the mapping recognises.
 * `none` for a control, and for a failure case whose expectations name nothing
 * any family claims — the honest answer, since a family this benchmark does not
 * govern is one it cannot score.
 */
function familyOf(meta, expect, mapping) {
  if (meta.kind !== "failure") return "none";
  for (const reason of expect.diagnostic_reasons ?? []) {
    const entry = Object.entries(mapping?.families ?? {}).find(
      ([, m]) => m.key === reason,
    );
    if (entry) return entry[0];
  }
  return "none";
}

/** Flatten the labels into the flat array scoring takes. */
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

/**
 * Refuse a binary that cannot say what it is, or lacks what the mapping reads.
 *
 * The `engine` provenance block arrived with agent-ix/quire-cli#68 precisely so
 * a saved number can name the build that produced it. A binary without one
 * predates it and cannot be scored against.
 */
function assertEngine(quire) {
  const probe = run(quire, ["--version"]);
  const line = (probe.stdout || "").trim();
  if (!line.includes("engine")) {
    throw new Error(
      `bench-tier1: ${quire} reports "${line}" — no engine version, so it ` +
        `predates agent-ix/quire-cli#68 and cannot say which engine it links. ` +
        `Refusing to score.`,
    );
  }
  console.error(`bench-tier1: engine ${line}`);
}

function main() {
  const update = process.argv.includes("--update");
  const asJson = process.argv.includes("--json");
  // NOT a PATH lookup. `quire` on PATH is whatever somebody installed —
  // measured at 0.29.0 here, which pins engine v0.42.0 and predates
  // `binding_census`. Scored with it, EVERY coverage family reported recall 0
  // and the run looked like a corpus regression rather than a stale binary.
  // That is agent-ix/quire-rs#265's defect, a third repository over.
  const quire = argOf("--quire") ?? process.env.QUIRE;
  if (!quire) {
    throw new Error(
      "bench-tier1: pass --quire <path> or set QUIRE. Deliberately not a PATH " +
        "lookup: scoring a benchmark with an unidentified binary is the defect " +
        "this benchmark exists to catch.",
    );
  }
  assertEngine(quire);

  const mapping = JSON.parse(readFileSync(MAPPING, "utf8"));
  const dictionary = loadMetrics(METRICS);

  const corpus = loadCorpus(mapping);
  let report;
  // Cases the corpus marks `pending` assert behaviour the engine does not
  // have yet. Scoring them counts a known-missing detector as a miss on every
  // run, which turns a deliberate red fixture into permanent noise in the
  // benchmark. They are excluded and NAMED, never silently dropped.
  const pending = corpus.corpora.filter((c) => c.pending);
  const labels = { corpora: corpus.corpora.filter((c) => !c.pending) };
  const flat = flattenLabels(labels);
  if (pending.length) {
    console.error(
      `bench-tier1: ${pending.length} case(s) excluded as pending a fix: ` +
        pending.map((c) => `${c.name} (${c.pending})`).join(", "),
    );
  }

  // FR-065: "The runner SHALL fail the run when a case declaring `pending`
  // passes." Excluding a pending case from SCORING is right — a known-missing
  // detector counted as a miss on every run turns a deliberate red into
  // permanent noise. Excluding it from CHECKING is not: without this, the fix
  // lands, the marker goes stale, and no score ever moves to say so.
  const stale = pending.filter((c) => {
    const want = c.defects.map((d) => d.expect_reason).filter(Boolean);
    if (!want.length) return false;
    const found = findingsFor(quire, c.input, c.module, mapping);
    return want.every((r) => found.some((f) => f.reason === r));
  });
  if (stale.length) {
    throw new Error(
      `bench-tier1: ${stale.length} pending case(s) now PASS — ` +
        stale.map((c) => `${c.name} (${c.pending})`).join(", ") +
        `. The fix appears to have landed; remove \`pending:\` from case.yaml ` +
        `so the case is scored.`,
    );
  }

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
    found.push(...findingsFor(quire, corpus.input, corpus.module, mapping));
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

  // The shape a family is scored under is declared in the mapping, beside the
  // key it reads — not inferred here, so a reader of the table can see why a
  // family reports no precision without tracing the scorer.
  const shapes = Object.fromEntries(
    Object.entries(mapping.families).map(([family, m]) => [
      family,
      m.shape ?? "defect",
    ]),
  );
  const score = scoreFindings(scoredFindings, flat, shapes);
  report = {
    families: score.families,
    excluded: score.excluded,
    collateral: score.collateral,
    positional: score.positional,
    finding_localisation_rate: localisationRate(score),
    actionability: scoreActionability(scoredFindings),
    corpora: labels.corpora.length,
    // In the REPORT, not only on stderr: a consumer piping stdout would
    // otherwise read a corpus count with nothing recording that a case exists
    // and was deliberately dropped.
    pending: pending.map((c) => ({ case: c.name, ticket: c.pending })),
    findings: scoredFindings.length,
  };

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
