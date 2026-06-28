import ModuleEnsureDefaults from "../module/ensure-defaults.js";
import { PLUGIN_DEPRECATION_NOTICE } from "./deprecation.js";

/** Deprecated alias for `quoin module ensure-defaults`. */
export default class PluginEnsureDefaults extends ModuleEnsureDefaults {
  static override summary =
    "[DEPRECATED] Alias for `quoin module ensure-defaults`.";
  static override hidden = true;

  override async run(): Promise<void> {
    this.logToStderr(PLUGIN_DEPRECATION_NOTICE);
    await super.run();
  }
}
