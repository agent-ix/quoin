import { Args } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { removeModule } from "../../core/modules.js";

export default class ModuleRemove extends QuoinCommand {
  static summary = "Remove an installed spec module.";
  static examples = ["quoin module remove spec-objects-custom"];

  static args = {
    name: Args.string({ description: "Installed module name." }),
  };

  async run(): Promise<void> {
    const { args } = await this.parse(ModuleRemove);
    const name = args.name;
    if (!name) throw new Error("module remove requires <name>");
    removeModule(name);
    this.log(`removed ${name}`);
  }
}
