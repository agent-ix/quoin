import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { carriesPayload, runCoreAllowFailure } from "../core/exec.js";
import { repoSnapshot } from "../core/snapshot.js";
import type { RunPayload } from "../core/types.js";

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
    // The first of 54 commands on the `quoin-core` boundary (quoin#412). The
    // analysis is `quoin_validators::inspect_empty_gates_in`; everything left
    // here is the command's own opinions — how to print, and what `--strict`
    // means — which is the split `GateReport::verdict(strict)` already makes on
    // the far side.
    const result = runCoreAllowFailure(
      "validators.run",
      repoSnapshot(flags.repo),
    );
    if (!carriesPayload(result.exitCode)) {
      const detail = result.diagnostics
        .map((d) => `${d.code}: ${d.message}`)
        .join("\n");
      // The repo is named here because the boundary cannot name it. Paths in a
      // request are repository-relative, so an unlistable ROOT arrives on the
      // far side as the empty path and comes back as "repository root  is not
      // a readable directory" — a blank where the user's typo was. On `main`
      // `readdirSync`'s ENOENT escaped and oclif printed the offending path
      // itself; this restores that much (quoin#448 FND-008).
      this.error(
        `quoin validate --repo ${flags.repo}: quoin-core validators.run ` +
          `exited ${result.exitCode}` +
          (detail ? `:\n${detail}` : " with no diagnostic on stderr."),
        { exit: result.exitCode },
      );
    }
    const { findings } = result.payload as RunPayload;

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
