import { FlowCommand } from "../flow-command.js";

export default class ToPlan extends FlowCommand {
  static summary = "Convert accepted requirements into an implementation plan.";
  static description = `Starts the bundled to-plan workflow run in ~/.ix/flows. Use ix-flow to inspect,
resume, advance phases, and acknowledge human gates.`;

  static examples = ["quoin to-plan --target spec/", "ix-flow status <run-id>"];

  static flags = FlowCommand.flowFlags;

  protected readonly flowName = "to-plan";

  async run(): Promise<void> {
    const { flags } = await this.parse(ToPlan);
    await this.launch(flags);
  }
}
