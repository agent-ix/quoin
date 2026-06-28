import { QuoinCommand } from "../../base.js";
import { listPlugins } from "../../plugins.js";

export default class ModuleList extends QuoinCommand {
  static summary = "List installed spec modules.";
  static description = `Spec modules add or override artifact/object types under ~/.ix/filament/modules.
Use "quoin catalog list" to see the full active catalog, including default
modules and any installed modules.`;

  static examples = ["quoin module list"];

  async run(): Promise<void> {
    await this.parse(ModuleList);
    this.log(JSON.stringify({ plugins: listPlugins() }, null, 2));
  }
}
