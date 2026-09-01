import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  produceGitHubReleaseOperational,
  type GitHubReleaseProducerDefinition,
} from "../../measurement/index.js";

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
    try {
      const definition = JSON.parse(
        readFileSync(flags.definition, "utf8"),
      ) as GitHubReleaseProducerDefinition;
      this.log(produceGitHubReleaseOperational(flags.repo, definition).path);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.error(detail, { exit: 2 });
    }
  }
}
