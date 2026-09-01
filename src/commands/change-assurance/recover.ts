import { QuoinCommand } from "../../base.js";
import { recoverChangeAssuranceStaging } from "../../change-assurance/index.js";
import { canonicalOutput, jsonFlag, messageOf, repoFlag } from "./common.js";

export default class ChangeAssuranceRecover extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary = "Remove staging directories left by an interrupted intake.";
  static description = `Intake makes an attestation and its output visible with a single rename. If it
is interrupted before that rename, the half-written pair stays in a staging
directory that is never read as an attestation — invisible, but not free.

This removes exactly those staging directories and reports how many it removed.
Retained records, attestations, and outputs are left untouched, and nothing
here re-runs, re-hashes, or re-verifies anything.`;

  static examples = ["quoin change-assurance recover --repo . --json"];

  static flags = {
    repo: repoFlag,
    json: jsonFlag,
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(ChangeAssuranceRecover);

    let removed: number;
    try {
      removed = recoverChangeAssuranceStaging(flags.repo);
    } catch (error) {
      this.error(`cannot recover staging: ${messageOf(error)}`, { exit: 2 });
    }

    if (flags.json) {
      this.log(canonicalOutput({ removed }));
      return;
    }
    this.log(
      `removed ${removed} interrupted intake staging director${removed === 1 ? "y" : "ies"}`,
    );
  }
}
