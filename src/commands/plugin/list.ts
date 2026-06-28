import ModuleList from "../module/list.js";
import { PLUGIN_DEPRECATION_NOTICE } from "./deprecation.js";

/** Deprecated alias for `quoin module list`. */
export default class PluginList extends ModuleList {
  static override summary = "[DEPRECATED] Alias for `quoin module list`.";
  static override hidden = true;

  override async run(): Promise<void> {
    this.logToStderr(PLUGIN_DEPRECATION_NOTICE);
    await super.run();
  }
}
