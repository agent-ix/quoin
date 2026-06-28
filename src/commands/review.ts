import { FlowCommand } from "../flow-command.js";

export default class Review extends FlowCommand {
  static summary = "Run a composite spec review workflow.";
  static description = `Starts the bundled review workflow run in ~/.ix/flows. Use ix-flow to inspect,
resume, advance phases, and acknowledge human gates.`;

  static examples = ["quoin review --target spec/", "ix-flow status <run-id>"];

  static flags = FlowCommand.flowFlags;

  protected readonly flowName = "review";

  async run(): Promise<void> {
    const { flags } = await this.parse(Review);
    await this.launch(flags);
  }
}
