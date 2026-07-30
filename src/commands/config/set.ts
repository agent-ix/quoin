import { Args } from "@oclif/core";
import { runConfigSet } from "@agent-ix/ix-cli-core";

import { QuoinCommand } from "../../base.js";
import { QUOIN_PLUGIN_ID, registerQuoinSchema } from "../../config-schema.js";

export default class ConfigSet extends QuoinCommand {
  static summary = "Store a quoin config value.";
  static description = `Writes to ~/.config/ix/config.d/quoin.yaml. Scalars are coerced through the
schema; an unknown key is rejected rather than silently written.

Storing an org is what makes it persist: --org lasts one invocation and
QUOIN_ORG one shell, but a stored value is read on every run.`;

  static examples = ["quoin config set org acme"];

  static args = {
    key: Args.string({ required: true, description: "Config key path." }),
    value: Args.string({ required: true, description: "Value to store." }),
  };

  async run(): Promise<void> {
    const { args } = await this.parse(ConfigSet);
    registerQuoinSchema();
    await runConfigSet(QUOIN_PLUGIN_ID, args.key, args.value);
  }
}
