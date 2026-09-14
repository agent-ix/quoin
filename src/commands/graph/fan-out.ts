import { QuoinCommand } from "../../base.js";
import { askGraph, graphInputFlags, graphRequest } from "./common.js";

export default class GraphFanOut extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary = "Report distinct live obligations per evidence suite.";
  static flags = graphInputFlags;

  async run(): Promise<void> {
    const { flags } = await this.parse(GraphFanOut);
    this.log(
      askGraph("graph.fan_out", graphRequest(flags), (message) =>
        this.error(message, { exit: 2 }),
      ),
    );
  }
}
