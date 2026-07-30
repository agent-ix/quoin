import { Flags } from "@oclif/core";

import { QuoinCommand } from "./base.js";
import { startSpecFlow } from "./flows.js";

/**
 * Base for the bundled spec-workflow launchers (`review`, `matrix`, `to-plan`).
 *
 * quoin creates the workflow run in ~/.ix/flows by spawning `ix-flow` (see
 * {@link startSpecFlow}); ix-flow then inspects, resumes, advances phases, and
 * acknowledges human gates. The subprocess delegation is unchanged from the
 * legacy dispatcher — the launcher only translates flags into the ix-flow argv.
 */
export abstract class FlowCommand extends QuoinCommand {
  /** Flags shared by every flow launcher; concrete commands reuse this set. */
  static flowFlags = {
    target: Flags.string({
      description: "Spec target ref to feed the workflow (repeatable).",
      multiple: true,
    }),
    json: Flags.boolean({ description: "Forward --json to ix-flow." }),
    id: Flags.string({ description: "Explicit workflow run id." }),
  };

  /** ix-flow workflow name; equals the command id (review/matrix/to-plan). */
  protected abstract readonly flowName: string;

  protected async launch(flags: {
    target?: string[];
    json?: boolean;
    id?: string;
  }): Promise<void> {
    await startSpecFlow(this.flowName, {
      json: flags.json === true,
      ...(flags.id ? { id: flags.id } : {}),
      ...(flags.target ? { target: flags.target } : {}),
    });
  }
}
