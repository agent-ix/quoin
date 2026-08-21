import { execFileSync } from "node:child_process";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { affirm, readBindings, writeBindings } from "../../evidence/index.js";
import {
  checkVersionPremise,
  parseCoverage,
  quireVersion,
  runQuire,
} from "../../quire/index.js";

export default class EvidenceAffirm extends QuoinCommand {
  static summary =
    "Re-affirm a binding after the statement it was made against changed.";
  static description = `A reworded acceptance criterion changes its content hash, which makes every
binding on it SUSPECT until somebody says the evidence still discharges the new
wording. This is that act.

Auto-bind, explicit affirmation: first discharge binds without asking, because
requiring a signature on something the evidence already proves puts a gate in
front of the common case and teaches people to click through it. Re-affirmation
is the judgement call, so it stays explicit — and it is recorded with who, at
which commit, and optionally why.

Re-running the test does NOT clear suspicion. If it did, the suspect state would
clear itself on the next CI run and the detector would never fire.`;

  static examples = [
    "quoin evidence affirm --obligation FR-001-AC-1 --who @reviewer",
    "quoin evidence affirm --obligation NFR-006-M-2 --who @reviewer --note 'threshold widened deliberately'",
  ];

  static flags = {
    obligation: Flags.string({
      description: "Obligation id whose binding is being re-affirmed.",
      required: true,
    }),
    who: Flags.string({
      description: "Who is affirming. Recorded verbatim.",
      required: true,
    }),
    note: Flags.string({ description: "Why the evidence still holds." }),
    commit: Flags.string({
      description: "Commit of the affirmation. Defaults to HEAD.",
    }),
    repo: Flags.string({ description: "Repository root.", default: "." }),
    module: Flags.string({
      description: "Module directory supplying the traceability model.",
    }),
    json: Flags.boolean({ description: "Emit the outcome as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceAffirm);

    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const args = ["coverage", "--scope", flags.repo, "--json"];
    if (flags.module) args.push("--module", flags.module);
    const parsed = parseCoverage(runQuire(args));
    if (!parsed.ok) this.error(parsed.error.message, { exit: 2 });

    const current = (parsed.value.obligations ?? []).find(
      (o) => o.id === flags.obligation,
    );
    if (!current) {
      // Affirming an obligation the spec no longer states would write a record
      // about nothing — the failure this store exists to make impossible.
      this.error(
        `no obligation \`${flags.obligation}\` is derived from this spec today. ` +
          `Either the id is wrong, or the requirement it named is gone — in which ` +
          `case the binding should be removed rather than affirmed.`,
        { exit: 2 },
      );
    }

    const commit =
      flags.commit ??
      execFileSync("git", ["-C", flags.repo, "rev-parse", "HEAD"], {
        encoding: "utf8",
      }).trim();

    const before = readBindings(flags.repo).bindings;
    const { bindings, found } = affirm(
      before,
      flags.obligation,
      current.statement_hash,
      flags.who,
      commit,
      flags.note,
    );
    if (!found) {
      this.error(
        `no binding exists for \`${flags.obligation}\`, so there is nothing to ` +
          `affirm. A binding is created when a suite first discharges the ` +
          `obligation (\`quoin evidence record\`).`,
        { exit: 2 },
      );
    }

    writeBindings(flags.repo, {
      bindings,
    });
    const outcome = {
      obligation: flags.obligation,
      who: flags.who,
      commit,
      statementHash: current.statement_hash,
    };
    if (flags.json) {
      this.log(JSON.stringify(outcome, null, 2));
      return;
    }
    this.log(
      `affirmed ${flags.obligation} by ${flags.who} at ${commit.slice(0, 12)} ` +
        `(hash ${current.statement_hash.slice(0, 12)}…)`,
    );
  }
}
