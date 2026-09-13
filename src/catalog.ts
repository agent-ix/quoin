import { readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";

import { parse as parseYaml } from "yaml";

import {
  readSemanticBlocks,
  type SemanticBlock,
  type SemanticDiagnostic,
} from "./core/semantic.js";
import { defaultModuleRoots, locateModuleRoot } from "./module-roots.js";

/**
 * Where modules live, re-exported from `src/module-roots.ts`.
 *
 * These four used to be declared here and are still imported from here by
 * `src/base.ts`, `src/flows.ts`, `src/method-catalog.ts` and the CLI commands.
 * They moved one module down so that `catalog.ts` may import `src/core/` without
 * a cycle (quoin#376); re-exporting keeps every existing import working.
 */
export {
  defaultModuleRoots,
  filamentModulesDir,
  ixHome,
  locateModuleRoot,
} from "./module-roots.js";

export interface SpecCatalogEntry {
  /** Raw `data_schema` of an object type (inline object or FR-073 reference). */
  dataSchema?: unknown;
  name: string;
  kind: "artifact" | "object";
  moduleName: string;
  moduleRoot: string;
  schemaRef?: string;
  schemaPath?: string;
  skeletonPath?: string;
}

export interface SpecModule {
  name: string;
  version?: string;
  root: string;
  artifactTypes: string[];
  objectTypes: string[];
  /** Parsed `semantic` block (FR-070), absent when the manifest has none or it is invalid. */
  semantic?: SemanticBlock;
  /** Diagnostics from reading the semantic block at load time (install-time rejection lives in plugins.ts). */
  semanticDiagnostics?: SemanticDiagnostic[];
}

export interface SpecCatalog {
  modules: SpecModule[];
  entries: SpecCatalogEntry[];
  duplicates: Array<{
    kind: "artifact" | "object";
    name: string;
    modules: string[];
  }>;
}

/**
 * Read every module's `semantic` block across the boundary, in ONE call.
 *
 * One call and not one per module: `quoin-core` is a subprocess, `loadCatalog`
 * runs on every `quoin write`, and a crossing per module would make the cost of
 * the boundary proportional to the installed module set. The answers come back
 * in the order asked and each echoes the root it answers for, so the pairing is
 * by name rather than by position.
 */
function attachSemanticBlocks(modules: SpecModule[]): void {
  const views = readSemanticBlocks(modules.map((module) => module.root));
  const byRoot = new Map(views.map((view) => [view.root, view]));
  for (const module of modules) {
    const view = byRoot.get(module.root);
    if (!view) continue;
    if (view.block) module.semantic = view.block;
    if (view.diagnostics.length > 0)
      module.semanticDiagnostics = view.diagnostics;
  }
}

export function loadCatalog(moduleRoots = defaultModuleRoots()): SpecCatalog {
  const modules: SpecModule[] = [];
  const entries: SpecCatalogEntry[] = [];
  const seenRoots = new Set<string>();
  const seenModuleNames = new Set<string>();

  for (const candidate of moduleRoots) {
    const moduleRoot = locateModuleRoot(candidate);
    if (!moduleRoot || seenRoots.has(moduleRoot)) continue;
    seenRoots.add(moduleRoot);

    const manifestPath = join(moduleRoot, "manifest.yaml");
    const manifest = parseYaml(readFileSync(manifestPath, "utf8")) as Record<
      string,
      unknown
    >;
    const moduleName = stringValue(manifest.name) ?? basename(moduleRoot);
    if (seenModuleNames.has(moduleName)) continue;
    seenModuleNames.add(moduleName);

    const artifactTypes = arrayObjects(manifest.artifact_types);
    const objectTypes = arrayObjects(manifest.object_types);

    modules.push({
      name: moduleName,
      version: stringValue(manifest.version),
      root: moduleRoot,
      artifactTypes: artifactTypes.map((entry) => String(entry.name)),
      objectTypes: objectTypes.map((entry) => String(entry.name)),
    });

    for (const artifact of artifactTypes) {
      const name = String(artifact.name);
      const schemaRef = stringValue(artifact.frontmatter_schema_ref);
      entries.push({
        name,
        kind: "artifact",
        moduleName,
        moduleRoot,
        schemaRef,
        schemaPath: schemaRef ? join(moduleRoot, schemaRef) : undefined,
        skeletonPath: skeletonPath(moduleRoot, name),
      });
    }
    for (const object of objectTypes) {
      const name = String(object.name);
      entries.push({
        name,
        kind: "object",
        moduleName,
        moduleRoot,
        skeletonPath: skeletonPath(moduleRoot, name),
        ...(object.data_schema !== undefined
          ? { dataSchema: object.data_schema }
          : {}),
      });
    }
  }

  attachSemanticBlocks(modules);
  return { modules, entries, duplicates: findDuplicates(entries) };
}

export function findCatalogEntry(
  catalog: SpecCatalog,
  name: string,
): SpecCatalogEntry | undefined {
  const normalized = normalizeTypeName(name);
  return catalog.entries.find(
    (entry) => normalizeTypeName(entry.name) === normalized,
  );
}

function normalizeTypeName(name: string): string {
  return name.toLowerCase();
}

function skeletonPath(
  moduleRoot: string,
  typeName: string,
): string | undefined {
  // Match against real directory entries rather than probing with existsSync:
  // on a case-insensitive filesystem `FR.md` "exists" when the file on disk is
  // `fr.md`, so probing returns a path whose casing does not match disk.
  const dir = join(moduleRoot, "skeletons");
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return undefined;
  }
  const wanted = [`${typeName}.md`, `${typeName.toLowerCase()}.md`];
  const name = wanted.find((candidate) => entries.includes(candidate));
  return name ? join(dir, name) : undefined;
}

function arrayObjects(value: unknown): Array<Record<string, unknown>> {
  return Array.isArray(value)
    ? value.filter(
        (entry): entry is Record<string, unknown> =>
          !!entry && typeof entry === "object" && "name" in entry,
      )
    : [];
}

function stringValue(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function findDuplicates(
  entries: SpecCatalogEntry[],
): SpecCatalog["duplicates"] {
  const grouped = new Map<string, Set<string>>();
  for (const entry of entries) {
    const key = `${entry.kind}:${entry.name}`;
    const modules = grouped.get(key) ?? new Set<string>();
    modules.add(entry.moduleName);
    grouped.set(key, modules);
  }
  return [...grouped.entries()]
    .filter(([, modules]) => modules.size > 1)
    .map(([key, modules]) => {
      const [kind, name] = key.split(":") as ["artifact" | "object", string];
      return { kind, name, modules: [...modules].sort() };
    });
}
