/** Filesystem boundary for the pure governed graph portfolio projection. */

import {
  analyzeChangeImpact,
  analyzeChurn,
  analyzeFanOut,
  loadGraphAnalysisInput,
} from "../graph-analysis/index.js";
import { adaptQuireAssurance } from "./graph-adapters.js";
import {
  buildGovernedGraphPortfolioFrom,
  parseGraphPortfolioMappings,
  type GovernedGraphPortfolioReport,
  type GraphPortfolioMappingOptions,
  type InjectedStructuralGraph,
} from "./graph-portfolio.js";
import { buildPortfolioReport } from "./portfolio.js";
import { readMeasurementCollectionResults } from "./store.js";

export function buildGovernedGraphPortfolio(
  locations: readonly string[],
  options: GraphPortfolioMappingOptions = {},
): GovernedGraphPortfolioReport {
  // Mapping validation is deliberately first: conflicts cannot trigger reads.
  const mappings = parseGraphPortfolioMappings(locations, options);
  const portfolio = buildPortfolioReport([...locations]);
  const byRoot = new Map(
    portfolio.repositories.map((repository) => [repository.root, repository]),
  );
  const report = buildGovernedGraphPortfolioFrom(
    mappings.map((mapping) => {
      const base = byRoot.get(mapping.root);
      if (!base)
        throw new Error(
          `portfolio omitted resolved repository ${mapping.root}`,
        );
      let collections;
      try {
        collections = readMeasurementCollectionResults(mapping.root);
      } catch (cause) {
        collections = [
          {
            path: mapping.root,
            availability: "unreadable" as const,
            error: cause instanceof Error ? cause.message : String(cause),
          },
        ];
      }
      return {
        portfolio: base,
        plans: base.plans,
        collections,
        graph: loadStructuralGraph(mapping),
      };
    }),
  );
  return {
    ...report,
    newestCollectionTimestamp: portfolio.newestCollectionTimestamp,
    staleAfterDays: portfolio.staleAfterDays,
  };
}

function loadStructuralGraph(
  mapping: ReturnType<typeof parseGraphPortfolioMappings>[number],
): InjectedStructuralGraph {
  if (mapping.status !== "ready")
    return { availability: mapping.status, reason: mapping.reason };
  const loaded = loadGraphAnalysisInput({
    repo: mapping.root,
    exportPath: mapping.exportPath,
    premisesPath: mapping.premisesPath,
    auditPath: mapping.auditPath,
  });
  if (!loaded.ok) {
    return {
      availability:
        loaded.error.kind === "graph-input-invalid"
          ? "incompatible"
          : /ENOENT|no such file/i.test(loaded.error.message)
            ? "missing"
            : "unreadable",
      path: pathFor(mapping, loaded.error.input),
      reason: loaded.error.message,
    };
  }
  // FR-066's adapter performs the same closed producer-contract handoff used
  // by other consumers; FR-062 remains the sole owner of graph semantics.
  adaptQuireAssurance(loaded.value.assurance, {
    format: loaded.value.premises.format,
    formatVersion: loaded.value.premises.format_version,
    source: loaded.value.assurance.source,
    modules: loaded.value.premises.modules,
  });
  return {
    availability: "available",
    path: mapping.exportPath,
    premises: loaded.value.premises,
    fanOut: analyzeFanOut(loaded.value),
    churn: analyzeChurn(loaded.value),
    ...(mapping.changed.length > 0
      ? { changeImpact: [analyzeChangeImpact(loaded.value, mapping.changed)] }
      : {}),
  };
}

function pathFor(
  mapping: Extract<
    ReturnType<typeof parseGraphPortfolioMappings>[number],
    { status: "ready" }
  >,
  input: "export" | "premises" | "audit",
): string {
  if (input === "export") return mapping.exportPath;
  if (input === "premises") return mapping.premisesPath;
  return mapping.auditPath;
}
