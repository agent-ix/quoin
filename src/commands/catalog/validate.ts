import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { loadCatalog } from "../../catalog.js";
import { ensureDefaultModules } from "../../modules.js";

export default class CatalogValidate extends QuoinCommand {
  static summary = "Validate the active catalog has no duplicate types.";
  static examples = ["quoin catalog validate", "quoin catalog validate --json"];

  static flags = {
    json: Flags.boolean({ description: "Emit the result as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(CatalogValidate);
    ensureDefaultModules();
    const catalog = loadCatalog();
    if (catalog.duplicates.length > 0) {
      this.logToStderr(
        JSON.stringify({ ok: false, duplicates: catalog.duplicates }, null, 2),
      );
      process.exitCode = 1;
      return;
    }
    this.log(
      flags.json
        ? JSON.stringify({ ok: true, modules: catalog.modules.length }, null, 2)
        : `catalog ok (${catalog.modules.length} modules)`,
    );
  }
}
