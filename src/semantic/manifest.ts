/**
 * The `semantic` manifest block (FR-070, issue #293).
 *
 * Quoin reads the block at `quoin module install` and rejects a manifest
 * whose block is outside the contract rather than degrading to an empty model;
 * Quire applies the same vendored schema at artifact-validation time
 * (quire-rs#388). The block is optional: a manifest without it loads exactly as
 * before.
 */

import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import type { ValidateFunction } from "ajv";
import Ajv2020 from "ajv/dist/2020.js";
import { parse as parseYaml } from "yaml";

import {
  SEMANTIC_CONTRACT,
  moduleManifestSchemaPath,
  readJson,
  sweepReportSchemaPath,
} from "./contract.js";
import {
  resolveDataSchema,
  type ResolvedDataSchema,
  type SemanticDiagnostic,
} from "./data-schema.js";

export type { SemanticDiagnostic } from "./data-schema.js";

export interface SemanticBlock {
  contract_version: string;
  semantic_core: string;
  package: string;
  exports: string[];
  imports: Record<string, string>;
  targets: string[];
  mappings: string[];
  compatibility_posture: "strict" | "additive" | "declared-lossy";
  legacy_forms: "warning" | "error";
  sweep_report?: string;
}

export interface SemanticModule {
  name: string;
  version: string;
  root: string;
  block: SemanticBlock;
  /** Resolved `data_schema` per object type name. */
  dataSchemas: Record<string, ResolvedDataSchema>;
}

export interface SemanticReadResult {
  module?: SemanticModule;
  diagnostics: SemanticDiagnostic[];
}

type Json = Record<string, unknown>;

function isObject(value: unknown): value is Json {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

let cachedValidator: ValidateFunction | undefined;

function semanticBlockValidator(): ValidateFunction {
  if (cachedValidator) return cachedValidator;
  const schema = readJson(moduleManifestSchemaPath()) as Json;
  const block = (schema.properties as Json).semantic as Json;
  const ajv = new Ajv2020({
    verbose: true,
    allErrors: true,
    strict: true,
    useDefaults: false,
  });
  cachedValidator = ajv.compile(block);
  return cachedValidator;
}

interface AjvError {
  data?: unknown;
  instancePath: string;
  keyword: string;
  message?: string;
  params: Record<string, unknown>;
}

function mapAjvError(error: AjvError): SemanticDiagnostic {
  const at = `semantic${error.instancePath.replace(/\//g, ".")}`;
  switch (error.keyword) {
    case "additionalProperties":
      return {
        code: "semantic.unknown-key",
        severity: "error",
        path: `${at}.${String(error.params.additionalProperty)}`,
        message: `unknown key inside semantic: ${String(error.params.additionalProperty)}`,
      };
    case "required":
      return {
        code: "semantic.missing-key",
        severity: "error",
        path: `${at}.${String(error.params.missingProperty)}`,
        message: `semantic block requires ${String(error.params.missingProperty)}`,
      };
    case "enum":
      return {
        code:
          at.endsWith("targets") || /targets\.\d+$/.test(at)
            ? "semantic.unknown-target"
            : "semantic.invalid-value",
        severity: "error",
        path: at,
        message: `${at} is ${JSON.stringify(error.data)}, not one of ${JSON.stringify(error.params.allowedValues)}`,
      };
    case "pattern":
      return {
        code: at.endsWith("package")
          ? "semantic.invalid-package"
          : "semantic.invalid-value",
        severity: "error",
        path: at,
        message: `${at} does not match ${String(error.params.pattern)}`,
      };
    default:
      return {
        code: "semantic.invalid-value",
        severity: "error",
        path: at,
        message: `${at}: ${error.message ?? error.keyword}`,
      };
  }
}

/** Read and validate the `semantic` block of one parsed manifest. */
export function readSemanticBlock(
  manifest: Json,
  moduleRoot: string,
): SemanticReadResult {
  const raw = manifest.semantic;
  if (raw === undefined) return { diagnostics: [] };
  const diagnostics: SemanticDiagnostic[] = [];
  if (!isObject(raw)) {
    return {
      diagnostics: [
        {
          code: "semantic.invalid-value",
          severity: "error",
          path: "semantic",
          message: "semantic must be an object",
        },
      ],
    };
  }
  // The contract version gates everything else: an unknown version is rejected
  // before any other key is read (FR-070).
  if (raw.contract_version !== SEMANTIC_CONTRACT.contractVersion) {
    return {
      diagnostics: [
        {
          code: "semantic.unsupported-contract-version",
          severity: "error",
          path: "semantic.contract_version",
          message: `semantic.contract_version ${String(raw.contract_version)} is not ${SEMANTIC_CONTRACT.contractVersion}`,
        },
      ],
    };
  }
  const validate = semanticBlockValidator();
  if (!validate(raw)) {
    for (const error of (validate.errors ?? []) as AjvError[])
      diagnostics.push(mapAjvError(error));
    return { diagnostics };
  }
  const objectTypes = Array.isArray(manifest.object_types)
    ? (manifest.object_types as unknown[]).filter(isObject)
    : [];
  const objectNames = new Set(objectTypes.map((entry) => String(entry.name)));
  const exports = Array.isArray(raw.exports) ? (raw.exports as string[]) : [];
  for (const name of exports) {
    if (!objectNames.has(name)) {
      diagnostics.push({
        code: "semantic.unknown-export",
        severity: "error",
        path: `semantic.exports.${name}`,
        message: `semantic.exports names ${name}, which object_types does not declare`,
      });
    }
  }
  const semanticCore = String(raw.semantic_core);
  if (
    !(SEMANTIC_CONTRACT.semanticCoreVersions as readonly string[]).includes(
      semanticCore,
    )
  ) {
    diagnostics.push({
      code: "semantic.unknown-semantic-core",
      severity: "error",
      path: "semantic.semantic_core",
      message: `semantic_core ${semanticCore} is not a version this quoin ships (${SEMANTIC_CONTRACT.semanticCoreVersions.join(", ")})`,
    });
  }
  const block: SemanticBlock = {
    contract_version: String(raw.contract_version),
    semantic_core: semanticCore,
    package: String(raw.package),
    exports,
    imports: isObject(raw.imports)
      ? (raw.imports as Record<string, string>)
      : {},
    targets: Array.isArray(raw.targets) ? (raw.targets as string[]) : [],
    mappings: Array.isArray(raw.mappings) ? (raw.mappings as string[]) : [],
    compatibility_posture:
      (raw.compatibility_posture as SemanticBlock["compatibility_posture"]) ??
      "additive",
    legacy_forms:
      (raw.legacy_forms as SemanticBlock["legacy_forms"]) ?? "warning",
    ...(typeof raw.sweep_report === "string"
      ? { sweep_report: raw.sweep_report }
      : {}),
  };
  if (block.legacy_forms === "error") {
    const problem = sweepReportProblem(
      block,
      moduleRoot,
      String(manifest.version),
    );
    if (problem) {
      diagnostics.push({
        code: "semantic.sweep-report-required",
        severity: "error",
        path: "semantic.legacy_forms",
        message: problem,
      });
    }
  }
  const dataSchemas: Record<string, ResolvedDataSchema> = {};
  for (const entry of objectTypes) {
    const name = String(entry.name);
    if (entry.data_schema === undefined) continue;
    const resolved = resolveDataSchema(
      entry.data_schema,
      {
        moduleRoot,
        packageIdentity: block.package,
        moduleVersion: String(manifest.version),
        semanticCore,
        objectType: name,
      },
      true,
    );
    dataSchemas[name] = resolved;
    diagnostics.push(...resolved.diagnostics);
  }
  for (const name of block.exports) {
    if (dataSchemas[name]?.kind !== "reference") {
      diagnostics.push({
        code: "semantic.export-without-schema",
        severity: "error",
        path: `semantic.exports.${name}`,
        message: `semantic.exports names ${name}, whose data_schema is not a { schema, digest } reference; nothing can be pinned for it`,
      });
    }
  }
  return {
    module: {
      name: String(manifest.name),
      version: String(manifest.version),
      root: moduleRoot,
      block,
      dataSchemas,
    },
    diagnostics,
  };
}

/** FR-074: `legacy_forms: error` needs a shipped sweep report for this package and version. */
let cachedSweepValidator: ValidateFunction | undefined;

function sweepReportValidator(): ValidateFunction {
  if (!cachedSweepValidator) {
    const ajv = new Ajv2020({ allErrors: true, strict: true });
    cachedSweepValidator = ajv.compile(
      readJson(sweepReportSchemaPath()) as Record<string, unknown>,
    );
  }
  return cachedSweepValidator;
}

function sweepReportProblem(
  block: SemanticBlock,
  moduleRoot: string,
  version: string,
): string | undefined {
  if (!block.sweep_report)
    return "legacy_forms: error requires semantic.sweep_report";
  if (block.sweep_report.split(/[\\/]/).includes(".."))
    return "sweep_report escapes the module root";
  const file = resolve(moduleRoot, block.sweep_report);
  if (!existsSync(file))
    return `sweep_report ${block.sweep_report} is not shipped`;
  let report: unknown;
  try {
    report = JSON.parse(readFileSync(file, "utf8"));
  } catch {
    return `sweep_report ${block.sweep_report} is not JSON`;
  }
  if (!isObject(report))
    return `sweep_report ${block.sweep_report} is not an object`;
  if (report.package !== block.package)
    return `sweep_report is for package ${String(report.package)}, manifest is ${block.package}`;
  if (report.version !== version)
    return `sweep_report is for version ${String(report.version)}, manifest is ${version}`;
  const validate = sweepReportValidator();
  if (!validate(report)) {
    const first = validate.errors?.[0];
    return `sweep_report ${block.sweep_report} does not validate against the sweep-report schema: ${first?.instancePath || "/"} ${first?.message ?? ""}`.trim();
  }
  return undefined;
}

/** Read a module root's manifest and its semantic block. */
export function readModuleSemantic(moduleRoot: string): SemanticReadResult {
  const manifestPath = join(moduleRoot, "manifest.yaml");
  const manifest = parseYaml(readFileSync(manifestPath, "utf8")) as Json;
  return readSemanticBlock(manifest, moduleRoot);
}

/** Reject a module whose semantic package is already provided by another installed module (FR-070). */
export function duplicatePackageDiagnostic(
  candidate: SemanticModule,
  installed: SemanticModule[],
): SemanticDiagnostic | undefined {
  const other = installed.find(
    (module) =>
      module.block.package === candidate.block.package &&
      module.root !== candidate.root,
  );
  if (!other) return undefined;
  return {
    code: "semantic.duplicate-package",
    severity: "error",
    path: "semantic.package",
    message: `semantic.package ${candidate.block.package} is already declared by module ${other.name} at ${other.root}; ${candidate.name} at ${candidate.root} is rejected (sorted root order)`,
  };
}

export function hasErrors(diagnostics: SemanticDiagnostic[]): boolean {
  return diagnostics.some((d) => d.severity === "error");
}

export function formatDiagnostics(diagnostics: SemanticDiagnostic[]): string {
  return diagnostics
    .map((d) => `${d.severity}: ${d.code} at ${d.path}: ${d.message}`)
    .join("\n");
}
