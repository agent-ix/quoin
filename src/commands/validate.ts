import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { inspectEmptyGates } from "../validators/index.js";

export default class Validate extends QuoinCommand {
  static summary = "Validate repository QA gates and report located defects.";
  static description = `Runs deterministic repository validators. The gate-capability check joins an
explicit shell-gate claim, build/CI wiring, and the command that is supposed to
enforce it; report scripts are not treated as gates from their text alone.

Findings are advisory by default. Pass --strict in CI to exit non-zero.`;

  static examples = [
    "quoin validate",
    "quoin validate --strict",
    "quoin validate --json",
  ];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    strict: Flags.boolean({ description: "Exit 1 when a finding remains." }),
    json: Flags.boolean({ description: "Emit findings as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Validate);
    const findings = inspectEmptyGates(flags.repo);
    if (flags.json) {
      this.log(JSON.stringify({ findings }, null, 2));
    } else if (findings.length === 0) {
      this.log("repository QA gates: no findings");
    } else {
      for (const finding of findings) {
        this.log(
          `[warning] ${finding.kind}: ${finding.path}:${finding.line}: ${finding.summary}`,
        );
      }
      this.log(`${findings.length} gate finding(s)`);
    }
    if (flags.strict && findings.length > 0) this.exit(1);
  }
}
