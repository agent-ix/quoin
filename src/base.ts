import { BaseCommand, maybeOfferUpdate } from "@agent-ix/ix-cli-core";

import { ixHome } from "./catalog.js";
import { packageVersion } from "./version.js";

/**
 * Shared base for every quoin command.
 *
 * Extends ix-cli-core's {@link BaseCommand} (which owns the `--config-root` /
 * `--no-project-config` global flags and the capability lifecycle) and adds the
 * two cross-cutting concerns the legacy hand-rolled dispatcher used to apply to
 * every invocation:
 *
 *  1. Resolve the quoin home (`--config-root` flag, else `IX_HOME`) and export
 *     it as `process.env.IX_HOME` so the catalog / module modules — which read
 *     `ixHome()` — observe the per-invocation override exactly as before.
 *  2. Nudge toward a newer published quoin (throttled, interactive-only, silent
 *     in CI / under `--json` / for `update` itself). Never blocks or throws.
 */
export abstract class QuoinCommand extends BaseCommand {
  /** Commands that must not trigger the update nudge (e.g. `update`). */
  protected skipUpdateNudge = false;

  /** Resolved quoin home for this invocation. */
  protected home = "";

  public override async init(): Promise<void> {
    await super.init();

    const configRoot =
      configRootFromArgv(this.argv) ?? process.env.IX_CONFIG_ROOT;
    this.home =
      typeof configRoot === "string" && configRoot.length > 0
        ? configRoot
        : ixHome();
    process.env.IX_HOME = this.home;

    if (!this.skipUpdateNudge && !this.argv.includes("--json")) {
      await maybeOfferUpdate({
        packageName: "@agent-ix/quoin",
        currentVersion: packageVersion(),
      });
    }
  }
}

/** Read the `--config-root <dir>` / `--config-root=<dir>` value from argv. */
function configRootFromArgv(argv: readonly string[]): string | undefined {
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--config-root") return argv[i + 1];
    if (arg.startsWith("--config-root=")) {
      return arg.slice("--config-root=".length);
    }
  }
  return undefined;
}
