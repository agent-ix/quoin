import { run, type RunnerLoadOptions } from "@agent-ix/ix-cli-core";

import { packageVersion } from "./version.js";

export { packageVersion, resolveVersion } from "./version.js";

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
  await run(argv, options);
}
