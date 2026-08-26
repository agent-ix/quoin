#!/usr/bin/env node
// Tier-1 scores the validated qa-corpus inventory with a real Quire engine.
//
//   node scripts/bench-tier1.mjs                  # score and diff
//   node scripts/bench-tier1.mjs --update         # deliberate re-baseline
//   node scripts/bench-tier1.mjs --json           # the score, machine-readable
//   node scripts/bench-tier1.mjs --modules <dir>  # vary the declaration

import { createHash } from "node:crypto";
import {
  existsSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { crossCheckFamilies, loadMetrics } from "../evals/lib/dictionary.mjs";
import {
  scoreActionability,
  scoreCost,
  scoreFindings,
} from "../evals/lib/quality.mjs";
import {
  createMeasurementRecord,
  persistMeasurement as persistMeasurementCollection,
} from "./lib/tier1-measurement.mjs";
import { comparability, compare, ratchet } from "./lib/tier1-comparison.mjs";
import {
  adjudicationOf,
  byLanguage,
  flattenLabels,
  localisationRate,
  silentZeros,
} from "./lib/tier1-scoring.mjs";
import { createTier1Executor } from "./lib/tier1-execution.mjs";
import { renderTier1 } from "./lib/tier1-render.mjs";
import {
  loadCorpusData,
  standingAdjudications,
  validateCanonicalInventory,
} from "./lib/tier1-corpus.mjs";

export { comparability, compare, ratchet } from "./lib/tier1-comparison.mjs";
export {
  adjudicationOf,
  byLanguage,
  flattenLabels,
  localisationRate,
  silentZeros,
} from "./lib/tier1-scoring.mjs";
export { validateCanonicalInventory } from "./lib/tier1-corpus.mjs";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const MAPPING = join(ROOT, "bench", "tier1-mapping.json");
const METRICS = join(ROOT, "bench", "metrics.json");
const BASELINE = join(ROOT, "bench", "tier1-baseline.json");
const execution = createTier1Executor();

/** An absent engine metric is reported as not measured, never zero. */
const SECTION_HIT_RATE = "minting.section_hit_rate";

/** Load scorer inputs from qa-corpus's canonical validated inventory. */
export function loadCorpus(
  mapping = null,
  root = join(ROOT, "corpus"),
  modulesRoot = null,
  inventory = null,
) {
  mapping ??= existsSync(MAPPING)
    ? JSON.parse(readFileSync(MAPPING, "utf8"))
    : { families: {} };
  return loadCorpusData({
    mapping,
    root,
    modulesRoot,
    inventory: inventory ?? canonicalCorpusInventory(root),
  });
}

/** The validated case inventory emitted by qa-corpus's authoritative reader. */
export function canonicalCorpusInventory(root = join(ROOT, "corpus")) {
  const result = execution.execute("python3", [
    join(root, "bounds.py"),
    "--json",
  ]);
  if (!result.ok) {
    throw new Error(
      `bench-tier1: qa-corpus inventory failed: ${result.stderr}`,
    );
  }
  return validateCanonicalInventory(JSON.parse(result.stdout));
}

/** Every file under `dir`, as paths relative to it, in a stable order. */
function walkFiles(dir, base = dir) {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    if (name === ".git" || name === "__pycache__") continue;
    const full = join(dir, name);
    if (statSync(full).isDirectory()) out.push(...walkFiles(full, base));
    else out.push(relative(base, full).split(sep).join("/"));
  }
  return out;
}

/** A content digest over a declaration tree: path and bytes, nothing else. */
function digestOf(dir) {
  const hash = createHash("sha256");
  for (const rel of walkFiles(dir)) {
    hash.update(rel);
    hash.update("\0");
    hash.update(
      createHash("sha256")
        .update(readFileSync(join(dir, rel)))
        .digest(),
    );
    hash.update("\n");
  }
  return `sha256:${hash.digest("hex")}`;
}

/** Read upstream SHAs from vendored declarations; reject partial provenance. */
function vendoredSources(modulesRoot) {
  const out = {};
  let files = 0;
  const visit = (dir, depth) => {
    if (depth > 2 || !existsSync(dir)) return;
    for (const name of readdirSync(dir).sort()) {
      const full = join(dir, name);
      if (name === "VENDORED.md") {
        files += 1;
        let rows = 0;
        for (const line of readFileSync(full, "utf8").split("\n")) {
          const cells = line
            .split("|")
            .slice(1, -1)
            .map((c) => c.trim().replace(/`/g, ""));
          if (cells.length !== 3) continue;
          // A data row names a module that exists beside the provenance file.
          if (!existsSync(join(dir, cells[0]))) continue;
          // The SHA may be followed by an annotation; only its first token is data.
          const sha = cells[2].split(/\s+/)[0];
          if (!/^[0-9a-f]{7,40}$/.test(sha)) {
            throw new Error(
              `bench-tier1: ${full} records module \`${cells[0]}\` with ` +
                `\`${cells[2]}\` where a SHA belongs. Refusing to score a ` +
                `declaration one of whose modules has no upstream commit to ` +
                `join to.`,
            );
          }
          out[cells[0]] = sha;
          rows += 1;
        }
        if (!rows) {
          throw new Error(
            `bench-tier1: ${full} records no \`| module | path | sha |\` row ` +
              `this runner can read, so the declaration's upstream SHA cannot ` +
              `be recorded. Refusing to score a declaration whose provenance ` +
              `file is present and unreadable.`,
          );
        }
      } else if (statSync(full).isDirectory()) visit(full, depth + 1);
    }
  };
  visit(modulesRoot, 0);
  return files ? out : null;
}

/** Record declaration content and upstream identity as benchmark inputs. */
export function declarationProvenance(modulesRoot, bound = []) {
  const inRepo = !relative(ROOT, modulesRoot).startsWith("..");
  const paths = {};
  for (const id of [...new Set(bound)].sort()) {
    paths[id] = digestOf(join(modulesRoot, id));
  }
  return {
    root: inRepo
      ? relative(ROOT, modulesRoot).split(sep).join("/")
      : modulesRoot,
    digest: digestOf(modulesRoot),
    modules: paths,
    sources: vendoredSources(modulesRoot),
  };
}

function scorerDigest() {
  const hash = createHash("sha256");
  for (const path of [
    join(ROOT, "scripts"),
    join(ROOT, "evals", "lib"),
    join(ROOT, "bench"),
  ]) {
    hash.update(digestOf(path));
  }
  return `sha256:${hash.digest("hex")}`;
}

/** Build a plan-governed collection for one Tier-1 invocation. */
export function measurementRecord(report, at) {
  return createMeasurementRecord(report, at, {
    root: ROOT,
    metricsPath: METRICS,
    sectionHitRate: SECTION_HIT_RATE,
    execute: execution.execute,
    scorerDigest,
  });
}

function persistMeasurement(collection) {
  return persistMeasurementCollection(collection, ROOT);
}

/** The corpus revision this run read, or `null` when unavailable. */
function corpusRevision(root = join(ROOT, "corpus")) {
  const probe = execution.execute("git", ["-C", root, "rev-parse", "HEAD"]);
  return probe.ok ? probe.stdout.trim() : null;
}

function main() {
  const update = process.argv.includes("--update");
  const asJson = process.argv.includes("--json");
  // Require an explicit binary so engine identity is reviewable.
  const quire = argOf("--quire") ?? process.env.QUIRE;
  if (!quire) {
    throw new Error(
      "bench-tier1: pass --quire <path> or set QUIRE. Deliberately not a PATH " +
        "lookup: scoring a benchmark with an unidentified binary is the defect " +
        "this benchmark exists to catch.",
    );
  }
  const engine = execution.assertEngine(quire);
  console.error(`bench-tier1: engine ${engine}`);
  // An absent declaration override selects the corpus's vendored modules.
  const modules = argOf("--modules") ?? process.env.MODULES ?? null;

  const mapping = JSON.parse(readFileSync(MAPPING, "utf8"));
  const dictionary = loadMetrics(METRICS);

  const loaded = loadCorpus(
    mapping,
    join(ROOT, "corpus"),
    modules ? resolve(modules) : null,
  );
  let report;
  // Pending cases are excluded from scores but remain checked for expiry.
  const pending = loaded.corpora.filter((c) => c.pending);
  const labels = { corpora: loaded.corpora.filter((c) => !c.pending) };
  const flat = flattenLabels(labels);
  if (pending.length) {
    console.error(
      `bench-tier1: ${pending.length} case(s) excluded as pending a fix: ` +
        pending.map((c) => `${c.name} (${c.pending})`).join(", "),
    );
  }

  // A pending marker must carry an expiry signal in expect-pending.yaml.
  const deferred = [];
  for (const c of pending) {
    if (c.pendingReasons.length) continue;
    if (!c.hasPendingBlock) {
      throw new Error(
        `bench-tier1: pending case ${c.name} (${c.pending}) has no ` +
          `\`expect-pending.yaml\`, so no reader can ever say the fix landed ` +
          `and the marker would stand forever. State what the ticket makes ` +
          `true in the forward block — not in \`expect.yaml\`, where it ` +
          `would be a false claim about today — or drop the \`pending:\` ` +
          `marker (agent-ix/quoin#242).`,
      );
    }
    // Payload-only expiry stays with qa-corpus's authoritative graders.
    deferred.push(c);
  }
  if (deferred.length) {
    console.error(
      `bench-tier1: ${deferred.length} pending case(s) expire on a payload ` +
        `change, not a diagnostic, so their staleness is checked by the ` +
        `corpus's own graders and not here: ` +
        deferred.map((c) => `${c.name} (${c.pending})`).join(", ") +
        `. Run \`make ci\` in agent-ix/qa-corpus for those.`,
    );
  }
  const stale = pending.filter((c) => {
    if (!c.pendingReasons.length) return false;
    const emitted = execution.rawReasons(quire, c.input, c.module);
    return c.pendingReasons.every((r) => emitted.has(r));
  });
  if (stale.length) {
    throw new Error(
      `bench-tier1: ${stale.length} pending case(s) now PASS — ` +
        stale.map((c) => `${c.name} (${c.pending})`).join(", ") +
        `. The fix appears to have landed; remove \`pending:\` from case.yaml ` +
        `so the case is scored.`,
    );
  }

  // Refuse declared-but-unseeded and seeded-but-undeclared families.
  crossCheckFamilies(
    dictionary.families,
    labels.corpora.map((c) => c.family),
    { path: "bench/metrics.json" },
  );

  const found = [];
  // Aggregate section hits by document, not by mean case rate.
  const sectionHit = { matched: 0, examined: 0, cases: 0 };
  const payloads = [];
  for (const corpus of labels.corpora) {
    const { findings, metrics, diagnostics } = execution.findingsFor(
      quire,
      corpus.input,
      corpus.module,
      mapping,
    );
    payloads.push({ name: corpus.name, metrics, diagnostics });
    found.push(
      ...findings.map((f) => ({
        ...f,
        corpus: corpus.name,
        language: corpus.language,
      })),
    );
    const hit = metrics.find((m) => m.name === SECTION_HIT_RATE);
    if (hit && hit.state === "measured") {
      sectionHit.cases += 1;
      sectionHit.matched += Number(hit.matched ?? 0);
      sectionHit.examined += Number(hit.examined ?? 0);
    }
  }

  // A metric-sourced finding counts only at the value its label expects.
  const expectedValues = new Map(
    flat
      .filter((l) => l.expect_metric !== undefined)
      .map((l) => [l.expect_metric, Number(l.expect_value)]),
  );
  const scoredFindings = found.filter(
    (f) => f.metric === undefined || expectedValues.get(f.metric) === f.value,
  );

  // Family scoring shape is declared in the mapping.
  const shapes = Object.fromEntries(
    Object.entries(mapping.families).map(([family, m]) => [
      family,
      m.shape ?? "defect",
    ]),
  );
  // Advisory precision uses ruled cases; other firings remain unadjudicated.
  const adjudication = adjudicationOf(
    labels.corpora,
    mapping,
    standingAdjudications(join(ROOT, "corpus")),
  );
  const score = scoreFindings(scoredFindings, flat, shapes, adjudication);
  const silentZeroes = silentZeros(payloads);
  const bounds = loaded.bounds;
  report = {
    // No timestamp: identical inputs produce byte-identical reports.
    provenance: {
      engine,
      corpus: corpusRevision(),
      declaration: declarationProvenance(
        loaded.modulesRoot,
        labels.corpora.map((c) =>
          relative(loaded.modulesRoot, c.module).split(sep).join("/"),
        ),
      ),
    },
    bounds,
    families: score.families,
    excluded: score.excluded,
    collateral: score.collateral,
    positional: score.positional,
    finding_localisation_rate: localisationRate(score),
    // Population property: report it, but do not ratchet it.
    "minting.section_hit_rate": sectionHit.examined
      ? {
          rate: Number((sectionHit.matched / sectionHit.examined).toFixed(3)),
          matched: sectionHit.matched,
          examined: sectionHit.examined,
          cases_reporting: sectionHit.cases,
        }
      : null,
    actionability: scoreActionability(scoredFindings),
    // Tier 1 calls no model, so token cost is not measured; tool calls are.
    cost_per_confirmed_insight: scoreCost(
      { toolCalls: execution.toolCalls() },
      score.families.reduce((n, f) => n + f.truePositives, 0),
    ),
    // Exact-zero gate; instances make failures actionable.
    "sentinel.silent_zero": {
      count: silentZeroes.violations.length,
      instances: silentZeroes.violations,
      // Empty populations are visible but are not silent-zero violations.
      unread_population: silentZeroes.unread,
    },
    corpora: labels.corpora.length,
    by_language: byLanguage(
      labels.corpora,
      scoredFindings,
      flat,
      shapes,
      adjudication,
    ),
    // Preserve excluded pending cases in machine-readable output.
    pending: pending.map((c) => ({ case: c.name, ticket: c.pending })),
    findings: scoredFindings.length,
  };

  const previous = existsSync(BASELINE)
    ? JSON.parse(readFileSync(BASELINE, "utf8"))
    : null;
  const verdicts = ratchet(report, previous, dictionary);
  // Unknown comparability inputs are never assumed to match silently.
  const { unknown } = comparability(report, previous);
  if (unknown.length) {
    console.error(
      `bench-tier1: the baseline records no ${unknown.join(", ")}; this ` +
        `comparison ASSUMES those inputs did not move, and cannot check it.`,
    );
  }

  if (asJson) {
    console.log(JSON.stringify({ ...report, verdicts }, null, 2));
  } else {
    console.log(renderTier1(report, verdicts));
  }

  if (update) {
    // Persist the source-of-truth collection before its derived baseline.
    const recordPath = persistMeasurement(
      measurementRecord(report, new Date().toISOString()),
    );
    writeFileSync(BASELINE, JSON.stringify(report, null, 2) + "\n");
    console.error(
      `bench-tier1: collection written to ${recordPath}; derived baseline rewritten ` +
        `at ${BASELINE}`,
    );
    return 0;
  }
  // Incomparable inputs require a deliberate re-baseline.
  return verdicts.some(
    (v) => v.verdict === "regressed" || v.verdict === "incomparable",
  )
    ? 1
    : 0;
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
