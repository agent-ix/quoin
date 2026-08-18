import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  ADAPTER_NAMES,
  obligationsFrom,
  recordRun,
  selectAdapter,
  type RunEntry,
} from "../../evidence/index.js";
import {
  checkVersionPremise,
  parseCoverage,
  quireVersion,
  runQuire,
} from "../../quire/index.js";

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

--adapter selects a format reader. Without it the suite's --tool picks one, and
failing that the normalized shape above is assumed. An adapter transcribes; it
runs nothing and judges nothing.`;

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
    kind: Flags.string({
      description:
        "Evidence kind this run produced (Unit, Property, Static, Manual, …) " +
        "— the vocabulary the catalog's `evidence_kind` and the suite " +
        "registry's `Evidence Kind` column use. Method conformance compares " +
        "kind to kind; without it the check stays silent rather than guessing.",
    }),
    adapter: Flags.string({
      description:
        `Format reader for --results (${ADAPTER_NAMES.join(", ")}). ` +
        "Defaults to the adapter claiming --tool, else the normalized shape. " +
        "An unknown name is an error rather than a silent fall back, so a typo " +
        "reports itself instead of failing later as a JSON-shape complaint.",
      options: [...ADAPTER_NAMES],
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
    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const entries = readEntries(flags.results, {
      adapter: flags.adapter,
      tool: flags.tool,
    });

    const coverageArgs = ["coverage", "--scope", flags.repo, "--json"];
    if (flags.module) coverageArgs.push("--module", flags.module);
    const raw = runQuire(coverageArgs);
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
      evidenceKind: flags.kind,
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

/**
 * Read `--results` through the selected adapter.
 *
 * The only place a raw tool file becomes run entries. Reading the file is the
 * caller's side of the adapter contract: adapters are pure over text so that
 * `quoin evidence record` remains a transcriber and can never become a test
 * runner (ADR-0011 invariant 1).
 */
function readEntries(
  source: string,
  options: { adapter?: string; tool?: string },
): RunEntry[] {
  const text =
    source === "-" ? readFileSync(0, "utf8") : readFileSync(source, "utf8");
  return selectAdapter(options).parse(text).entries;
}
