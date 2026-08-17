import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { gc } from "../../evidence/index.js";

export default class EvidenceGc extends QuoinCommand {
  static summary = "Drop run records nothing references.";
  static description = `Retention policy: the latest run per suite, plus anything a binding points at.

Dir-per-suite is what makes this local — a policy change for one expensive suite
(mutation, DAST, fuzz) touches one directory, and a .gitignore decision can be
made per suite rather than for the whole store.`;

  static examples = ["quoin evidence gc --dry-run", "quoin evidence gc"];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    "dry-run": Flags.boolean({
      description: "List what would be deleted without deleting it.",
    }),
    json: Flags.boolean({ description: "Emit the deleted paths as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceGc);
    const deleted = gc(flags.repo, flags["dry-run"]);
    if (flags.json) {
      this.log(JSON.stringify({ deleted, dryRun: flags["dry-run"] }, null, 2));
      return;
    }
    if (deleted.length === 0) {
      this.log("nothing to collect");
      return;
    }
    for (const path of deleted) {
      this.log(`${flags["dry-run"] ? "would delete" : "deleted"} ${path}`);
    }
  }
}
