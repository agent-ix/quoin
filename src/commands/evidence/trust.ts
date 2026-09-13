import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { trustDecision } from "../../core/evidence.js";

export default class EvidenceTrust extends QuoinCommand {
  static summary = "Record a use-specific evidence-producer trust decision.";
  static description = `Transcribes an accountable decision from JSON and reports its effective state.
The decision applies to one producer use, not to the tool globally. Accepted and observed
contexts are compared using declared revalidation triggers; mismatch is recorded and shown
as invalidated rather than silently falling back to trusted.`;
  static examples = [
    "quoin evidence trust --decision trust-decision.json",
    "quoin evidence trust --decision trust-decision.json --json",
  ];
  static flags = {
    decision: Flags.string({
      description: "Completed trust-decision JSON; `-` reads stdin.",
      required: true,
    }),
    repo: Flags.string({ description: "Repository root.", default: "." }),
    json: Flags.boolean({ description: "Emit the assessment as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceTrust);
    const raw =
      flags.decision === "-"
        ? readFileSync(0, "utf8")
        : readFileSync(flags.decision, "utf8");
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw) as unknown;
    } catch (cause) {
      this.error(`trust decision is not JSON: ${(cause as Error).message}`, {
        exit: 2,
      });
    }
    // Validated, assessed and stored in one call, and in that order: a
    // decision the boundary refuses must never reach the store, because a
    // store that held one could render it as `accepted`.
    let path: string;
    let assessment;
    try {
      ({ path, assessment } = trustDecision(flags.repo, parsed));
    } catch (cause) {
      this.error((cause as Error).message, { exit: 2 });
    }
    if (flags.json) {
      this.log(JSON.stringify({ path, assessment }, null, 2));
      return;
    }
    this.log(`${assessment.id} (${assessment.useId}): ${assessment.status}`);
    if (assessment.triggeredBy.length > 0)
      this.log(`  revalidation required: ${assessment.triggeredBy.join(", ")}`);
    this.log(`  ${path}`);
  }
}
