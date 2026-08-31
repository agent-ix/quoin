import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { analyzeChangeImpact } from "../../graph-analysis/index.js";
import { graphInputFlags, graphOutput, loadGraphFlags } from "./common.js";

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
    const loaded = loadGraphFlags(flags);
    if (!loaded.ok) this.error(loaded.error.message, { exit: 2 });
    const report = analyzeChangeImpact(
      loaded.value,
      flags.requirement,
      flags.relation,
    );
    this.log(graphOutput(report, flags.json));
  }
}
