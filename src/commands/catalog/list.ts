import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { loadCatalog } from "../../catalog.js";
import { ensureDefaultModules } from "../../modules.js";

export default class CatalogList extends QuoinCommand {
  static summary = "List the active artifact/object catalog modules.";
  static description = `The catalog is assembled from, in order:
  1. QUOIN_MODULE_PATHS
  2. modules installed under ~/.ix/filament/modules

The default module set is installed there automatically on first catalog access,
and updated by "quoin module install". The same directory is read by quire-rs.`;

  static examples = ["quoin catalog list", "quoin catalog list --json"];

  static flags = {
    json: Flags.boolean({ description: "Emit the full catalog as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(CatalogList);
    ensureDefaultModules();
    const catalog = loadCatalog();
    if (flags.json) {
      this.log(JSON.stringify(catalog, null, 2));
      return;
    }
    for (const module of catalog.modules) {
      this.log(`${module.name}@${module.version ?? "unknown"} ${module.root}`);
    }
  }
}
