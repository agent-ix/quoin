import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { askCore, stringMember } from "./core.js";

export default class MeasurementIntervention extends QuoinCommand {
  static summary =
    "Record an intervention from two retained agent-eval reports.";
  static description = `Consumes retained reports only. It never invokes an agent,
evaluation harness, producer process, or network client.`;
  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    definition: Flags.string({
      description: "Versioned producer-definition JSON path.",
      required: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(MeasurementIntervention);
    const op = "measurement.produce_agent_eval_intervention";
    const fail = (message: string): never => this.error(message, { exit: 2 });
    let definition: unknown;
    try {
      definition = JSON.parse(
        readFileSync(flags.definition, "utf8"),
      ) as unknown;
    } catch (error) {
      fail(error instanceof Error ? error.message : String(error));
    }
    const payload = askCore(op, { repo: flags.repo, definition }, fail);
    this.log(stringMember(payload, "path", op, fail));
  }
}
