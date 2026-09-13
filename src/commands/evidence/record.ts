import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { ADAPTER_NAMES, parseLineage, record } from "../../core/evidence.js";
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
    "quoin evidence record --suite SUITE-001 --commit $(git rev-parse HEAD) --tool 'cargo test 1.94.1' --results run.json",
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
    lineage: Flags.string({
      description:
        "JSON file naming profile-relevant evidence lineage: actor, " +
        "implementationToolchain, technique, dataSource, and/or reviewPath. " +
        "These are relationship facts, not an independence verdict.",
    }),
    discharges: Flags.string({
      description:
        "Obligation ids this scan discharges, comma-separated. Finding-shaped " +
        "only. A CLEAN scan is the strongest evidence a scanner produces and " +
        "carries no finding to bind from, so the obligations it was run to " +
        "check are stated rather than inferred. A scan that evaluated no rules " +
        "binds nothing regardless.",
      multiple: false,
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
    if (!isVersionedToolIdentity(flags.tool)) {
      this.error(
        "--tool must include an immutable version (for example `vitest 3.2.4` or `scanner git:<full-sha>`)",
        { exit: 2 },
      );
    }

    let lineage;
    try {
      lineage = flags.lineage
        ? parseLineage(readFileSync(flags.lineage, "utf8"))
        : undefined;
    } catch (cause) {
      this.error((cause as Error).message, { exit: 2 });
    }

    // The premise first: an older quire does not fail, it emits an older shape
    // that this command would misread. By the time a parse failed, the wrong
    // obligations would already have been bound.
    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    // Obligations first, for BOTH paths. A scan discharges obligations too,
    // so deriving them only on the run path is what left the scan branch with
    // nothing to bind against (SR-005 FND-001).
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

    // One call for both record types. The choice between a run record and a
    // finding-shaped scan is made by the ADAPTER REGISTRY before anything is
    // parsed — letting a scan fall through to the run path would write it into
    // `runs/` and lose the clean-versus-unrun distinction at the point of
    // intake, silently and permanently for that commit (FR-034). The payload
    // is tagged, so this reads which record was written rather than inferring
    // it from which fields came back.
    const results =
      flags.results === "-"
        ? readFileSync(0, "utf8")
        : readFileSync(flags.results, "utf8");
    let outcome;
    try {
      outcome = record({
        repo: flags.repo,
        suite: flags.suite,
        commit: flags.commit,
        tool: flags.tool,
        ...(flags.kind === undefined ? {} : { kind: flags.kind }),
        ...(lineage === undefined ? {} : { lineage }),
        timestamp: flags.timestamp ?? new Date().toISOString(),
        ...(flags.adapter === undefined ? {} : { adapter: flags.adapter }),
        results,
        discharges: (flags.discharges ?? "")
          .split(",")
          .map((id) => id.trim())
          .filter((id) => id !== ""),
        obligations: parsed.value.obligations ?? [],
      });
    } catch (cause) {
      this.error((cause as Error).message, { exit: 2 });
    }

    if (outcome.kind === "scan") {
      if (flags.json) {
        this.log(
          JSON.stringify(
            {
              scanPath: outcome.scan_path,
              findings: outcome.findings,
              bound: outcome.bound,
              unknown: outcome.unknown,
            },
            null,
            2,
          ),
        );
        return;
      }
      this.log(
        `recorded scan ${flags.suite} @ ${flags.commit.slice(0, 12)} → ${outcome.scan_path}`,
      );
      this.log(`  bound: ${outcome.bound.length}`);
      if (outcome.unknown.length > 0) {
        // Named, not counted: an id nothing states is a typo or a reworded
        // obligation, and both need the id to act on.
        this.log(
          `  discharges no such obligation: ${outcome.unknown.join(", ")}`,
        );
      }
      if (outcome.vacuous && (flags.discharges ?? "").trim() !== "") {
        this.log(
          "  bound nothing: the scan evaluated no rules, so it found nothing " +
            "because it looked for nothing",
        );
      }
      // Said explicitly, because "0 findings" is the one line a reader is most
      // likely to mistake for "nothing ran".
      this.log(
        `  findings: ${outcome.findings.length}` +
          (outcome.rules_evaluated === undefined ||
          outcome.rules_evaluated === null
            ? " (the tool reported no rule count, so this cannot be told from a scan with no rules enabled)"
            : ` over ${outcome.rules_evaluated} rules evaluated`),
      );
      return;
    }

    if (flags.json) {
      this.log(
        JSON.stringify(
          {
            runPath: outcome.run_path,
            bound: outcome.bound,
            suspect: outcome.suspect,
            unmatched: outcome.unmatched,
            ...(outcome.unrepresented === undefined ||
            outcome.unrepresented === null
              ? {}
              : { unrepresented: outcome.unrepresented }),
          },
          null,
          2,
        ),
      );
      return;
    }

    this.log(
      `recorded ${flags.suite} @ ${flags.commit.slice(0, 12)} → ${outcome.run_path}`,
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
    // Said out loud, never dropped: the producer reported these and no
    // run-entry outcome carries them, so the record is short by exactly this
    // much and the reader is told which.
    for (const item of outcome.unrepresented ?? []) {
      this.log(
        `  not transcribed — ${item.symbol} reported ${item.state}: ${item.reason}`,
      );
    }
  }
}

export function isVersionedToolIdentity(value: string): boolean {
  return /(?:^|[\s/@])(?:v?\d+(?:\.\d+)+(?:[-+][0-9A-Za-z.-]+)?|git:[0-9a-f]{40}|sha256:[0-9a-f]{64})(?:$|[\s,)])/i.test(
    value,
  );
}
