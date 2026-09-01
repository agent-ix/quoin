import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  CHANGE_ASSURANCE_SCHEMA_NAMES,
  changeAssuranceSchemaPath,
  type ChangeAssuranceSchemaName,
} from "../../change-assurance/index.js";
import { canonicalOutput, jsonFlag, messageOf } from "./common.js";

export default class ChangeAssuranceSchema extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary = "Emit a packaged change-assurance JSON Schema asset.";
  static description = `Prints one of the three normative, versioned schema assets shipped with this
build, byte-for-byte as packaged, so a consumer validates against the same file
the sealing and verification code was written against rather than a copy that
has drifted.

Without --name the three asset names are listed. An unknown name is refused.`;

  static examples = [
    "quoin change-assurance schema",
    "quoin change-assurance schema --name proof-attestation-v1.schema.json",
  ];

  static flags = {
    name: Flags.string({
      description: "Schema asset to emit.",
      options: [...CHANGE_ASSURANCE_SCHEMA_NAMES],
    }),
    json: jsonFlag,
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(ChangeAssuranceSchema);

    if (flags.name === undefined) {
      if (flags.json) {
        this.log(
          canonicalOutput({ schemas: [...CHANGE_ASSURANCE_SCHEMA_NAMES] }),
        );
        return;
      }
      for (const name of CHANGE_ASSURANCE_SCHEMA_NAMES) this.log(name);
      return;
    }

    const name = flags.name as ChangeAssuranceSchemaName;
    let text: string;
    try {
      text = readFileSync(changeAssuranceSchemaPath(name), "utf8");
    } catch (error) {
      this.error(`cannot read schema ${name}: ${messageOf(error)}`, {
        exit: 2,
      });
    }
    // Emitted verbatim: a re-serialization would be a different file from the
    // one this build validates against, which is the whole point of the flag.
    this.log(text.replace(/\n$/, ""));
  }
}
