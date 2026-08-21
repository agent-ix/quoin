import type { RunEntry } from "../types.js";
import { cargoMutantsAdapter } from "./cargo-mutants.js";
import { junitAdapter } from "./junit.js";
import { parseAuditScript } from "./audit-script.js";
import { agentEvalAdapter } from "./agent-eval.js";
import { parseDependencyCruiser } from "./dependency-cruiser.js";
import { sbomAdapter } from "./sbom.js";
import { parseCargoAudit, parseSarif, type FindingResult } from "./sarif.js";
import {
  AdapterError,
  type AdapterResult,
  type EvidenceAdapter,
} from "./types.js";

/**
 * The normalized shape `--results` has always accepted, kept as an adapter so
 * the default path goes through the same seam as every other format.
 *
 * Named `entries` rather than `quoin` because that is what the payload calls
 * itself, and because it is the shape a consumer writes by hand when no adapter
 * fits their tool — the escape hatch that keeps the registry from being a
 * gate.
 */
export const entriesAdapter: EvidenceAdapter = {
  name: "entries",
  summary:
    'Normalized {"entries": [{symbol, outcome, traceIds?}]} — the default.',
  tools: [],
  parse(raw: string): AdapterResult {
    let parsed: { entries?: RunEntry[] };
    try {
      parsed = JSON.parse(raw) as { entries?: RunEntry[] };
    } catch (error) {
      throw new AdapterError(
        "entries",
        `input is not JSON: ${(error as Error).message}`,
      );
    }
    if (!Array.isArray(parsed.entries)) {
      throw new AdapterError(
        "entries",
        'expected JSON of the form {"entries": [{symbol, outcome, traceIds?}]}',
      );
    }
    return { entries: parsed.entries };
  },
};

/**
 * Every adapter quoin ships, in the order `--help` lists them.
 *
 * Registration is a list rather than a hardcoded switch so an external user can
 * add an adapter for their own tool. Nothing here is specific to agent-ix
 * repositories: the three formats were chosen to exercise three different
 * corners of the contract, not because of who produces them.
 */
export const ADAPTERS: readonly EvidenceAdapter[] = [
  entriesAdapter,
  junitAdapter,
  cargoMutantsAdapter,
  sbomAdapter,
  agentEvalAdapter,
];

/**
 * Adapters producing a {@link FindingRecord} rather than run entries.
 *
 * A separate registry because the two produce different record types and the
 * command must know which it is writing BEFORE it parses — a scan written into
 * `runs/` would lose the clean-versus-unrun distinction that FR-034 exists to
 * make, silently and at the point of intake.
 */
export interface FindingAdapter {
  readonly name: string;
  readonly summary: string;
  readonly tools: readonly string[];
  parse(raw: string): FindingResult;
}

export const FINDING_ADAPTERS: readonly FindingAdapter[] = [
  {
    name: "sarif",
    summary:
      "SARIF 2.1.0 — semgrep --sarif, CodeQL, ESLint, ZAP via converter.",
    tools: ["sarif", "semgrep", "codeql"],
    parse: parseSarif,
  },
  {
    name: "dependency-cruiser",
    summary:
      "dependency-cruiser JSON — project-owned boundary and cycle rule violations.",
    tools: ["dependency-cruiser", "depcruise"],
    parse: parseDependencyCruiser,
  },
  {
    name: "audit-script",
    summary:
      "Architecture-conformance audit scripts: '<name>: OK' / '<name>: FAIL — … (AC-ID)'.",
    tools: ["audit-script", "make ci", "audit-static", "import-linter"],
    parse: parseAuditScript,
  },
  {
    name: "cargo-audit",
    summary: "cargo audit --json — RUSTSEC advisories and warning kinds.",
    tools: ["cargo-audit", "cargo audit", "cargo-deny", "cargo deny"],
    parse: parseCargoAudit,
  },
];

export const ADAPTER_NAMES: readonly string[] = [
  ...ADAPTERS.map((a) => a.name),
  ...FINDING_ADAPTERS.map((a) => a.name),
];

/**
 * The finding-shaped adapter for these options, or `undefined` when the
 * selection is run-shaped.
 *
 * Checked before {@link selectAdapter} so a `--adapter sarif` never falls
 * through to the run path, where its output would be parsed as entries and
 * fail with a message about JSON shape.
 */
export function selectFindingAdapter(options: {
  adapter?: string;
  tool?: string;
}): FindingAdapter | undefined {
  if (options.adapter !== undefined && options.adapter !== "") {
    return FINDING_ADAPTERS.find((a) => a.name === options.adapter);
  }
  const tool = (options.tool ?? "").toLowerCase();
  if (tool === "") return undefined;
  return FINDING_ADAPTERS.find((a) => a.tools.some((c) => tool.includes(c)));
}

/**
 * Choose an adapter: an explicit `--adapter` wins, else the suite's declared
 * `tool` selects one, else the normalized default.
 *
 * **An unknown explicit name is an error, never a silent fall back to the
 * default.** Falling back would parse the file as normalized entries, fail with
 * a message about JSON shape, and send the reader to look at their JUnit file
 * instead of at their typo.
 */
export function selectAdapter(options: {
  adapter?: string;
  tool?: string;
}): EvidenceAdapter {
  if (options.adapter !== undefined && options.adapter !== "") {
    const named = ADAPTERS.find((a) => a.name === options.adapter);
    if (named === undefined) {
      throw new AdapterError(
        "adapter",
        `unknown adapter '${options.adapter}'. Available: ${ADAPTER_NAMES.join(", ")}`,
      );
    }
    return named;
  }
  const tool = (options.tool ?? "").toLowerCase();
  if (tool !== "") {
    const matched = ADAPTERS.find((a) =>
      a.tools.some((claim) => tool.includes(claim)),
    );
    if (matched !== undefined) return matched;
  }
  return entriesAdapter;
}
