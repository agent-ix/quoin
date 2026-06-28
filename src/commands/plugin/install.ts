import ModuleInstall from "../module/install.js";
import { PLUGIN_DEPRECATION_NOTICE } from "./deprecation.js";

/** Deprecated alias for `quoin module install`. */
export default class PluginInstall extends ModuleInstall {
  static override summary = "[DEPRECATED] Alias for `quoin module install`.";
  static override hidden = true;

  override async run(): Promise<void> {
    this.logToStderr(PLUGIN_DEPRECATION_NOTICE);
    await super.run();
  }
}
