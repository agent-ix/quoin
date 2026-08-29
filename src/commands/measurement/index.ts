import { QuoinCommand } from "../../base.js";

export default class MeasurementIndex extends QuoinCommand {
  static summary = "Record and inspect versioned QA measurements.";
  static description = `Measurement collections connect raw producer output to active
MeasurementPlans. Use \`quoin measurement record\` to persist one complete producer
invocation; use \`quoin report\` for current state, comparisons, and series.`;

  async run(): Promise<void> {
    // The app does not install @oclif/plugin-help. Delegating to that missing
    // optional plugin made this successful command emit repeated load errors.
    await this.parse(MeasurementIndex);
    this.log(MeasurementIndex.description);
  }
}
