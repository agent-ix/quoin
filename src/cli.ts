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

/**
 * `true` when argv's first non-flag token names no command quoin ships.
 *
 * Decided from the resolved config — quoin's own data — rather than by sniffing
 * the thrown oclif error. That error carries no stable discriminator: its `code`
 * is an own property whose value is `undefined`, and its message is user-facing
 * prose, so matching on either would be matching on an accident.
 */
export function isUnknownCommand(
  argv: readonly string[],
  config: { commands: ReadonlyArray<{ id: string; aliases?: string[] }> },
): boolean {
  const requested = argv.find((token) => !token.startsWith("-"));
  if (requested === undefined) return false;
  const known = new Set(
    config.commands.flatMap((command) => [
      command.id.split(":")[0],
      ...(command.aliases ?? []).map((alias) => alias.split(":")[0]),
    ]),
  );
  return !known.has(requested);
}

/**
 * Append the root usage to an unknown-command error (FR-005-AC-1).
 *
 * The hand-rolled dispatcher FR-026 replaced used to print usage on an unknown
 * command; the oclif runner reports only `command <x> not found`, which names
 * the mistake but not the remedy (NFR-003). This restores the remedy without
 * reintroducing a dispatcher.
 */
export function withRootUsage(error: unknown, usage: string): unknown {
  if (!(error instanceof Error)) return error;
  error.message = `${error.message}\n\n${usage}`;
  return error;
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
  const config = await loadConfig(options);
  try {
    await run(argv, config);
  } catch (error) {
    // The error still propagates (FR-026-AC-6); only an unknown command's
    // message gains the root usage FR-005 requires.
    if (isUnknownCommand(argv, config)) {
      throw withRootUsage(error, rootUsage(config));
    }
    throw error;
  }
}
