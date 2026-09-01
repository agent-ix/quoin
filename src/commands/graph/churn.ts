import { QuoinCommand } from "../../base.js";
import { analyzeChurn } from "../../graph-analysis/index.js";
import { graphInputFlags, graphOutput, loadGraphFlags } from "./common.js";

export default class GraphChurn extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary = "Report retained obligation reaffirmation history.";
  static flags = graphInputFlags;

  async run(): Promise<void> {
    const { flags } = await this.parse(GraphChurn);
    const loaded = loadGraphFlags(flags);
    if (!loaded.ok) this.error(loaded.error.message, { exit: 2 });
    this.log(graphOutput(analyzeChurn(loaded.value), flags.json));
  }
}
