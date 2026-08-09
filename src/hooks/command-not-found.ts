import { Errors, type Hook } from "@oclif/core";

import { rootUsage } from "../cli.js";

/**
 * `command_not_found` hook — attach the root usage to an unknown command
 * (FR-005-AC-1).
 *
 * The hand-rolled dispatcher FR-026 replaced printed usage on an unknown
 * command. The oclif runner reports only `command <x> not found`, which names
 * the mistake but not the remedy, leaving NFR-003 unmet on the one path a
 * mistyping user is guaranteed to hit.
 *
 * This is oclif's own extension point for that: `Config.runCommand` invokes it
 * when no command matches, and rethrows a hook failure in place of its default
 * error. So the message is fixed where oclif expects it to be fixed — no error
 * sniffing (the thrown `CLIError` carries no stable discriminator), and no
 * bypassing `execute`, whose `flush()` the shipped bin needs so piped output is
 * not truncated on exit.
 *
 * It fires on the library path too: `run(argv, config)` reaches the same
 * `Config.runCommand`.
 */
const hook: Hook<"command_not_found"> = async function (options) {
  throw new Errors.CLIError(
    `command ${options.id} not found\n\n${rootUsage(this.config)}`,
    { exit: 2 },
  );
};

export default hook;
