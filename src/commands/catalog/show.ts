import { Args, Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { findCatalogEntry, loadCatalog } from "../../catalog.js";
import { ensureDefaultModules } from "../../modules.js";

export default class CatalogShow extends QuoinCommand {
  static summary = "Show a single artifact/object type from the catalog.";
  static examples = [
    "quoin catalog show FR",
    "quoin catalog show entity",
    "quoin catalog show FR --json",
  ];

  static args = {
    type: Args.string({ description: "Artifact or object type name." }),
  };

  static flags = {
    json: Flags.boolean({ description: "Emit the entry as JSON." }),
  };

  async run(): Promise<void> {
    const { args, flags } = await this.parse(CatalogShow);
    ensureDefaultModules();
    const catalog = loadCatalog();
    const name = args.type;
    if (!name) throw new Error("catalog show requires <type>");
    const entry = findCatalogEntry(catalog, name);
    if (!entry) throw new Error(`catalog type not found: ${name}`);
    this.log(
      flags.json
        ? JSON.stringify(entry, null, 2)
        : `${entry.kind} ${entry.name} from ${entry.moduleName}\n${entry.moduleRoot}`,
    );
  }
}
