import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import type { SealRecordPayload } from "../../core/types.js";
import {
  askCore,
  canonicalOutput,
  hexOf,
  jsonFlag,
  messageOf,
  readInputBytes,
  repoFlag,
} from "./common.js";

/** The three members of a sealed record this command prints. */
interface SealedRecord {
  record_id: string;
  revision: number;
  digest: string;
}

export default class ChangeAssuranceSealRecord extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary = "Seal an explicit change-assurance record and retain it.";
  static description = `Seals ONE reviewed change definition the caller supplies in full (FR-063) and
retains it under its own digest.

The body is the record without its \`digest\`: the digest is computed from the
canonical bytes of everything else, so supplying one is refused rather than
overwritten. Nothing in the body is inferred, defaulted, or discovered — a
field the caller did not state is a validation failure, not a blank.

This command runs no producer and reads no repository state beyond the store
it writes into.`;

  static examples = [
    "quoin change-assurance seal-record --input record.json --json",
    "cat record.json | quoin change-assurance seal-record --input -",
  ];

  static flags = {
    input: Flags.string({
      description:
        "Change-assurance record body as JSON, without `digest`. `-` reads stdin.",
      required: true,
    }),
    repo: repoFlag,
    json: jsonFlag,
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(ChangeAssuranceSealRecord);

    let body: Uint8Array;
    try {
      body = readInputBytes(flags.input);
    } catch (error) {
      this.error(`cannot read --input ${flags.input}: ${messageOf(error)}`, {
        exit: 2,
      });
    }

    // The bytes cross unparsed. Sealing, schema validation, and the refusal of
    // a supplied `digest` are all decided on the far side (FR-096), over the
    // bytes the producer wrote.
    const payload = askCore(
      "change_assurance.seal_record",
      { repo: flags.repo, record_hex: hexOf(body) },
      "cannot seal record",
      (message) => this.error(message, { exit: 2 }),
    ) as unknown as SealRecordPayload;
    const record = payload.record as SealedRecord;

    if (flags.json) {
      this.log(
        canonicalOutput({
          record_id: record.record_id,
          revision: record.revision,
          digest: record.digest,
          path: payload.path,
        }),
      );
      return;
    }
    this.log(`sealed ${record.record_id} revision ${record.revision}`);
    this.log(`  digest: ${record.digest}`);
    this.log(`  retained: ${payload.path}`);
  }
}
