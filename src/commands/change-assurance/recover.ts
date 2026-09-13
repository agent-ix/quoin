import { QuoinCommand } from "../../base.js";
import type { RecoverPayload } from "../../core/types.js";
import { askCore, canonicalOutput, jsonFlag, repoFlag } from "./common.js";

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

    const payload = askCore(
      "change_assurance.recover",
      { repo: flags.repo },
      "cannot recover staging",
      (message) => this.error(message, { exit: 2 }),
    ) as unknown as RecoverPayload;
    const removed = payload.removed;

    if (flags.json) {
      this.log(canonicalOutput({ removed }));
      return;
    }
    this.log(
      `removed ${removed} interrupted intake staging director${removed === 1 ? "y" : "ies"}`,
    );
  }
}
