import { FlowCommand } from "../flow-command.js";

export default class Matrix extends FlowCommand {
  static summary = "Build or update a requirements test matrix.";
  static description = `Starts the bundled matrix workflow run in ~/.ix/flows. Use ix-flow to inspect,
resume, advance phases, and acknowledge human gates.`;

  static examples = ["quoin matrix --target spec/", "ix-flow status <run-id>"];

  static flags = FlowCommand.flowFlags;

  protected readonly flowName = "matrix";

  async run(): Promise<void> {
    const { flags } = await this.parse(Matrix);
    await this.launch(flags);
  }
}
