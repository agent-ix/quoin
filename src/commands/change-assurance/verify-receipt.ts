import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  verifyReceipt,
  type VerificationReceipt,
} from "../../change-assurance/index.js";
import {
  canonicalOutput,
  jsonFlag,
  messageOf,
  readInputJson,
} from "./common.js";

export default class ChangeAssuranceVerifyReceipt extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary = "Re-verify a sealed verification receipt.";
  static description = `Checks that a receipt still has the shape FR-065 declares and still hashes to
its own digest, so an edited outcome, reason, proof row, or check is refused
rather than read back as fact.

This re-verifies the receipt document. It does not re-run verification: the
underlying record, attestations, and decisions are not consulted, and no proof
command is executed.

Exit status is 0 when the verified receipt is \`valid\`, 1 when it is
\`invalid\` or \`incomplete\`, and 2 when the document itself is refused.`;

  static examples = [
    "quoin change-assurance verify-receipt --input receipt.json --json",
  ];

  static flags = {
    input: Flags.string({
      description: "Sealed verification receipt JSON. `-` reads stdin.",
      required: true,
    }),
    json: jsonFlag,
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(ChangeAssuranceVerifyReceipt);

    let body: unknown;
    try {
      body = readInputJson(flags.input);
    } catch (error) {
      this.error(`cannot read --input ${flags.input}: ${messageOf(error)}`, {
        exit: 2,
      });
    }

    let receipt: VerificationReceipt;
    try {
      receipt = verifyReceipt(body);
    } catch (error) {
      this.error(`receipt refused: ${messageOf(error)}`, { exit: 2 });
    }

    if (flags.json) {
      this.log(
        canonicalOutput({
          digest: receipt.digest,
          record_digest: receipt.record_digest,
          candidate_revision: receipt.candidate_revision,
          outcome: receipt.outcome,
          reasons: receipt.reasons,
        }),
      );
    } else {
      this.log(`receipt ${receipt.digest} verified`);
      this.log(`  outcome: ${receipt.outcome}`);
      if (receipt.reasons.length > 0) {
        this.log(`  reasons: ${receipt.reasons.join(", ")}`);
      }
    }

    if (receipt.outcome !== "valid") this.exit(1);
  }
}
