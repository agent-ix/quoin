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
  // `package:@scope/name[@version]` installs a module from an npm package whose
  // tarball root is the module (manifest.yaml at the top) — e.g. the published
  // @agent-ix/spec-* modules. ts-plugin-kit resolves it via `npm pack`.
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
  return installEntry({ source: parseSourceArg(source) }, installOptions(home));
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
