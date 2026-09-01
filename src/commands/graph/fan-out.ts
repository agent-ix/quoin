import { QuoinCommand } from "../../base.js";
import { analyzeFanOut } from "../../graph-analysis/index.js";
import { graphInputFlags, graphOutput, loadGraphFlags } from "./common.js";

export default class GraphFanOut extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary = "Report distinct live obligations per evidence suite.";
  static flags = graphInputFlags;

  async run(): Promise<void> {
    const { flags } = await this.parse(GraphFanOut);
    const loaded = loadGraphFlags(flags);
    if (!loaded.ok) this.error(loaded.error.message, { exit: 2 });
    this.log(graphOutput(analyzeFanOut(loaded.value), flags.json));
  }
}
