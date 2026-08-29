import { runConfigEdit } from "@agent-ix/ix-cli-core";

import { QuoinCommand } from "../../base.js";
import { QUOIN_PLUGIN_ID, registerQuoinSchema } from "../../config-schema.js";

export default class ConfigEdit extends QuoinCommand {
  static summary = "Open quoin's config file in $EDITOR.";

  static examples = ["quoin config edit"];

  async run(): Promise<void> {
    await this.parse(ConfigEdit);
    registerQuoinSchema();
    await runConfigEdit(QUOIN_PLUGIN_ID);
  }
}
