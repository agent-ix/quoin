import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { basename, join, resolve } from "node:path";

import { parse as parseYaml } from "yaml";

export interface SpecCatalogEntry {
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

export function ixHome(): string {
  return process.env.IX_HOME && process.env.IX_HOME.length > 0
    ? process.env.IX_HOME
    : join(homedir(), ".ix");
}

/** The single directory that holds installed Filament modules; also read by quire-rs. */
export function filamentModulesDir(home = ixHome()): string {
  return join(home, "filament", "modules");
}

export function defaultModuleRoots(home = ixHome()): string[] {
  const roots: string[] = [];
  const env = process.env.QUOIN_MODULE_PATHS;
  if (env) roots.push(...env.split(":").filter(Boolean));

  const installed = filamentModulesDir(home);
  if (existsSync(installed)) {
    for (const name of readdirSync(installed))
      roots.push(join(installed, name));
  }
  return roots;
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
      });
    }
  }

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

function locateModuleRoot(candidate: string): string | undefined {
  const root = resolve(candidate);
  if (!existsSync(root)) return undefined;
  if (existsSync(join(root, "manifest.yaml"))) return root;
  if (!statSync(root).isDirectory()) return undefined;
  for (const child of readdirSync(root)) {
    const childRoot = join(root, child);
    if (existsSync(join(childRoot, "manifest.yaml"))) return childRoot;
  }
  return undefined;
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
