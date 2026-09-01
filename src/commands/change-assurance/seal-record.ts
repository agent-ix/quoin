import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  sealChangeRecord,
  writeChangeRecord,
  type ChangeAssuranceRecord,
} from "../../change-assurance/index.js";
import {
  canonicalOutput,
  jsonFlag,
  messageOf,
  readInputJson,
  refuseSuppliedFields,
  repoFlag,
} from "./common.js";

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

    let body: unknown;
    try {
      body = readInputJson(flags.input);
    } catch (error) {
      this.error(`cannot read --input ${flags.input}: ${messageOf(error)}`, {
        exit: 2,
      });
    }

    const supplied = refuseSuppliedFields(body, ["digest"]);
    if (supplied) this.error(supplied, { exit: 2 });

    let record: ChangeAssuranceRecord;
    try {
      record = sealChangeRecord(body as Omit<ChangeAssuranceRecord, "digest">);
    } catch (error) {
      this.error(`cannot seal record: ${messageOf(error)}`, { exit: 2 });
    }

    let path: string;
    try {
      path = writeChangeRecord(flags.repo, record);
    } catch (error) {
      this.error(`cannot retain record: ${messageOf(error)}`, { exit: 2 });
    }

    if (flags.json) {
      this.log(
        canonicalOutput({
          record_id: record.record_id,
          revision: record.revision,
          digest: record.digest,
          path,
        }),
      );
      return;
    }
    this.log(`sealed ${record.record_id} revision ${record.revision}`);
    this.log(`  digest: ${record.digest}`);
    this.log(`  retained: ${path}`);
  }
}
