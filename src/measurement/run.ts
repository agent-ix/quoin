/**
 * The measurement run (FR-093).
 *
 * Orchestrates the pieces and writes the artifacts. Nothing here decides
 * conformance, classifies a finding, or computes a rate: those live in the
 * modules that own them, and this composes them so the run is one path rather
 * than a set of steps someone remembers to perform in order.
 *
 * The run exits zero whatever it finds. A non-zero exit is reserved for the
 * measurement failing to run, because "the corpus has failures" and "the
 * measurement did not happen" must not be the same signal to a caller.
 */

import { createHash } from "node:crypto";
import { mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import {
  assertOutputOutsideCorpus,
  enumerateCorpus,
  type CorpusRecord,
} from "./enumerate.js";
import {
  resolveModuleSet,
  toolchainRecord,
  type ModuleFinding,
  type ModuleRecord,
} from "./modules.js";
import {
  assertPartition,
  assignDocuments,
  buildVocabulary,
  type DocumentAssignment,
} from "./document-state.js";
import { materializeModules, runBatch, type Evaluation } from "./engine-run.js";
import { propertiesCensus, conformingShare } from "./properties-census.js";
import { LEDGER, coverage } from "./tool-defects.js";
import { identity, partition, type Finding } from "./partition.js";
import { populationId, rate } from "./rates.js";
import { ruleFor } from "./classify-rules.js";
import { runEnvironment, stableDigest } from "./fixture-corpus.js";

export interface RunOptions {
  readonly workspaceRoot: string;
  readonly exclusionVocabulary: readonly string[];
  readonly corpusId: string;
  readonly outputDir: string;
  readonly modules: readonly { name: string; repositoryPath: string; ref: string }[];
  /** Skip the engine pass; the structural rate is then absent, never zero. */
  readonly skipEngine?: boolean;
}

export interface RunManifest {
  readonly corpusId: string;
  readonly populationId: string;
  readonly startedAt: string;
  readonly finishedAt: string;
  readonly durationMs: number;
  readonly environment: ReturnType<typeof runEnvironment>;
  readonly artifacts: Readonly<Record<string, string>>;
}

function documentsUnder(root: string): string[] {
  const out: string[] = [];
  const walk = (dir: string): void => {
    let entries: string[];
    try {
      entries = readdirSync(dir).sort();
    } catch {
      return;
    }
    if (entries.includes(".git") && dir !== root) return;
    for (const entry of entries) {
      const full = join(dir, entry);
      let info: ReturnType<typeof statSync>;
      try {
        info = statSync(full);
      } catch {
        continue;
      }
      if (info.isDirectory()) walk(full);
      else if (entry.endsWith(".md")) out.push(full);
    }
  };
  walk(root);
  return out;
}

function write(dir: string, name: string, value: unknown): string {
  const text = `${JSON.stringify(value, null, "\t")}\n`;
  writeFileSync(join(dir, name), text);
  return `sha256:${createHash("sha256").update(text).digest("hex")}`;
}

export interface RunResult {
  readonly corpus: CorpusRecord;
  readonly modules: readonly ModuleRecord[];
  readonly moduleFindings: readonly ModuleFinding[];
  readonly assignments: readonly DocumentAssignment[];
  readonly evaluations: readonly Evaluation[];
  readonly manifest: RunManifest;
}

export function runMeasurement(options: RunOptions): RunResult {
  const startedAt = new Date().toISOString();
  const started = Date.now();

  // Refuses before a file is read: a run that writes into its own population
  // has changed what it measures, and no later assertion undoes that.
  assertOutputOutsideCorpus(
    options.outputDir,
    options.workspaceRoot,
    options.exclusionVocabulary,
  );
  mkdirSync(options.outputDir, { recursive: true });

  const corpus = enumerateCorpus({
    workspaceRoot: options.workspaceRoot,
    exclusionVocabulary: options.exclusionVocabulary,
    corpusId: options.corpusId,
  });

  // Throws if any required module is unresolvable: a rate computed without one
  // is a rate for a population nobody declared.
  const { modules, findings: moduleFindings } = resolveModuleSet(options.modules);
  const vocabulary = buildVocabulary(modules);

  const documents = corpus.repositories.flatMap((r) =>
    documentsUnder(join(r.path, "spec")),
  );
  const { assignments, findings: contested } = assignDocuments(
    documents,
    vocabulary,
  );
  const states = assertPartition(assignments, documents.length);

  let evaluations: Evaluation[] = [];
  if (!options.skipEngine) {
    const modulesPath = materializeModules(modules);
    for (const repository of corpus.repositories) {
      const mine = assignments
        .filter(
          (a) => a.state === "measured" && a.path.startsWith(`${repository.path}/`),
        )
        .map((a) => a.path.slice(repository.path.length + 1));
      evaluations = evaluations.concat(
        runBatch({ scope: repository.path, documents: mine, modulesPath }),
      );
    }
  }

  const census = propertiesCensus(documents);
  const share = conformingShare(census);

  const pop = populationId({
    corpusId: options.corpusId,
    repositoryCommits: corpus.repositories.map((r) => r.commit ?? "none"),
    moduleCommits: modules.map((m) => m.commit),
    engineRevision: toolchainRecord(modules).engineVersion,
  });

  const findings: Finding[] = evaluations
    .filter((e) => e.outcome === "fail")
    .flatMap((e) =>
      e.diagnostics
        .filter((d) => d.severity === "error")
        .map((d) => ({
          repository:
            corpus.repositories.find((r) => e.path.startsWith(r.path))?.path ?? "",
          path: e.path,
          check: d.code ?? "unknown",
          code: d.code,
          message: d.message,
        })),
    );
  // Authored rules, each carrying the sample that settled it. A finding whose
  // reason no rule covers stays `unknown` rather than being absorbed.
  const authored = new Map(
    findings.flatMap((f) => {
      const rule = ruleFor(f.message);
      if (!rule) return [];
      return [
        [
          identity(f),
          {
            classification: rule.classification,
            disposition: rule.disposition(f.repository || "unknown-repository"),
          },
        ] as const,
      ];
    }),
  );
  const classified = partition({ findings, ledger: LEDGER, authored });

  const passed = evaluations.filter((e) => e.outcome === "pass").length;
  const couldNotRun = evaluations.filter(
    (e) => e.outcome === "could-not-run",
  ).length;

  const structural = rate({
    id: "structural",
    numerator: passed,
    denominator: evaluations.length - couldNotRun,
    unit: "documents",
    populationId: pop,
    methodId: "engine-structural-v1",
    excluded: { "could-not-run": couldNotRun, ...states },
  });

  const formCensus = rate({
    id: "properties-form",
    numerator: share.conforming,
    denominator: share.applicable,
    unit: "documents",
    populationId: pop,
    methodId: "properties-form-census-v1",
    excluded: { "not-applicable": census.notApplicable },
  });

  const artifacts: Record<string, string> = {};
  artifacts["corpus.json"] = write(options.outputDir, "corpus.json", corpus);
  artifacts["modules.json"] = write(options.outputDir, "modules.json", {
    modules,
    findings: moduleFindings,
    toolchain: toolchainRecord(modules),
  });
  artifacts["states.json"] = write(options.outputDir, "states.json", {
    states,
    contested,
  });
  artifacts["census.json"] = write(options.outputDir, "census.json", {
    unit: census.unit,
    total: census.total,
    byForm: census.byForm,
    advisoryFindings: census.advisoryFindings,
    notApplicable: census.notApplicable,
    fieldLevelCitation: census.fieldLevelCitation,
  });
  artifacts["rates.json"] = write(options.outputDir, "rates.json", {
    structural,
    formCensus,
    unstableRepositories: corpus.repositories.filter((r) => !r.stable).length,
    dirtyRepositories: corpus.repositories.filter((r) => !r.clean).length,
  });
  artifacts["findings.json"] = write(options.outputDir, "findings.json", {
    tally: classified.tally,
    unknown: classified.unknown,
    undispositioned: classified.undispositioned,
    unmatchedLedgerEntries: classified.unmatchedLedgerEntries,
    ledgerCoverage: coverage(LEDGER, documents),
  });

  const finishedAt = new Date().toISOString();
  const manifest: RunManifest = {
    corpusId: options.corpusId,
    populationId: pop,
    startedAt,
    finishedAt,
    durationMs: Date.now() - started,
    environment: runEnvironment(),
    artifacts,
  };
  write(options.outputDir, "run-manifest.json", manifest);

  return {
    corpus,
    modules,
    moduleFindings,
    assignments,
    evaluations,
    manifest,
  };
}

/** The digest of a run's artifacts, ignoring only the manifest's timestamps. */
export function runDigest(manifest: RunManifest): string {
  return stableDigest({ ...manifest });
}
