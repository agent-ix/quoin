import { QuoinCommand } from "../../base.js";
import { askGraph, graphInputFlags, graphRequest } from "./common.js";

export default class GraphChurn extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary = "Report retained obligation reaffirmation history.";
  static flags = graphInputFlags;

  async run(): Promise<void> {
    const { flags } = await this.parse(GraphChurn);
    this.log(
      askGraph("graph.churn", graphRequest(flags), (message) =>
        this.error(message, { exit: 2 }),
      ),
    );
  }
}
