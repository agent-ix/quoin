import { execFileSync } from "node:child_process";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { loadMethodCatalog } from "../../advisor/index.js";
import { audit, ratchet } from "../../auditor/index.js";
import {
  baselinePath,
  latestRuns,
  latestScans,
  mockInspectionInput,
  readBaseline,
  readBindings,
} from "../../evidence/index.js";
import {
  checkVersionPremise,
  parseCoverage,
  quireVersion,
  runQuire,
} from "../../quire/index.js";

export default class EvidenceAudit extends QuoinCommand {
  static summary =
    "Audit the evidence store: suspect links, staleness, vacuity.";
  static description = `Reads the store and reports. RUNS NOTHING (ADR-0011 invariant 1) — the
consumer's CI refreshes evidence, and an auditor that could re-run a suite could
also make a finding disappear by re-running it.

Checks:
  suspect-link      the statement changed after the evidence was bound
  stale-evidence    bound to a missing run, or to a run behind HEAD
  vacuous-evidence  every bound symbol was skipped or absent from the run
  undischarged      no evidence is bound at all
  method-conformance   evidence of a kind the declared method does not produce
  unknown-method       the declared method is in no catalog
  insufficient-multiplicity   criticality demands two independent suites
  insufficient-mutation-score criticality demands a mutation score it misses
  unmeasured-mutation-score   a demanded mutation score, with none recorded

--ratchet compares against spec/evidence/baseline.json and fails only on NEW
violations. A gate that fails on the whole existing backlog gets disabled within
a week. Write that baseline with: quoin evidence baseline`;

  static examples = [
    "quoin evidence audit",
    "quoin evidence audit --ratchet --strict",
    "quoin evidence audit --json",
  ];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    module: Flags.string({
      description: "Module directory supplying the traceability model.",
    }),
    ratchet: Flags.boolean({
      description: "Report only violations absent from the baseline.",
    }),
    strict: Flags.boolean({
      description: "Exit 1 when any reported finding remains.",
    }),
    json: Flags.boolean({ description: "Emit the report as JSON." }),
    "multiplicity-requires": Flags.string({
      description:
        "Criticality values demanding two independent suites. Repeatable. " +
        "Unset by default: no obligation source in the ecosystem declares a " +
        "criticality column, so a built-in default could never fire.",
      multiple: true,
    }),
    "mutation-floor": Flags.string({
      description:
        "Minimum mutation score per criticality, as <criticality>=<0..1>. " +
        "Repeatable. Unset by default, for the same reason as " +
        "--multiplicity-requires: a built-in floor is a rule nobody chose.",
      multiple: true,
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceAudit);

    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const args = ["coverage", "--scope", flags.repo, "--json"];
    if (flags.module) args.push("--module", flags.module);
    const parsed = parseCoverage(runQuire(args));
    if (!parsed.ok) this.error(parsed.error.message, { exit: 2 });

    const head = headCommit(flags.repo);
    const mockInspections = mockInspectionInput(flags.repo, head);
    const report = audit({
      obligations: parsed.value.obligations ?? [],
      bindings: readBindings(flags.repo).bindings,
      runs: latestRuns(flags.repo),
      scans: latestScans(flags.repo),
      injections: mockInspections.injections,
      mockInspectionSuites: mockInspections.suites,
      // The SAME module the coverage call above used. Defaulting to the
      // installed roots meant `--module <dir>` derived obligations from one
      // catalog and checked conformance against another — the exact disagreement
      // `src/advisor/methods.ts` opens by warning about (agent-ix/quoin#105).
      catalog: loadMethodCatalog(flags.module ? [flags.module] : undefined),
      headCommit: head,
      // Deliberately unset. No obligation source in the ecosystem declares a
      // `criticality_column` — measured: 2,304 of 2,304 `Acceptance Criteria`
      // tables are `ID | Criteria | Verification` and carry no priority
      // (spec-artifacts-process CR-005) — so a hardcoded `["P0"]` was a rule
      // that could never fire, and would have fired on *everything* the moment
      // a column appeared. `--multiplicity-requires` makes it a choice.
      multiplicityRequires: flags["multiplicity-requires"],
      mutationFloor: parseMutationFloor(flags["mutation-floor"], (message) =>
        this.error(message, { exit: 2 }),
      ),
    });

    const baseline = flags.ratchet ? readBaseline(flags.repo) : null;
    // Whether ratcheting was ACTUALLY applied — `flags.ratchet` is only what
    // was asked for. A missing baseline degrades the run to a full report, and
    // labelling that full report "(new violations only)" told a day-one reader
    // their whole backlog was new violations (#169).
    const ratcheted = baseline !== null;
    const reported = baseline ? ratchet(report, baseline) : report.findings;

    if (flags.ratchet && !ratcheted && !flags.json) {
      this.log(missingBaselineNotice(flags.repo));
    }

    if (flags.json) {
      this.log(
        JSON.stringify(
          {
            findings: reported,
            healthy: report.healthy,
            unevaluated: report.unevaluated,
            ratchet: ratcheted,
          },
          null,
          2,
        ),
      );
    } else if (reported.length === 0 && report.unevaluated.length === 0) {
      this.log(
        `${report.healthy.length} obligation(s) with healthy evidence; nothing to report`,
      );
    } else {
      for (const finding of reported) {
        this.log(`[${finding.severity}] ${finding.kind}: ${finding.summary}`);
      }
      for (const check of report.unevaluated) {
        this.log(
          `[not-evaluated] ${check.check}: ${check.obligation}: ${check.reason}`,
        );
      }
      this.log("");
      this.log(
        `${reported.length} finding(s), ${report.unevaluated.length} check(s) not evaluated, ` +
          `${report.healthy.length} healthy` +
          (ratcheted ? " (new violations only)" : ""),
      );
    }

    if (
      flags.strict &&
      (reported.length > 0 || report.unevaluated.length > 0)
    ) {
      this.exit(1);
    }
  }
}

/**
 * What `--ratchet` prints when there is no baseline to ratchet against.
 *
 * Named path, named remedy: the reader who hits this on day one has not
 * broken anything and has not introduced the backlog they are looking at —
 * they are one `quoin evidence baseline` away from the gate they asked for.
 */
export function missingBaselineNotice(repo: string): string {
  return (
    `--ratchet requested but ${baselinePath(repo)} does not exist; ` +
    `reporting the full backlog, not new violations. ` +
    `Write the baseline with: quoin evidence baseline`
  );
}

/**
 * The newest recorded run per suite, by `timestamp`.
 *
 * This used to take `listRuns(...).at(-1)` — the lexicographically last
 * `<commit12>.json`, which is uniformly random hex and so picked the newest run
 * with probability 1/n. It drove `stale-evidence` (`run.commit !== headCommit`)
 * and `vacuous-evidence` (which symbols the run reported), so a fresh run at
 * HEAD could still be reported stale, and re-recording could not fix it
 * (agent-ix/quoin#104).
 */
function headCommit(repo: string): string | undefined {
  try {
    return execFileSync("git", ["-C", repo, "rev-parse", "HEAD"], {
      encoding: "utf8",
    }).trim();
  } catch {
    return undefined;
  }
}

/**
 * `--mutation-floor P0=0.8` → `{ P0: 0.8 }`.
 *
 * Rejected rather than ignored: a malformed entry means the operator asked for
 * a threshold and would otherwise get a clean report saying nothing was below
 * it. A floor that silently does not apply is worse than no floor, because it
 * reads as a passing gate.
 */
export function parseMutationFloor(
  entries: string[] | undefined,
  fail: (message: string) => never,
): Record<string, number> | undefined {
  if (!entries || entries.length === 0) return undefined;
  const out: Record<string, number> = {};
  for (const entry of entries) {
    const [criticality, raw] = entry.split("=");
    const score = Number(raw);
    if (!criticality || raw === undefined || Number.isNaN(score)) {
      fail(
        `--mutation-floor expects <criticality>=<score>, got '${entry}'. ` +
          `Example: --mutation-floor P0=0.8`,
      );
    }
    if (score < 0 || score > 1) {
      // A ratio, not a percentage. `--mutation-floor P0=80` would otherwise be
      // a floor nothing can ever reach, reported as every P0 failing.
      fail(
        `--mutation-floor score must be a ratio in [0, 1], got '${raw}'. ` +
          `80% is 0.8, not 80.`,
      );
    }
    out[criticality] = score;
  }
  return out;
}
