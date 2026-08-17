import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  obligationsFrom,
  recordRun,
  type RunEntry,
} from "../../evidence/index.js";
import { checkVersionPremise, parseCoverage } from "../../quire/index.js";

export default class EvidenceRecord extends QuoinCommand {
  static summary = "Transcribe one suite run into the evidence store.";
  static description = `Records ONE run of ONE suite at ONE commit, then binds every obligation that
run discharged, stamping the statement hash as it stands now.

quoin transcribes; the consumer's CI executes (ADR-0011 invariant 1). This
command runs no test and judges no result — recording a FAILING run is as
legitimate as recording a passing one, and more useful, because a suite that
stopped passing is exactly what a freshness check needs to see.

--results takes normalized entries:

  {"entries": [{"symbol": "tests::tc001", "outcome": "pass", "traceIds": ["FR-001-AC-1"]}]}

Format adapters (junit, llvm-cov, cargo-mutants, SARIF) are a separate ticket
(agent-ix/quoin#91); this verb owns the store, not the parsing of every tool.`;

  static examples = [
    "quoin evidence record --suite SUITE-001 --commit $(git rev-parse HEAD) --tool 'cargo test' --results run.json",
  ];

  static flags = {
    suite: Flags.string({
      description: "Suite id from spec/evidence/suites.md (e.g. SUITE-001).",
      required: true,
    }),
    commit: Flags.string({
      description: "Full commit sha the run was performed at.",
      required: true,
    }),
    tool: Flags.string({
      description: "Tool and version, as it identifies itself.",
      required: true,
    }),
    results: Flags.string({
      description: "Normalized run entries as JSON. `-` reads stdin.",
      required: true,
    }),
    repo: Flags.string({
      description: "Repository root. Defaults to the working directory.",
      default: ".",
    }),
    module: Flags.string({
      description: "Module directory supplying the traceability model.",
    }),
    timestamp: Flags.string({
      description: "ISO-8601 run time. Defaults to now.",
    }),
    json: Flags.boolean({ description: "Emit the outcome as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceRecord);

    // The premise first: an older quire does not fail, it emits an older shape
    // that this command would misread. By the time a parse failed, the wrong
    // obligations would already have been bound.
    const premise = checkVersionPremise(readQuireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const entries = readEntries(flags.results);

    const coverageArgs = ["coverage", "--scope", flags.repo, "--json"];
    if (flags.module) coverageArgs.push("--module", flags.module);
    const raw = execFileSync("quire", coverageArgs, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    const parsed = parseCoverage(raw);
    if (!parsed.ok) {
      this.error(
        `${parsed.error.message}\n${parsed.error.errors.slice(0, 10).join("\n")}`,
        { exit: 2 },
      );
    }

    const outcome = recordRun({
      repo: flags.repo,
      suite: flags.suite,
      commit: flags.commit,
      tool: flags.tool,
      timestamp: flags.timestamp ?? new Date().toISOString(),
      entries,
      obligations: obligationsFrom(parsed.value),
    });

    if (flags.json) {
      this.log(JSON.stringify(outcome, null, 2));
      return;
    }

    this.log(
      `recorded ${flags.suite} @ ${flags.commit.slice(0, 12)} → ${outcome.runPath}`,
    );
    this.log(`  bound: ${outcome.bound.length}`);
    if (outcome.suspect.length > 0) {
      // Named rather than counted: a suspect binding is a thing somebody has to
      // read a statement about, so the ids are what the reader needs.
      this.log(
        `  suspect (statement changed since binding, re-affirm to clear): ${outcome.suspect.join(", ")}`,
      );
    }
    if (outcome.unmatched.length > 0) {
      this.log(
        `  unmatched trace ids (tagged in the suite, stated by no obligation): ${outcome.unmatched.join(", ")}`,
      );
    }
  }
}

function readQuireVersion(): string | null {
  try {
    return execFileSync("quire", ["--version"], { encoding: "utf8" });
  } catch {
    return null;
  }
}

function readEntries(source: string): RunEntry[] {
  const text =
    source === "-" ? readFileSync(0, "utf8") : readFileSync(source, "utf8");
  const parsed = JSON.parse(text) as { entries?: RunEntry[] };
  if (!Array.isArray(parsed.entries)) {
    throw new Error(
      '--results must be JSON of the form {"entries": [{symbol, outcome, traceIds?}]}',
    );
  }
  return parsed.entries;
}
