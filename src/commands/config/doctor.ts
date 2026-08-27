import { runConfigDoctor } from "@agent-ix/ix-cli-core";

import { QuoinCommand } from "../../base.js";
import { registerQuoinSchema } from "../../config-schema.js";

export default class ConfigDoctor extends QuoinCommand {
  static summary = "Report config files that failed to parse or validate.";
  static description = `A malformed config never blocks a command -- schema defaults are used and the
problem is recorded instead. This is where those records surface.`;

  async run(): Promise<void> {
    await this.parse(ConfigDoctor);
    registerQuoinSchema();
    // Adopt the handler's code unconditionally: a doctor run's result is the
    // invocation's result, and assigning 0 on a clean run is what the process
    // would exit with anyway. Guarding the assignment would add a branch whose
    // only purpose is to skip a no-op.
    const { exitCode } = await runConfigDoctor();
    process.exitCode = exitCode;
  }
}
