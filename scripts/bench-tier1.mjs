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
import { format as prettierFormat } from "prettier";

import { crossCheckFamilies, loadMetrics } from "../evals/lib/dictionary.mjs";
import { normalizeFinding } from "../evals/lib/finding-envelope.mjs";
import {
  scoreActionability,
  scoreActionabilityV2,
  scoreCost,
  scoreFindings,
  scoreGroundingQuality,
  scoreSpanGrounding,
  scoreSpanGroundingV2,
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
import { loadAdvisoryAdjudication } from "./lib/advisory-adjudication.mjs";
import { createTier1Executor } from "./lib/tier1-execution.mjs";
import { evaluateGuidanceProof } from "./lib/guidance-proof.mjs";
import { renderTier1 } from "./lib/tier1-render.mjs";
import {
  detectionRecall,
  localityMissInventory,
  recallGateFails,
  recallVerdicts,
} from "./lib/tier1-recall.mjs";
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
const CORPUS_METRICS = join(ROOT, "corpus", "config", "metrics.json");
const BASELINE = join(ROOT, "bench", "tier1-baseline.json");
const RECALL_BASELINE = join(ROOT, "corpus", "baselines", "quoin.json");
const GROUNDING_LABELS = join(ROOT, "bench", "span-grounding-labels.json");
const GROUNDING_V2_LABELS = join(
  ROOT,
  "bench",
  "span-grounding-v2-labels.json",
);
const GROUNDING_FIXTURE = join(ROOT, "bench", "fixtures", "span-grounding");
const ADVISORY_ADJUDICATION = join(
  ROOT,
  "bench",
  "advisory-adjudication-v1.json",
);
const GUIDANCE_CONTRACT = join(
  ROOT,
  "bench",
  "guidance-evaluator-contract-v1.json",
);
const GUIDANCE_REVIEW = join(
  ROOT,
  "bench",
  "guidance-independent-review-v1.json",
);
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

function digestFile(path) {
  return `sha256:${createHash("sha256").update(readFileSync(path)).digest("hex")}`;
}

/** Fail before corpus execution when a canonical stack attestation moved. */
export function validateVerificationAttestation(value, quire, tool, corpus) {
  if (value?.schemaVersion !== "verification-stack-attestation-v1") {
    throw new Error(
      "bench-tier1: canonical runs require verification-stack-attestation-v1",
    );
  }
  for (const key of ["lockDigest", "executableDigest"]) {
    if (!/^sha256:[0-9a-f]{64}$/.test(value[key] ?? "")) {
      throw new Error(
        `bench-tier1: attestation ${key} is not a full sha256 digest`,
      );
    }
  }
  for (const name of ["node", "rust", "python"]) {
    if (
      typeof value.toolchains?.[name] !== "string" ||
      !value.toolchains[name]
    ) {
      throw new Error(
        "bench-tier1: attestation must pin node, rust, and python toolchains",
      );
    }
  }
  const observedExecutable = digestFile(quire);
  if (observedExecutable !== value.executableDigest) {
    throw new Error(
      `bench-tier1: executable moved after attestation: expected ${value.executableDigest}, observed ${observedExecutable}`,
    );
  }
  const expectedSources = {
    "quire-cli": tool.cli.sourceRevision,
    quire: tool.engine.sourceRevision,
    "qa-corpus": corpus,
  };
  for (const [name, revision] of Object.entries(expectedSources)) {
    const source = value.sources?.[name];
    if (source?.revision !== revision || source?.sourceState !== "clean") {
      throw new Error(
        `bench-tier1: attested ${name} does not match the selected clean source ${revision}`,
      );
    }
  }
  const attestedCapabilities = new Set(value.capabilities ?? []);
  for (const capability of tool.capabilities ?? []) {
    if (!attestedCapabilities.has(capability)) {
      throw new Error(
        `bench-tier1: attestation omits selected capability ${capability}`,
      );
    }
  }
  return structuredClone(value);
}

/** Build a plan-governed collection for one Tier-1 invocation. */
export function measurementRecord(report, at) {
  return createMeasurementRecord(report, at, {
    root: ROOT,
    metricsPath: METRICS,
    corpusMetricsPath: CORPUS_METRICS,
    corpusPlanOverrides: {
      "detection.recall": "spec/assurance/MP-210-detection-recall.md",
      "bounds.gap_count": "spec/assurance/MP-211-corpus-gap-count.md",
    },
    sectionHitRate: SECTION_HIT_RATE,
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

/** Content identity of scored inputs, excluding ratchets stored beside them. */
function corpusInputDigest(root = join(ROOT, "corpus")) {
  const hash = createHash("sha256");
  for (const rel of walkFiles(root).filter(
    (path) => path !== "baselines/README.md" && !path.startsWith("baselines/"),
  )) {
    hash.update(rel);
    hash.update("\0");
    hash.update(readFileSync(join(root, rel)));
    hash.update("\n");
  }
  return `sha256:${hash.digest("hex")}`;
}

async function main() {
  const update = process.argv.includes("--update");
  const asJson = process.argv.includes("--json");
  const experimental = process.argv.includes("--experimental");
  const guidanceCandidateOut = argOf("--guidance-candidate-out");
  const guidanceCandidateOnly = process.argv.includes(
    "--guidance-candidate-only",
  );
  const recallBaselineOut = argOf("--recall-baseline-out");
  if (experimental && update) {
    throw new Error(
      "bench-tier1: a noncanonical experimental run cannot update governed evidence",
    );
  }
  if (guidanceCandidateOnly && (!experimental || !guidanceCandidateOut)) {
    throw new Error(
      "bench-tier1: --guidance-candidate-only requires --experimental and --guidance-candidate-out <path>",
    );
  }
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
  const quoinArg = argOf("--quoin");
  if (!experimental && !quoinArg) {
    throw new Error(
      "bench-tier1: canonical runs require --quoin <isolated executable>",
    );
  }
  const quoin = resolve(quoinArg ?? join(ROOT, "bin", "quoin.js"));
  let verificationStack = null;
  if (!experimental) {
    const attestationPath = argOf("--attestation");
    if (!attestationPath) {
      throw new Error(
        "bench-tier1: canonical runs require --attestation <path>; use --experimental for an explicitly noncanonical run that cannot compare or update baselines",
      );
    }
    verificationStack = validateVerificationAttestation(
      JSON.parse(readFileSync(resolve(attestationPath), "utf8")),
      resolve(quire),
      execution.engineProvenance(),
      corpusRevision(),
    );
  } else {
    console.error(
      "bench-tier1: NONCANONICAL experimental run; comparison and baseline updates are disabled",
    );
  }
  const spanBreadthPath = argOf("--span-breadth");
  if (!experimental && !spanBreadthPath) {
    throw new Error(
      "bench-tier1: canonical runs require --span-breadth <result>",
    );
  }
  const spanBreadth = spanBreadthPath
    ? JSON.parse(readFileSync(resolve(spanBreadthPath), "utf8"))
    : null;
  // An absent declaration override selects the corpus's vendored modules.
  const modules = argOf("--modules") ?? process.env.MODULES ?? null;

  const mapping = JSON.parse(readFileSync(MAPPING, "utf8"));
  const dictionary = loadMetrics(METRICS);
  loadMetrics(CORPUS_METRICS);

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
  const propertyPayloads = [];
  for (const [index, corpus] of labels.corpora.entries()) {
    console.error(
      `bench-tier1: case ${index + 1}/${labels.corpora.length} ${corpus.name}`,
    );
    const { findings, metrics, diagnostics, untrackedSymbols } =
      execution.findingsFor(quire, corpus.input, corpus.module, mapping, quoin);
    payloads.push({
      name: corpus.name,
      metrics,
      diagnostics,
      untrackedSymbols,
    });
    propertyPayloads.push({
      case: corpus.name,
      payload: execution.properties(quire, corpus.input, corpus.module),
    });
    found.push(
      ...findings.map((f) =>
        normalizeFinding(f, {
          sourceClass: f.sourceClass,
          producer: f.producer,
          channel: f.channel,
          family: f.family,
          corpus: corpus.name,
          language: corpus.language,
          declaration: f.declaration,
        }),
      ),
    );
    const hit = metrics.find((m) => m.name === SECTION_HIT_RATE);
    if (hit && hit.state === "measured") {
      sectionHit.cases += 1;
      sectionHit.matched += Number(hit.matched ?? 0);
      sectionHit.examined += Number(hit.examined ?? 0);
    }
  }
  const groundingLabels = JSON.parse(readFileSync(GROUNDING_LABELS, "utf8"));
  const groundingV2Labels = JSON.parse(
    readFileSync(GROUNDING_V2_LABELS, "utf8"),
  );
  const groundingPayload = execution.properties(
    quire,
    GROUNDING_FIXTURE,
    join(loaded.modulesRoot, "ecosystem"),
  );

  // A metric-sourced finding counts only at the value its label expects.
  const expectedValues = new Map(
    flat
      .filter((l) => l.expect_metric !== undefined)
      .map((l) => [l.expect_metric, Number(l.expect_value)]),
  );
  const scoredFindings = found.filter(
    (f) =>
      f.evaluation.metric === undefined ||
      expectedValues.get(f.evaluation.metric) === f.evaluation.value,
  );

  // Family scoring shape is declared in the mapping.
  const shapes = Object.fromEntries(
    Object.entries(mapping.families).map(([family, m]) => [
      family,
      m.shape ?? "defect",
    ]),
  );
  const retainedAdjudication = loadAdvisoryAdjudication(ADVISORY_ADJUDICATION);
  // Advisory precision uses ruled cases; other firings remain unadjudicated.
  const adjudication = adjudicationOf(
    labels.corpora,
    mapping,
    standingAdjudications(join(ROOT, "corpus")),
    retainedAdjudication,
  );
  const score = scoreFindings(scoredFindings, flat, shapes, adjudication);
  if (guidanceCandidateOut) {
    const candidate = {
      schemaVersion: "guidance-candidate-v1",
      corpusRevision: corpusRevision(),
      corpusInputDigest: corpusInputDigest(),
      producer: execution.engineProvenance(),
      findingRecords: scoredFindings,
    };
    writeFileSync(
      resolve(guidanceCandidateOut),
      await formatTier1Json(candidate, resolve(guidanceCandidateOut)),
    );
    console.error(
      `bench-tier1: guidance candidate written to ${resolve(guidanceCandidateOut)}`,
    );
    if (guidanceCandidateOnly) return 0;
  }
  const guidanceQuality = evaluateGuidanceProof(
    scoredFindings,
    JSON.parse(readFileSync(GUIDANCE_CONTRACT, "utf8")),
    JSON.parse(readFileSync(GUIDANCE_REVIEW, "utf8")),
  );
  const silentZeroes = silentZeros(payloads);
  const bounds = loaded.bounds;
  report = {
    // No timestamp: identical inputs produce byte-identical reports.
    provenance: {
      engine,
      tool: execution.engineProvenance(),
      verification_stack: verificationStack,
      corpus: corpusRevision(),
      corpus_input: corpusInputDigest(),
      declaration: declarationProvenance(
        loaded.modulesRoot,
        labels.corpora.map((c) =>
          relative(loaded.modulesRoot, c.module).split(sep).join("/"),
        ),
      ),
    },
    bounds,
    detection_recall: detectionRecall(
      labels.corpora,
      scoredFindings,
      bounds.gap_count,
      payloads,
    ),
    locality_miss_inventory: localityMissInventory(
      labels.corpora,
      scoredFindings,
      payloads,
    ),
    families: score.families,
    advisory_adjudication: {
      metric_version: retainedAdjudication.metricVersion,
      rubric_version: retainedAdjudication.rubricVersion,
      population: retainedAdjudication.population,
      dispositions: retainedAdjudication.counts,
    },
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
    actionability_v1: scoreActionability(scoredFindings),
    actionability_v2: scoreActionabilityV2(scoredFindings),
    guidance_quality: guidanceQuality,
    span_grounding: scoreSpanGrounding(propertyPayloads),
    span_grounding_v2: scoreSpanGroundingV2(
      propertyPayloads,
      groundingV2Labels,
    ),
    span_breadth: spanBreadth,
    property_payloads: propertyPayloads,
    grounding_quality: scoreGroundingQuality(groundingPayload, groundingLabels),
    grounding_quality_payload: groundingPayload,
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
    finding_records: scoredFindings,
  };

  const previous =
    !experimental && existsSync(BASELINE)
      ? JSON.parse(readFileSync(BASELINE, "utf8"))
      : null;
  const recallBaseline =
    !experimental && existsSync(RECALL_BASELINE)
      ? JSON.parse(readFileSync(RECALL_BASELINE, "utf8"))
      : null;
  if (!experimental && !update && recallBaseline === null) {
    throw new Error(
      `bench-tier1: ${RECALL_BASELINE} is missing; run the explicit ` +
        "bench update to create a measured recall ratchet, rather than " +
        "treating an absent baseline as zero",
    );
  }
  const recall = experimental
    ? []
    : recallVerdicts(report.detection_recall, recallBaseline?.rows ?? []);
  const verdicts = experimental
    ? []
    : [
        ...ratchet(report, previous, dictionary),
        ...recall.map((verdict) => ({
          metric: "detection.recall",
          ...verdict,
          observed: verdict.rate,
        })),
      ];
  // Unknown comparability inputs are never assumed to match silently.
  const { unknown } = experimental
    ? { unknown: [] }
    : comparability(report, previous);
  if (unknown.length) {
    console.error(
      `bench-tier1: the baseline records no ${unknown.join(", ")}; this ` +
        `comparison ASSUMES those inputs did not move, and cannot check it.`,
    );
  }
  if (!experimental) {
    if (
      report.span_breadth?.rate !== 1 ||
      report.span_breadth?.namedMisses?.length
    ) {
      throw new Error(
        "bench-tier1: broad span grounding is not 100% exact or safely refused",
      );
    }
    for (const [name, result] of Object.entries(report.guidance_quality).filter(
      ([name]) =>
        ["correctness", "repairSuccess", "diagnosticYield"].includes(name),
    )) {
      if (result.rate !== 1) {
        throw new Error(
          `bench-tier1: guidance ${name} must be 100%, observed ${result.numerator}/${result.denominator}`,
        );
      }
    }
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
    writeFileSync(BASELINE, await formatTier1Json(report, BASELINE));
    // The ratchet lives inside the corpus, so rewriting it solely because its
    // own commit changed would create an endless self-revision cycle. Only a
    // measured score or GAP movement rewrites it; corpus_input excludes this
    // directory for the same reason.
    const recallMoved =
      recallBaseline === null ||
      recallBaseline.gap_count !== report.bounds.gap_count ||
      JSON.stringify(recallBaseline.rows) !==
        JSON.stringify(report.detection_recall);
    if (recallMoved) {
      const output = recallBaselineOut
        ? resolve(recallBaselineOut)
        : RECALL_BASELINE;
      writeFileSync(
        output,
        await formatTier1Json(
          {
            definition_version: "detection-recall-v1",
            runner: "quoin",
            corpus_revision: report.provenance.corpus,
            gap_count: report.bounds.gap_count,
            rows: report.detection_recall,
          },
          output,
        ),
      );
      console.error(`bench-tier1: recall baseline written to ${output}`);
    }
    console.error(
      `bench-tier1: collection written to ${recordPath}; derived baseline rewritten ` +
        `at ${BASELINE}`,
    );
    return 0;
  }
  if (experimental) return 0;
  // A new or improved recall partition is useful only after the update
  // workflow retains it as the new floor. Regressions and incomparable runs
  // also refuse the gate.
  return recallGateFails(recall) ||
    verdicts.some(
      (v) => v.verdict === "regressed" || v.verdict === "incomparable",
    )
    ? 1
    : 0;
}

/** Emit generated JSON in the same form the repository gate enforces. */
export async function formatTier1Json(value, filepath) {
  return prettierFormat(JSON.stringify(value), { filepath });
}

function argOf(flag) {
  const i = process.argv.indexOf(flag);
  return i === -1 ? null : process.argv[i + 1];
}

if (
  resolve(process.argv[1] ?? "") === resolve(fileURLToPath(import.meta.url))
) {
  process.exit(await main());
}
