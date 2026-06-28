import { QuoinCommand } from "../../base.js";
import { ensureDefaultModules } from "../../modules.js";
import { listPlugins } from "../../plugins.js";

export default class ModuleEnsureDefaults extends QuoinCommand {
  static summary = "Idempotently install the default spec module set.";
  static description = `Materializes the default module set into ~/.ix/filament/modules. Exposed as an
explicit command so other tools (notably "quire validate") can lazy-install the
defaults by shelling out here, rather than relying on the side effect of
"quoin catalog list".`;

  static examples = ["quoin module ensure-defaults"];

  async run(): Promise<void> {
    await this.parse(ModuleEnsureDefaults);
    ensureDefaultModules();
    this.log(
      JSON.stringify(
        { ensured: true, plugins: listPlugins().map((p) => p.name) },
        null,
        2,
      ),
    );
  }
}
