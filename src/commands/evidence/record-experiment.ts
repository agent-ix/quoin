import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { recordExperiment, recordId } from "../../core/evidence.js";

export default class EvidenceRecordExperiment extends QuoinCommand {
  static summary = "Publish one content-addressed experiment record.";
  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    input: Flags.string({
      description: "experiment-record-v1 JSON path, or - for stdin.",
      required: true,
    }),
    json: Flags.boolean({ description: "Emit the stored result as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceRecordExperiment);
    try {
      const text =
        flags.input === "-"
          ? readFileSync(0, "utf8")
          : readFileSync(flags.input, "utf8");
      const stored = recordExperiment(flags.repo, JSON.parse(text) as unknown);
      this.log(
        flags.json
          ? JSON.stringify(stored, null, 2)
          : `${recordId(stored.record)} ${stored.created ? "created" : "exists"} ${stored.path}`,
      );
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.error(detail, { exit: 2 });
    }
  }
}
