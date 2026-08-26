import { Command } from "@oclif/core";

export default class MeasurementIndex extends Command {
  static summary = "Record and inspect versioned QA measurements.";
  static description = `Measurement collections connect raw producer output to active
MeasurementPlans. Use \`quoin measurement record\` to persist one complete producer
invocation; use \`quoin report\` for current state, comparisons, and series.`;

  async run(): Promise<void> {
    await this.config.runCommand("help", ["measurement"]);
  }
}
