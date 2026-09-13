import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import type { IntakePayload } from "../../core/types.js";
import {
  askCore,
  canonicalOutput,
  hexOf,
  jsonFlag,
  messageOf,
  readInputBytes,
  repoFlag,
} from "./common.js";

export default class ChangeAssuranceIntake extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary =
    "Retain an exact sealed attestation and its exact output bytes.";
  static description = `Verifies ONE sealed attestation against the bytes it claims and retains the
pair atomically: either both the attestation JSON and the output file become
visible together, or neither does.

The retained attestation is the canonical form of what was supplied and the
retained output is byte-for-byte the file named by --output. A recorded digest
or size that contradicts those bytes is refused and nothing is retained.

Re-running with byte-identical inputs succeeds and retains nothing new. The
same digest with different bytes is a collision and is refused.

An interrupted intake leaves an invisible staging directory that is never read
as an attestation; \`quoin change-assurance recover\` removes it.`;

  static examples = [
    "quoin change-assurance intake --attestation attestation.json --output junit.xml --json",
  ];

  static flags = {
    attestation: Flags.string({
      description: "Exact sealed attestation JSON. `-` reads stdin.",
      required: true,
    }),
    output: Flags.string({
      description:
        "Exact retained result file the attestation names. Its bytes are " +
        "retained and re-hashed; it is never executed.",
      required: true,
    }),
    repo: repoFlag,
    json: jsonFlag,
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(ChangeAssuranceIntake);

    let raw: Uint8Array;
    try {
      raw = readInputBytes(flags.attestation);
    } catch (error) {
      this.error(
        `cannot read --attestation ${flags.attestation}: ${messageOf(error)}`,
        { exit: 2 },
      );
    }

    let output: Uint8Array;
    try {
      output = readInputBytes(flags.output);
    } catch (error) {
      this.error(`cannot read --output ${flags.output}: ${messageOf(error)}`, {
        exit: 2,
      });
    }

    const payload = askCore(
      "change_assurance.intake",
      {
        repo: flags.repo,
        attestation_hex: hexOf(raw),
        output_hex: hexOf(output),
      },
      "cannot retain attestation",
      (message) => this.error(message, { exit: 2 }),
    ) as unknown as IntakePayload;

    if (flags.json) {
      this.log(
        canonicalOutput({
          directory: payload.directory,
          size_bytes: payload.size_bytes,
        }),
      );
      return;
    }
    this.log(`retained attestation → ${payload.directory}`);
    this.log(`  output bytes: ${payload.size_bytes}`);
  }
}
