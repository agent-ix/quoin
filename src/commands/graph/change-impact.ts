import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { askGraph, graphInputFlags, graphRequest } from "./common.js";

export default class GraphChangeImpact extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary =
    "Report reverse dependency exposure from requirement changes.";
  static flags = {
    ...graphInputFlags,
    requirement: Flags.string({
      description: "Changed requirement id. Repeatable.",
      multiple: true,
      required: true,
    }),
    relation: Flags.string({
      description:
        "Relationship kind replacing the default selection. Repeatable.",
      multiple: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(GraphChangeImpact);
    this.log(
      askGraph(
        "graph.change_impact",
        {
          ...graphRequest(flags),
          requirements: flags.requirement,
          // Absent and empty are different walks — the eight defaults, and no
          // edges at all — so an unsupplied `--relation` stays absent on the
          // wire rather than being normalised to `[]`.
          ...(flags.relation === undefined
            ? {}
            : { relations: flags.relation }),
        },
        (message) => this.error(message, { exit: 2 }),
      ),
    );
  }
}
