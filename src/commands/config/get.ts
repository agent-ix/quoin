import { Args } from "@oclif/core";
import { runConfigGet } from "@agent-ix/ix-cli-core";

import { QuoinCommand } from "../../base.js";
import { QUOIN_PLUGIN_ID, registerQuoinSchema } from "../../config-schema.js";

export default class ConfigGet extends QuoinCommand {
  static summary = "Read a stored quoin config value.";
  static description = `Values are read from ~/.config/ix/config.d/quoin.yaml, with any project-local
.ix config and a bound environment variable layered over it.`;

  static examples = ["quoin config get org"];

  static args = {
    key: Args.string({ required: true, description: "Config key path." }),
  };

  async run(): Promise<void> {
    const { args } = await this.parse(ConfigGet);
    registerQuoinSchema();
    await runConfigGet(QUOIN_PLUGIN_ID, args.key);
  }
}
