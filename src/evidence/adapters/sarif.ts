import type { Finding } from "../types.js";
import { AdapterError } from "./types.js";

interface SarifResult {
  ruleId?: string;
  rule?: { id?: string };
  level?: string;
  message?: { text?: string };
  locations?: Array<{
    physicalLocation?: {
      artifactLocation?: { uri?: string };
      region?: { startLine?: number };
    };
  }>;
  properties?: Record<string, unknown>;
}

interface SarifRun {
  tool?: { driver?: { name?: string; version?: string; rules?: unknown[] } };
  results?: SarifResult[];
  properties?: Record<string, unknown>;
}

/** What a finding-shaped adapter produces. */
export interface FindingResult {
  findings: Finding[];
  tool?: string;
  ruleset?: string;
  rulesEvaluated?: number;
}

/**
 * SARIF 2.1.0 → findings.
 *
 * **SARIF is the primary finding adapter** because many scanners converge on
 * it — semgrep (`--sarif`), CodeQL, ESLint, ZAP via its converter — so one
 * robust reader subsumes several bespoke ones. Bespoke adapters are kept only
 * where a tool emits no SARIF at all, which is why `cargo-audit` has its own.
 *
 * A `run` with an empty `results` array is a scan that **happened and found
 * nothing**. That is the whole reason this record type exists, and SARIF
 * already models it: the run envelope is the proof of execution, independent of
 * whether anything was reported.
 */
export function parseSarif(raw: string): FindingResult {
  let parsed: { runs?: SarifRun[] };
  try {
    parsed = JSON.parse(raw) as { runs?: SarifRun[] };
  } catch (error) {
    throw new AdapterError(
      "sarif",
      `input is not JSON: ${(error as Error).message}`,
    );
  }
  if (!Array.isArray(parsed.runs)) {
    throw new AdapterError(
      "sarif",
      "no `runs` array — expected a SARIF 2.1.0 log",
    );
  }
  if (parsed.runs.length === 0) {
    // Not the same as a run with no results. A log carrying no run at all is a
    // file that proves nothing executed, and recording it would manufacture
    // exactly the evidence this record type exists to distinguish.
    throw new AdapterError(
      "sarif",
      "`runs` is empty — a log with no run proves no scan executed",
    );
  }

  const findings: Finding[] = [];
  const tools: string[] = [];
  // `undefined` until some driver declares a `rules` array. The distinction
  // between "the tool reported ZERO rules" and "the tool reported NO count" is
  // the one FR-034 turns on: the first is a vacuous scan, the second is a
  // question that cannot be asked. Defaulting to 0 and omitting it when 0
  // erased exactly that difference, so a scan declaring `rules: []` read as a
  // tool that had said nothing.
  let rulesEvaluated: number | undefined;
  for (const run of parsed.runs) {
    const driver = run.tool?.driver;
    if (driver?.name !== undefined && driver.name !== "") {
      tools.push(
        driver.version === undefined
          ? driver.name
          : `${driver.name} ${driver.version}`,
      );
    }
    if (Array.isArray(driver?.rules)) {
      rulesEvaluated = (rulesEvaluated ?? 0) + driver.rules.length;
    }
    for (const result of run.results ?? []) {
      const ruleId = result.ruleId ?? result.rule?.id;
      if (ruleId === undefined || ruleId === "") continue;
      const location = result.locations?.[0]?.physicalLocation;
      findings.push({
        ruleId,
        ...(result.level === undefined ? {} : { severity: result.level }),
        ...(result.message?.text === undefined
          ? {}
          : { message: result.message.text }),
        ...(location?.artifactLocation?.uri === undefined
          ? {}
          : { path: location.artifactLocation.uri }),
        ...(location?.region?.startLine === undefined
          ? {}
          : { line: location.region.startLine }),
      });
    }
  }
  return {
    findings,
    ...(tools.length === 0 ? {} : { tool: tools.join(", ") }),
    ...(rulesEvaluated === undefined ? {} : { rulesEvaluated }),
  };
}

interface AuditAdvisory {
  id?: string;
  title?: string;
}
interface AuditPackage {
  name?: string;
  version?: string;
}
interface AuditEntry {
  advisory?: AuditAdvisory;
  package?: AuditPackage;
  kind?: string;
}

/**
 * `cargo audit --json` → findings.
 *
 * Kept as a bespoke adapter for one measured reason: **cargo-audit emits no
 * SARIF**, so the SARIF reader cannot subsume it. Its native output is what a
 * consumer's CI actually produces.
 *
 * It also models clean-versus-unrun natively — `vulnerabilities.found: false`
 * beside the `database` and `lockfile` it consulted — which is the same shape
 * this record type generalizes.
 */
export function parseCargoAudit(raw: string): FindingResult {
  let parsed: {
    database?: { "advisory-count"?: number };
    vulnerabilities?: { found?: boolean; list?: AuditEntry[] };
    warnings?: Record<string, AuditEntry[]>;
  };
  try {
    parsed = JSON.parse(raw) as typeof parsed;
  } catch (error) {
    throw new AdapterError(
      "cargo-audit",
      `input is not JSON: ${(error as Error).message}`,
    );
  }
  if (parsed.vulnerabilities === undefined) {
    throw new AdapterError(
      "cargo-audit",
      "no `vulnerabilities` object — expected `cargo audit --json` output",
    );
  }

  const findings: Finding[] = [];
  const push = (entry: AuditEntry, severity: string): void => {
    const id = entry.advisory?.id;
    if (id === undefined || id === "") return;
    const pkg = entry.package;
    findings.push({
      ruleId: id,
      severity,
      ...(entry.advisory?.title === undefined
        ? {}
        : { message: entry.advisory.title }),
      ...(pkg?.name === undefined
        ? {}
        : {
            path:
              pkg.version === undefined
                ? pkg.name
                : `${pkg.name}@${pkg.version}`,
          }),
    });
  };
  for (const entry of parsed.vulnerabilities.list ?? [])
    push(entry, "vulnerability");
  // Warnings keep their own kind (`unsound`, `unmaintained`, `yanked`) as the
  // severity string. Flattening them to one word would discard the distinction
  // the tool drew, and quoin normalizes no scanner's severities (FR-034-CON-2).
  for (const [kind, entries] of Object.entries(parsed.warnings ?? {})) {
    for (const entry of entries) push(entry, kind);
  }
  const count = parsed.database?.["advisory-count"];
  return {
    findings,
    tool: "cargo-audit",
    ...(count === undefined ? {} : { rulesEvaluated: count }),
  };
}
