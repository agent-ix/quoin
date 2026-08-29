import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { writeMeasurementCollection } from "../../measurement/index.js";

export default class MeasurementRecord extends QuoinCommand {
  static summary = "Atomically record one plan-validated producer invocation.";
  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    input: Flags.string({
      description: "Collection JSON path, or - for stdin.",
      required: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(MeasurementRecord);
    const text =
      flags.input === "-"
        ? readFileSync(0, "utf8")
        : readFileSync(flags.input, "utf8");
    let value: unknown;
    try {
      value = JSON.parse(text) as unknown;
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.error(`measurement input is not JSON: ${detail}`, { exit: 2 });
    }
    try {
      this.log(writeMeasurementCollection(flags.repo, value));
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.error(detail, { exit: 2 });
    }
  }
}
