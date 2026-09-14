import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { askCore, stringMember } from "./core.js";

export default class OperationalRelease extends QuoinCommand {
  static summary =
    "Record operational evidence from retained GitHub release exports.";
  static description = `Consumes retained workflow and API files only. It performs no
network request, workflow dispatch, release publication, or process execution.`;
  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    definition: Flags.string({
      description: "Versioned release-producer definition JSON path.",
      required: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(OperationalRelease);
    const op = "measurement.produce_github_release_operational";
    const fail = (message: string): never => this.error(message, { exit: 2 });
    let definition: unknown;
    try {
      definition = JSON.parse(
        readFileSync(flags.definition, "utf8"),
      ) as unknown;
    } catch (error) {
      fail(error instanceof Error ? error.message : String(error));
    }
    const payload = askCore(op, { repo: flags.repo, definition }, fail);
    this.log(stringMember(payload, "path", op, fail));
  }
}
