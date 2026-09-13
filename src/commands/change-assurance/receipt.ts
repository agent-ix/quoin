import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import type { ReceiptPayload } from "../../core/types.js";
import {
  askCore,
  canonicalOutput,
  hexOf,
  jsonFlag,
  messageOf,
  parseSelection,
  readInputBytes,
  repoFlag,
} from "./common.js";

/** The members of a verification receipt this command reports. */
interface Receipt {
  digest: string;
  outcome: string;
  reasons: string[];
  proofs: Array<{ proof_id: string; outcome: string; reasons: string[] }>;
}

export default class ChangeAssuranceReceipt extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary =
    "Verify a candidate from retained inputs and emit its receipt.";
  static description = `Assembles the FR-065 verification input from what the caller names — the
stored record, its parents, the explicitly selected attestations and their
retained outputs, the retained ix-flow decision history, and any retained
FR-032 audit reports — and emits the resulting receipt.

Nothing is discovered. A stored attestation that is not named by --select has
no effect on the receipt, so a stray upload cannot discharge a proof.

No proof, suite, tool, workflow, Git, or network operation runs, and nothing is
written. Missing evidence stays missing: \`unavailable\`, \`not_computed\`, and
absent attestations remain their own outcomes and reasons and are never turned
into a pass or a failure.

Exit status is 0 for a \`valid\` receipt and 1 for an \`invalid\` or
\`incomplete\` one. The receipt is emitted either way; a usage, parse, or
integrity error exits 2 instead and emits none.`;

  static examples = [
    "quoin change-assurance receipt --record <record-digest> --candidate-revision <sha> --select PROOF-1=<attestation-digest> --decisions decisions.json --json",
  ];

  static flags = {
    record: Flags.string({
      description: "Digest of the stored change-assurance record to verify.",
      required: true,
    }),
    "candidate-revision": Flags.string({
      description:
        "Candidate revision the selected attestations must be bound to.",
      required: true,
    }),
    parent: Flags.string({
      description:
        "Digest of a stored parent record, repeated for each earlier " +
        "revision. Named rather than walked, so a missing ancestor is a " +
        "stated gap instead of a silently shortened chain.",
      multiple: true,
      default: [],
    }),
    select: Flags.string({
      description:
        "Explicit `<proof-id>=<attestation-digest>` selection, repeated per " +
        "proof. Only selected attestations are read.",
      multiple: true,
      default: [],
    }),
    decisions: Flags.string({
      description:
        "Retained ix-flow decision history JSON (`run_id` plus its exact " +
        "event chain). `-` reads stdin.",
      required: true,
    }),
    audits: Flags.string({
      description:
        "Retained FR-032 audit reports as a JSON array. Absent means no " +
        "audit was retained, which stays distinct from an audit with no " +
        "findings.",
    }),
    repo: repoFlag,
    json: jsonFlag,
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(ChangeAssuranceReceipt);

    // The `<proof-id>=<digest>` spelling is this surface's own grammar, so it
    // is refused here, naming the flag the user typed. Everything the grammar
    // produces is then decided on the far side: whether the digests name
    // retained evidence, and what the receipt says.
    const selections: Array<{ proof_id: string; attestation_digest: string }> =
      [];
    for (const raw of flags.select) {
      const selection = parseSelection(raw);
      if (!selection) {
        this.error(
          `--select ${raw} must be <proof-id>=<64-character lowercase hex digest>`,
          { exit: 2 },
        );
      }
      selections.push(selection);
    }

    let decisions: Uint8Array;
    try {
      decisions = readInputBytes(flags.decisions);
    } catch (error) {
      this.error(
        `cannot read --decisions ${flags.decisions}: ${messageOf(error)}`,
        { exit: 2 },
      );
    }

    let audits: Uint8Array | null = null;
    if (flags.audits !== undefined) {
      try {
        audits = readInputBytes(flags.audits);
      } catch (error) {
        this.error(
          `cannot read --audits ${flags.audits}: ${messageOf(error)}`,
          { exit: 2 },
        );
      }
    }

    const payload = askCore(
      "change_assurance.receipt",
      {
        repo: flags.repo,
        record_digest: flags.record,
        candidate_revision: flags["candidate-revision"],
        parent_digests: flags.parent,
        selections,
        decisions_hex: hexOf(decisions),
        audits_hex: audits === null ? null : hexOf(audits),
      },
      "cannot verify candidate",
      (message) => this.error(message, { exit: 2 }),
    ) as unknown as ReceiptPayload;
    const receipt = payload.receipt as Receipt;

    if (flags.json) {
      this.log(canonicalOutput(receipt));
    } else {
      this.log(`receipt ${receipt.digest}`);
      this.log(`  outcome: ${receipt.outcome}`);
      if (receipt.reasons.length > 0) {
        // Named rather than counted: a reason is the thing somebody has to act
        // on, and "3 reasons" tells them nothing about which.
        this.log(`  reasons: ${receipt.reasons.join(", ")}`);
      }
      for (const proof of receipt.proofs) {
        this.log(
          `  ${proof.proof_id}: ${proof.outcome}` +
            (proof.reasons.length > 0 ? ` (${proof.reasons.join(", ")})` : ""),
        );
      }
    }

    // An incomplete receipt is not a pass and not a failure of this command:
    // it is the honest state of the evidence, and it exits non-zero so a gate
    // cannot mistake "nothing was retained" for "everything checked out".
    if (receipt.outcome !== "valid") this.exit(1);
  }
}
