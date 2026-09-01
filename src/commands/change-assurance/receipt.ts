import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  readAttestation,
  readChangeRecord,
  verifyChangeAssurance,
  type ChangeAssuranceRecord,
  type DecisionHistory,
  type ProofAttestation,
  type RetainedAuditInput,
  type VerificationReceipt,
} from "../../change-assurance/index.js";
import {
  canonicalOutput,
  jsonFlag,
  messageOf,
  parseSelection,
  readInputJson,
  repoFlag,
} from "./common.js";

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

    const record = this.storedRecord(flags.repo, flags.record, "--record");
    const parents = flags.parent.map((digest) =>
      this.storedRecord(flags.repo, digest, "--parent"),
    );

    const selections: Array<{ proof_id: string; attestation_digest: string }> =
      [];
    const attestations: Array<{
      attestation: ProofAttestation;
      output: Uint8Array | null;
    }> = [];
    for (const raw of flags.select) {
      const selection = parseSelection(raw);
      if (!selection) {
        this.error(
          `--select ${raw} must be <proof-id>=<64-character lowercase hex digest>`,
          { exit: 2 },
        );
      }
      selections.push(selection);
      let stored: { attestation: ProofAttestation; output: Uint8Array } | null;
      try {
        stored = readAttestation(flags.repo, selection.attestation_digest);
      } catch (error) {
        this.error(
          `cannot read attestation ${selection.attestation_digest}: ${messageOf(error)}`,
          { exit: 2 },
        );
      }
      if (!stored) {
        this.error(
          `--select ${raw} names no retained attestation in ${flags.repo}`,
          { exit: 2 },
        );
      }
      attestations.push(stored);
    }

    let decisionHistory: DecisionHistory;
    try {
      decisionHistory = readInputJson(flags.decisions) as DecisionHistory;
    } catch (error) {
      this.error(
        `cannot read --decisions ${flags.decisions}: ${messageOf(error)}`,
        { exit: 2 },
      );
    }

    let audits: RetainedAuditInput[] = [];
    if (flags.audits !== undefined) {
      let parsed: unknown;
      try {
        parsed = readInputJson(flags.audits);
      } catch (error) {
        this.error(
          `cannot read --audits ${flags.audits}: ${messageOf(error)}`,
          { exit: 2 },
        );
      }
      if (!Array.isArray(parsed)) {
        this.error("--audits must be a JSON array of retained audit reports", {
          exit: 2,
        });
      }
      audits = parsed as RetainedAuditInput[];
    }

    let receipt: VerificationReceipt;
    try {
      receipt = verifyChangeAssurance({
        record,
        parents,
        candidate_revision: flags["candidate-revision"],
        selections,
        attestations,
        decision_history: decisionHistory,
        audits,
      });
    } catch (error) {
      this.error(`cannot verify candidate: ${messageOf(error)}`, { exit: 2 });
    }

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

  /** Read one stored record, refusing an unknown digest rather than skipping it. */
  private storedRecord(
    repo: string,
    digest: string,
    flag: string,
  ): ChangeAssuranceRecord {
    let record: ChangeAssuranceRecord | null;
    try {
      record = readChangeRecord(repo, digest);
    } catch (error) {
      this.error(`cannot read ${flag} ${digest}: ${messageOf(error)}`, {
        exit: 2,
      });
    }
    if (!record) {
      this.error(`${flag} ${digest} names no retained record in ${repo}`, {
        exit: 2,
      });
    }
    return record;
  }
}
