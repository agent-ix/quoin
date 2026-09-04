/**
 * Derived package manifest and registry pins (FR-075, issue #293).
 *
 * From a module's `semantic` block Quoin derives the filament-core-data
 * `package-manifest` document (FR-021 there) and records one schema digest per
 * exported object type in the ts-plugin-kit registry entry, so dynamic use and
 * a later generated package share one identity graph. The catalog lock
 * (quoin#287) does not exist yet; the pins live where a surface does.
 */

import { writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

import type { ValidateFunction } from "ajv";
import Ajv2020 from "ajv/dist/2020.js";

import {
  commonSchemaPath,
  packageManifestSchemaPath,
  readJson,
} from "./contract.js";
import { fileSha256 } from "./contract.js";
import type { SemanticDiagnostic } from "./data-schema.js";
import type { SemanticModule } from "./manifest.js";

type Json = Record<string, unknown>;

export const SEMANTIC_CORE_PACKAGE = "agent-ix/semantic-core";

export function typeIdentity(packageIdentity: string, name: string): string {
  const [org, repo] = packageIdentity.split("/");
  return `ix://${org}/${repo}/type/${name}`;
}

/** `semantic.mappings` names a mapping; the package manifest records its identity. */
export function mappingIdentity(packageIdentity: string, name: string): string {
  const [org, repo] = packageIdentity.split("/");
  return `ix://${org}/${repo}/mapping/${name}`;
}

/** Derive the filament-core-data package manifest for one semantic module (FR-075). */
export function derivePackageManifest(module: SemanticModule): Json {
  const { block } = module;
  const imports = [
    {
      packageIdentity: SEMANTIC_CORE_PACKAGE,
      versionConstraint: `=${block.semantic_core}`,
      exports: [],
      capabilities: [],
    },
    ...Object.entries(block.imports)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([packageIdentity, version]) => ({
        packageIdentity,
        versionConstraint: `=${version}`,
        exports: [],
        capabilities: [],
      })),
  ];
  const mappings = [...block.mappings]
    .sort()
    .map((name) => mappingIdentity(block.package, name));
  const exports = [...block.exports].sort().map((name) => ({
    name,
    typeIdentity: typeIdentity(block.package, name),
    visibility: "public",
  }));
  return {
    contractVersion: "1.0.0",
    package: { identity: block.package, version: module.version },
    schemaDialect: "https://json-schema.org/draft/2020-12/schema",
    sourceRoots: ["schemas/"],
    imports,
    exports,
    profiles: [
      {
        name: "default",
        version: module.version,
        exports: exports.map((entry) => entry.name),
        targets: [...block.targets],
        mappings: mappings,
        options: {},
        compatibilityPosture: block.compatibility_posture,
      },
    ],
    targets: [...block.targets],
    mappings,
    extensions: [],
  };
}

let cachedValidator: ValidateFunction | undefined;

/** Validate a derived manifest against the vendored filament-core-data schema. */
export function validatePackageManifest(manifest: unknown): {
  valid: boolean;
  errors: unknown;
} {
  if (!cachedValidator) {
    const ajv = new Ajv2020({ allErrors: true, strict: true });
    ajv.addSchema(readJson(commonSchemaPath()) as Json);
    cachedValidator = ajv.compile(
      readJson(packageManifestSchemaPath()) as Json,
    );
  }
  const validate = cachedValidator;
  const valid = validate(manifest);
  return { valid, errors: validate.errors ?? null };
}

/** One sha256 per exported object type's referenced schema (FR-075-AC-2). */
export function exportDigests(module: SemanticModule): Record<string, string> {
  const digests: Record<string, string> = {};
  for (const name of [...module.block.exports].sort()) {
    const resolved = module.dataSchemas[name];
    if (resolved?.kind === "reference" && resolved.file) {
      digests[name] = fileSha256(resolved.file);
    }
  }
  return digests;
}

/** The registry pin recorded under a plugin entry's `semantic` key. */
export interface SemanticRegistryPin {
  package: string;
  semanticCore: string;
  exports: Record<string, string>;
}

export function registryPin(module: SemanticModule): SemanticRegistryPin {
  return {
    package: module.block.package,
    semanticCore: module.block.semantic_core,
    exports: exportDigests(module),
  };
}

/** Write `<module root>/semantic/package-manifest.json`; returns the path. */
export function writePackageManifest(
  module: SemanticModule,
  manifest: Json = derivePackageManifest(module),
): string {
  const dir = join(module.root, "semantic");
  mkdirSync(dir, { recursive: true });
  const path = join(dir, "package-manifest.json");
  writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
  return path;
}

/**
 * Resolve `semantic.imports` against the installed semantic modules
 * (FR-075): every import must be provided at exactly that version, and the
 * import graph must be acyclic.
 */
export function resolveImports(
  candidate: SemanticModule,
  installed: SemanticModule[],
): SemanticDiagnostic[] {
  const diagnostics: SemanticDiagnostic[] = [];
  const byPackage = new Map(
    [candidate, ...installed].map((module) => [module.block.package, module]),
  );
  for (const [packageIdentity, version] of Object.entries(
    candidate.block.imports,
  )) {
    const provider = byPackage.get(packageIdentity);
    if (!provider || provider.version !== version) {
      const installedVersions = installed
        .filter((module) => module.block.package === packageIdentity)
        .map((module) => module.version);
      diagnostics.push({
        code: "semantic.import-unresolved",
        severity: "error",
        path: `semantic.imports.${packageIdentity}`,
        message: `import ${packageIdentity}@${version} is provided by no installed module (installed: ${installedVersions.length > 0 ? installedVersions.join(", ") : "none"})`,
      });
    }
  }
  // Cycle detection over the import graph rooted at the candidate.
  const state = new Map<string, "open" | "done">();
  const trail: string[] = [];
  const visit = (packageIdentity: string): void => {
    const module = byPackage.get(packageIdentity);
    if (!module) return;
    if (state.get(packageIdentity) === "open") {
      const cycle = [
        ...trail.slice(trail.indexOf(packageIdentity)),
        packageIdentity,
      ];
      diagnostics.push({
        code: "semantic.import-cycle",
        severity: "error",
        path: "semantic.imports",
        message: `import cycle: ${cycle.join(" -> ")}`,
      });
      return;
    }
    if (state.get(packageIdentity) === "done") return;
    state.set(packageIdentity, "open");
    trail.push(packageIdentity);
    for (const dependency of Object.keys(module.block.imports))
      visit(dependency);
    trail.pop();
    state.set(packageIdentity, "done");
  };
  visit(candidate.block.package);
  return diagnostics;
}
