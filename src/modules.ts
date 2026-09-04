import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { parse as parseYaml } from "yaml";
import {
  reconcile,
  validateMarketplaceManifest,
  type MarketplaceManifest,
} from "@agent-ix/ts-plugin-kit";

import { ixHome } from "./catalog.js";
import { installOptions, validateInstalledSemantics } from "./plugins.js";

function packageRoot(): string {
  return dirname(dirname(fileURLToPath(import.meta.url)));
}

/** The committed default module set shipped with quoin. */
export function defaultModulesManifest(): MarketplaceManifest {
  const path = join(packageRoot(), "default-modules.yaml");
  return validateMarketplaceManifest(parseYaml(readFileSync(path, "utf8")));
}

/**
 * Lazily install the default module set into `~/.ix/filament/modules`. Idempotent
 * and (once installed and pinned) does no git — safe to call before every catalog
 * read. Pass a manifest to override the committed default set (used by tests).
 */
export function ensureDefaultModules(
  home = ixHome(),
  manifest: MarketplaceManifest = defaultModulesManifest(),
): void {
  reconcile(manifest, { ...installOptions(home), mode: "lazy" });
  validateInstalledSemantics(home);
}
