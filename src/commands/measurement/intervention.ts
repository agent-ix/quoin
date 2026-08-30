import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  produceAgentEvalIntervention,
  type AgentEvalInterventionDefinition,
} from "../../measurement/index.js";

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
    try {
      const definition = JSON.parse(
        readFileSync(flags.definition, "utf8"),
      ) as AgentEvalInterventionDefinition;
      this.log(produceAgentEvalIntervention(flags.repo, definition).path);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.error(detail, { exit: 2 });
    }
  }
}
