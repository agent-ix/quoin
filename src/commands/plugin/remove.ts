import ModuleRemove from "../module/remove.js";
import { PLUGIN_DEPRECATION_NOTICE } from "./deprecation.js";

/** Deprecated alias for `quoin module remove`. */
export default class PluginRemove extends ModuleRemove {
  static override summary = "[DEPRECATED] Alias for `quoin module remove`.";
  static override hidden = true;

  override async run(): Promise<void> {
    this.logToStderr(PLUGIN_DEPRECATION_NOTICE);
    await super.run();
  }
}
