import { existsSync, readFileSync, rmSync } from "node:fs";
import { basename, join } from "node:path";

import { parse as parseYaml } from "yaml";
import {
  installEntry,
  readRegistry,
  writeRegistry,
  type InstallOptions,
  type InstalledPlugin,
  type Source,
} from "@agent-ix/ts-plugin-kit";

import { filamentModulesDir, ixHome } from "./catalog.js";
import {
  duplicatePackageDiagnostic,
  formatDiagnostics,
  hasErrors,
  readModuleSemantic,
  type SemanticModule,
} from "./semantic/manifest.js";
import {
  registryPin,
  resolveImports,
  validatePackageManifest,
  writePackageManifest,
  derivePackageManifest,
  type SemanticRegistryPin,
} from "./semantic/package-manifest.js";

export type { InstalledPlugin } from "@agent-ix/ts-plugin-kit";

export function registryPath(home = ixHome()): string {
  return join(home, "filament", "registry.json");
}

/** Shared ts-plugin-kit install options for this home (default-set reconcile + ad-hoc install). */
export function installOptions(home: string): InstallOptions {
  return {
    cacheRoot: join(home, "cache", "ts-plugin-kit"),
    targetRoot: filamentModulesDir(home),
    registryPath: registryPath(home),
    readName: readModuleName,
    materialize: "copy",
  };
}

/** Map a quoin CLI source argument to a typed ts-plugin-kit {@link Source}. */
export function parseSourceArg(arg: string): Source {
  if (arg.startsWith("path:")) return { type: "path", path: arg.slice(5) };
  if (arg.startsWith("github:")) {
    const [spec, ref] = arg.slice(7).split("@");
    // `owner/repo//subdir` installs a module that lives in a monorepo subdirectory
    // (e.g. `agent-ix/spec-objects-security//spec_objects_security`) — the same
    // git-subdir source the default module set uses.
    const sep = spec.indexOf("//");
    if (sep !== -1) {
      return {
        type: "git-subdir",
        url: spec.slice(0, sep),
        path: spec.slice(sep + 2),
        ref,
      };
    }
    return { type: "github", repo: spec, ref };
  }
  // `package:` is forward-declared: it maps to an npm Source, but ts-plugin-kit's
  // resolveSource currently rejects npm (UnsupportedSourceError), so an install
  // fails until npm support lands. Not advertised in PLUGIN_USAGE.
  if (arg.startsWith("package:")) {
    const spec = arg.slice(8);
    const at = spec.lastIndexOf("@");
    return at > 0
      ? { type: "npm", package: spec.slice(0, at), version: spec.slice(at + 1) }
      : { type: "npm", package: spec };
  }
  return { type: "path", path: arg };
}

export function installPlugin(
  source: string,
  home = ixHome(),
): InstalledPlugin {
  const installed = installEntry(
    { source: parseSourceArg(source) },
    installOptions(home),
  );
  // FR-070 / FR-073: a module whose `semantic` block or `data_schema`
  // references are outside the contract is rejected at install, not loaded
  // as an empty model. The block is optional; modules without it are untouched.
  const root = join(filamentModulesDir(home), installed.name);
  const result = readModuleSemantic(root);
  const diagnostics = [...result.diagnostics];
  if (result.module) {
    const others = installedSemanticModules(home).filter(
      (module) => module.name !== installed.name,
    );
    const duplicate = duplicatePackageDiagnostic(result.module, others);
    if (duplicate) diagnostics.push(duplicate);
    diagnostics.push(...resolveImports(result.module, others));
  }
  if (hasErrors(diagnostics)) {
    removePlugin(installed.name, home);
    throw new Error(
      `module ${installed.name} rejected: semantic contract violations\n${formatDiagnostics(diagnostics)}`,
    );
  }
  if (result.module) {
    // FR-075: derive the package manifest and pin export digests in the registry.
    const manifest = derivePackageManifest(result.module);
    const verdict = validatePackageManifest(manifest);
    if (!verdict.valid) {
      removePlugin(installed.name, home);
      throw new Error(
        `module ${installed.name} rejected: derived package manifest is invalid\n${JSON.stringify(verdict.errors)}`,
      );
    }
    writePackageManifest(result.module, manifest);
    pinSemantic(installed.name, registryPin(result.module), home);
  }
  return installed;
}

/** Record the FR-075 pin under the plugin's registry entry. */
function pinSemantic(
  name: string,
  pin: SemanticRegistryPin,
  home: string,
): void {
  const reg = readRegistry(registryPath(home));
  writeRegistry(registryPath(home), {
    schemaVersion: 1,
    plugins: reg.plugins.map((plugin) =>
      plugin.name === name
        ? ({ ...plugin, semantic: pin } as InstalledPlugin)
        : plugin,
    ),
  });
}

/** The FR-075 pin recorded for an installed module, if any. */
export function semanticPin(
  name: string,
  home = ixHome(),
): SemanticRegistryPin | undefined {
  const plugin = readRegistry(registryPath(home)).plugins.find(
    (entry) => entry.name === name,
  ) as (InstalledPlugin & { semantic?: SemanticRegistryPin }) | undefined;
  return plugin?.semantic;
}

/** Every installed module that declares a semantic block, in sorted root order. */
export function installedSemanticModules(home = ixHome()): SemanticModule[] {
  const modulesDir = filamentModulesDir(home);
  const roots = listPlugins(home)
    .map((plugin) => join(modulesDir, plugin.name))
    .filter((root) => existsSync(join(root, "manifest.yaml")))
    .sort();
  const modules: SemanticModule[] = [];
  for (const root of roots) {
    const result = readModuleSemantic(root);
    if (result.module) modules.push(result.module);
  }
  return modules;
}

export function listPlugins(home = ixHome()): InstalledPlugin[] {
  return readRegistry(registryPath(home)).plugins;
}

export function removePlugin(name: string, home = ixHome()): void {
  const reg = readRegistry(registryPath(home));
  rmSync(join(filamentModulesDir(home), name), {
    force: true,
    recursive: true,
  });
  writeRegistry(registryPath(home), {
    schemaVersion: 1,
    plugins: reg.plugins.filter((plugin) => plugin.name !== name),
  });
}

/** Read a module's declared name from its `manifest.yaml`. Used as the `readName` hook. */
export function readModuleName(moduleRoot: string): string {
  const manifestPath = existsSync(join(moduleRoot, "manifest.yaml"))
    ? join(moduleRoot, "manifest.yaml")
    : join(moduleRoot, basename(moduleRoot), "manifest.yaml");
  if (!existsSync(manifestPath))
    throw new Error(`no manifest.yaml found under ${moduleRoot}`);
  const manifest = parseYaml(readFileSync(manifestPath, "utf8")) as {
    name?: unknown;
  };
  if (typeof manifest.name !== "string" || manifest.name.length === 0) {
    throw new Error(`manifest ${manifestPath} has no name`);
  }
  return manifest.name;
}
