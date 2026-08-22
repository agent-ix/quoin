import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";

import { loadConfig, run, type RunnerLoadOptions } from "@agent-ix/ix-cli-core";

import { packageVersion } from "./version.js";

export { packageVersion, resolveVersion } from "./version.js";

/**
 * Root usage: the bin name plus the top-level command ids oclif discovered.
 *
 * Built from the resolved config rather than a hand-maintained string, so a
 * command added to `dist/commands` (or contributed by a core plugin) appears
 * here without a second edit — the failure mode FR-003's delegation to oclif
 * exists to avoid.
 */
export function rootUsage(config: {
  bin: string;
  commands: ReadonlyArray<{ id: string; hidden?: boolean }>;
}): string {
  const topics = [
    ...new Set(
      config.commands
        .filter((command) => !command.hidden)
        .map((command) => command.id.split(":")[0]),
    ),
  ].sort();
  return `Usage: ${config.bin} <command> [options]\n\nCommands: ${topics.join(", ")}\n\nRun \`${config.bin} <command> --help\` for details.`;
}

// quoin bakes a truthful `git describe` version at build time and prints the
// bare string for `--version` / `-v` / `version` (the oclif default version
// output is decorated with the platform/node triple, which would break the
// historical surface and the build-time drift signal). Intercept those forms
// before handing off to the oclif runner.
const VERSION_REQUESTS = new Set(["--version", "-v", "version"]);

export function isVersionRequest(argv: readonly string[]): boolean {
  return argv.length > 0 && VERSION_REQUESTS.has(argv[0]);
}

/**
 * quoin entry point.
 *
 * Replaces the legacy hand-rolled argv dispatcher: every command now runs as a
 * `BaseCommand` subclass discovered and dispatched by the ix-cli-core oclif
 * runner (FR-016). The only pre-dispatch concern retained here is the bare
 * version print (above).
 *
 * @param argv argument vector (without the node/bin prefix)
 * @param options oclif config source — a file URL (`import.meta.url`), a
 *   directory, or a pre-loaded `Config` (used by tests). Defaults to oclif's
 *   own resolution.
 */
export async function main(
  argv: string[],
  options?: RunnerLoadOptions,
): Promise<void> {
  if (isVersionRequest(argv)) {
    console.log(packageVersion());
    return;
  }
  await run(argv, await versionedConfig(options));
}

/**
 * The oclif {@link Config} with its `version` corrected to the build-time one
 * (#196).
 *
 * oclif reads `version` from `package.json`, which carries `0.9.0` — the
 * placeholder the CI tag rewrite replaces on release. `--version` prints the
 * baked `git describe` string. So a locally built binary reported two
 * different versions depending on which flag you asked:
 *
 * ```
 * $ quoin --version
 * 0.21.2-27-g07aa699
 * $ quoin --help
 * VERSION
 *   @agent-ix/quoin/0.9.0 wsl-x64 node-v22.15.0
 * ```
 *
 * Version provenance is load-bearing here: every SpecReview records the tool
 * version it measured with, and three reviews in `agent-ix/filament-ide-rs`
 * cite numbers from a binary whose self-reported version was wrong. Same class
 * as `agent-ix/quire-cli#52`, where five tags shipped binaries all reporting
 * the version before them.
 *
 * The flag stays the source of truth — it is the one that knows about drift —
 * and the help block is made to agree with it.
 */
export async function versionedConfig(
  options?: RunnerLoadOptions,
): Promise<Config> {
  return loadConfig(withBakedVersion(options));
}

/**
 * The load options with the build-time version injected.
 *
 * `Config` resolves `this.version = this.options.version || this.pjson.version`,
 * so the **option** wins over `package.json` — and `Config.load` rebuilds from
 * `opts.options`, so it survives the reload that `run`/`execute` perform.
 *
 * Mutating an already-loaded `Config` does not work and was the first thing
 * tried: `Config.load` calls `new Config({...opts.options, plugins})` and
 * re-runs `load()`, so the corrected `version` and `userAgent` are both
 * discarded. Recorded here because the failure is silent — the mutation
 * appears to take, and only the rendered help shows it did not.
 *
 * A non-string `options` (a pre-loaded `Config`, as the tests supply) passes
 * through untouched: those callers own their own version.
 */
function withBakedVersion(options?: RunnerLoadOptions): RunnerLoadOptions {
  if (typeof options !== "string") return options as RunnerLoadOptions;
  const root = options.startsWith("file://") ? fileURLToPath(options) : options;
  return { root, version: packageVersion() } as RunnerLoadOptions;
}
