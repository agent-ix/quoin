import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { advise, loadMethodCatalog } from "../advisor/index.js";
import type {
  Advice,
  MethodCatalog,
  ObligationEvidence,
  ObligationFacts,
} from "../advisor/index.js";
import { scoresFor } from "../auditor/index.js";
import { latestRuns, readBindings } from "../evidence/index.js";
import type { Binding, RunRecord } from "../evidence/index.js";
import {
  checkVersionPremise,
  parseCoverage,
  parseProperties,
  quireVersion,
  runQuire,
  runQuireAllowFailure,
} from "../quire/index.js";
import type { Obligation, PropertiesReport } from "../quire/index.js";

export default class Advise extends QuoinCommand {
  static summary =
    "Recommend a verification method for each obligation, from the catalog.";
  static description = `The deterministic half of the test-plan advisor (FR-031). Rules match or they do
not; where they are inconclusive it says so and stops, rather than guessing.

Before this, the method table was skill-local prose and \`Verification\` columns
defaulted to Test by habit — nothing ever advised DAST for an attack surface,
monitors for a temporal property, or fault injection for a reliability NFR.

Obligations come from quire (coverage --json), and each criterion's FR-052
property shape from quire (properties --json), so the advice is over what the
spec actually states rather than over a document somebody pasted in.

Read the output as a recommendation, never a verdict. An LLM may judge the
residue afterwards — labelled as judgement (the FR-042 / ADR-0010 discipline).`;

  static examples = [
    "quoin advise",
    "quoin advise --mismatch-only",
    "quoin advise --json",
  ];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    module: Flags.string({
      description: "Module directory supplying the traceability model.",
    }),
    "mismatch-only": Flags.boolean({
      description:
        "Only obligations whose authored method is not among the recommendations.",
    }),
    "inconclusive-only": Flags.boolean({
      description: "Only obligations no rule matched.",
    }),
    json: Flags.boolean({ description: "Emit the advice as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Advise);

    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const scoped = (verb: string) => {
      const args = [verb, "--scope", flags.repo, "--json"];
      if (flags.module) args.push("--module", flags.module);
      return args;
    };

    const coverage = parseCoverage(runQuire(scoped("coverage")));
    if (!coverage.ok) this.error(coverage.error.message, { exit: 2 });

    const catalog = loadMethodCatalog(
      flags.module ? [flags.module] : undefined,
    );
    if (catalog.methods.length === 0) {
      this.error(
        "no active module declares a `verification_catalog`, so there is " +
          "nothing to advise from. Install one (e.g. spec-artifacts-process) " +
          "or pass --module.",
        { exit: 2 },
      );
    }
    for (const bad of catalog.unreadable) {
      this.warn(`skipped ${bad.moduleRoot}: ${bad.reason}`);
    }

    const shapes = propertyShapes(flags.repo);
    const obligations = coverage.value.obligations ?? [];
    if (obligations.length > 0 && shapes.size === 0) {
      // Said out loud rather than degraded silently. `property_shapes` is the
      // axis most catalog entries are keyed on, so without it `round-trip`
      // never reaches property-based or metamorphic testing and the advice is
      // characteristics-only — which looks like "no rule matched" and is not.
      this.warn(
        "no property shapes were read, so recommendations rest on statement " +
          "text alone. `quire properties` resolves archetypes from the whole " +
          "installed module set; check that the modules declaring FR/NFR/StR " +
          "are installed (quoin module list).",
      );
    }

    // The store is read ONCE, here, and handed in. `advise` performs no I/O —
    // an advisor that could reach the filesystem could also disagree with the
    // auditor about what it found (ADR-0011).
    const bindings = readBindings(flags.repo).bindings;
    const runs = latestRuns(flags.repo);

    let advice = obligations.map((o) =>
      advise(
        catalog,
        factsFor(
          o,
          shapes.get(o.id),
          evidenceFor(o.id, bindings, runs, catalog),
        ),
      ),
    );
    if (flags["mismatch-only"]) advice = advice.filter((a) => a.mismatch);
    if (flags["inconclusive-only"]) {
      advice = advice.filter((a) => a.inconclusive);
    }

    if (flags.json) {
      this.log(JSON.stringify({ advice }, null, 2));
      return;
    }
    this.render(advice, obligations.length);
  }

  private render(advice: Advice[], total: number): void {
    for (const a of advice) {
      const head = a.recommended
        .slice(0, 3)
        .map((r) => `${r.method} (${r.reasons.map((x) => x.value).join(", ")})`)
        .join("; ");
      this.log(
        `${a.obligation}  authored=${a.authored ?? "—"}  ` +
          (a.inconclusive ? "inconclusive" : `recommend: ${head}`) +
          (a.mismatch ? "  ⚠ mismatch" : ""),
      );
    }
    this.log("");
    const mismatched = advice.filter((a) => a.mismatch).length;
    const inconclusive = advice.filter((a) => a.inconclusive).length;
    this.log(
      `${advice.length} of ${total} obligation(s) shown — ` +
        `${mismatched} mismatch, ${inconclusive} inconclusive. ` +
        `Recommendations, not verdicts: confirm the method in spec review.`,
    );
  }
}

/**
 * The FR-052 property shape and archetype of each criterion, by row id.
 *
 * Read from `properties --json` because the coverage payload carries neither,
 * and `property_shapes` is the axis most catalog entries are keyed on — without
 * it, `round-trip` would never reach property-based or metamorphic testing.
 *
 * **Deliberately scoped, never `--module`.** `--module` selects the *one*
 * module supplying the traceability model, and classification needs the
 * archetype the document's `type:` names — which usually lives in a different
 * module. Passing `--module <process>` here resolves no `FR` archetype at all
 * and yields zero criteria, silently. Scoped discovery loads the whole
 * installed set, which is what classification requires.
 *
 * An NFR measurement row is an obligation and not a criterion, so it has no
 * entry here and is advised from its statement alone. That is a real limit, not
 * a gap to paper over: no classifier ran on it.
 */
function propertyShapes(
  repo: string,
): Map<string, { property: string; archetype: string }> {
  const args = ["properties", "spec/**/*.md", "--scope", repo, "--json"];
  const out = new Map<string, { property: string; archetype: string }>();
  // `properties` exits 1 when ANY input document fails to resolve — an asset
  // with no `type:`, say — while still writing a complete payload for every
  // document that did. Two untyped files must not cost the whole shape axis.
  const result = runQuireAllowFailure(args);
  if (!result.stdout.trim()) return out;
  const parsed = parseProperties(result.stdout);
  if (!parsed.ok) return out;
  for (const doc of (parsed.value as PropertiesReport).documents) {
    for (const c of doc.criteria) {
      if (c.row_id)
        out.set(c.row_id, { property: c.property, archetype: doc.archetype });
    }
  }
  return out;
}

/**
 * What the store records about one obligation.
 *
 * `scoresFor` is the auditor's, reused rather than reimplemented: one
 * definition of what a fault-detection score is, so the auditor's finding and
 * the advisor's recommendation cannot disagree about the same run.
 */
function evidenceFor(
  id: string,
  bindings: Binding[],
  runs: RunRecord[],
  catalog: MethodCatalog,
): ObligationEvidence {
  const mine = bindings.filter((b) => b.obligation === id);
  return {
    bound: mine.length > 0,
    faultDetectionScores: scoresFor(mine, runs, catalog),
  };
}

function factsFor(
  obligation: Obligation,
  shape: { property: string; archetype: string } | undefined,
  evidence: ObligationEvidence,
): ObligationFacts {
  return {
    id: obligation.id,
    statement: obligation.statement,
    authoredMethod: obligation.method ?? null,
    propertyShape: shape?.property ?? null,
    archetype: shape?.archetype ?? archetypeOf(obligation.id),
    criticality: obligation.criticality ?? null,
    // The one STRUCTURED signal quire emits about an obligation. It was typed
    // and parsed and then dropped on this exact seam, so the advisor guessed
    // from prose while `{"target": "< 4 min"}` sat unread in the record (#166).
    parameters: obligation.parameters,
    evidence,
  };
}

/** `NFR-006-M-2` → `NFR`. The id prefix is the archetype for every ISO id. */
function archetypeOf(id: string): string | null {
  const prefix = id.split("-")[0];
  return prefix.length > 0 ? prefix : null;
}
