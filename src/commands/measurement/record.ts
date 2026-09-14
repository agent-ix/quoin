import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { askCore, stringMember } from "./core.js";

/**
 * Which of the three intakes accepts the candidate is no longer decided here:
 * `measurement.record` reads the document's own `record_type` and publishes it
 * through the intake that member names (quoin#478). The branch moved with the
 * operation rather than being copied on both sides of the boundary.
 */
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
    const op = "measurement.record";
    const fail = (message: string): never => this.error(message, { exit: 2 });
    const payload = askCore(op, { repo: flags.repo, record: value }, fail);
    this.log(stringMember(payload, "path", op, fail));
  }
}
