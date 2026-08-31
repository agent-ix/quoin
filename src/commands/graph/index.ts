import { QuoinCommand } from "../../base.js";

export default class Graph extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary = "Read-only evidence-graph analysis views.";
  static description = `Analyze an existing, accepted Quire assurance export together with retained
Quoin evidence and an existing FR-032 audit. These commands run no producer,
suite, Quire, Git, or network operation and write nothing.

Subcommands:
  quoin graph fan-out
  quoin graph change-impact
  quoin graph churn`;

  async run(): Promise<void> {
    await this.parse(Graph);
    this.log(Graph.description);
  }
}
