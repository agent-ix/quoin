import { MUTATION_SCORE_METRIC, type RunEntry } from "../types.js";
import {
  AdapterError,
  type AdapterResult,
  type EvidenceAdapter,
} from "./types.js";

interface MutantScenario {
  name?: string;
  file?: string;
  function?: { function_name?: string };
}

interface Outcome {
  scenario: "Baseline" | { Mutant?: MutantScenario };
  summary?: string;
}

/**
 * `cargo-mutants` `outcomes.json` → one entry per mutated FUNCTION.
 *
 * The reason this format is a reference adapter: it is the only one of the
 * three with a native numeric result, so it is the one that proves the
 * contract carries more than pass/fail (agent-ix/quoin#114, folding in
 * agent-ix/quoin#48's plumbing half). A contract designed without exercising
 * `score` would be designed blind.
 *
 * **What `symbol` means here is not what it means for JUnit, and that is worth
 * stating.** cargo-mutants exercises PRODUCTION functions, not test symbols, so
 * an entry names `src/path.rs::function` — the thing mutated. Binding that back
 * to a requirement needs a requirement→production-code relation, which does not
 * exist yet (agent-ix/quire-rs#171). Until it does, these entries record a
 * measured fact about the code and bind to no obligation, which is the honest
 * state rather than a convenient one.
 */
function symbolOf(mutant: MutantScenario): string | undefined {
  const file = mutant.file?.trim();
  const fn = mutant.function?.function_name?.trim();
  if (file === undefined || file === "" || fn === undefined || fn === "") {
    return undefined;
  }
  return `${file}::${fn}`;
}

export const cargoMutantsAdapter: EvidenceAdapter = {
  name: "cargo-mutants",
  summary: "cargo-mutants outcomes.json — mutation score per mutated function.",
  tools: ["cargo-mutants", "cargo mutants"],
  parse(raw: string): AdapterResult {
    let parsed: { outcomes?: Outcome[] };
    try {
      parsed = JSON.parse(raw) as { outcomes?: Outcome[] };
    } catch (error) {
      throw new AdapterError(
        "cargo-mutants",
        `input is not JSON: ${(error as Error).message}`,
      );
    }
    if (!Array.isArray(parsed.outcomes)) {
      throw new AdapterError(
        "cargo-mutants",
        "no `outcomes` array — expected the contents of mutants.out/outcomes.json",
      );
    }

    // caught / missed per function. `Unviable` mutants are counted in NEITHER:
    // they do not compile, so they exercised no test and belong in no
    // denominator. Including them would depress every score by an amount that
    // says nothing about the suite.
    const caught = new Map<string, number>();
    const missed = new Map<string, number>();
    for (const outcome of parsed.outcomes) {
      if (
        outcome.scenario === "Baseline" ||
        typeof outcome.scenario !== "object"
      ) {
        continue;
      }
      const mutant = outcome.scenario.Mutant;
      if (mutant === undefined) continue;
      const symbol = symbolOf(mutant);
      if (symbol === undefined) continue;
      if (outcome.summary === "CaughtMutant") {
        caught.set(symbol, (caught.get(symbol) ?? 0) + 1);
      } else if (
        outcome.summary === "MissedMutant" ||
        outcome.summary === "Timeout"
      ) {
        missed.set(symbol, (missed.get(symbol) ?? 0) + 1);
      }
    }

    const symbols = [...new Set([...caught.keys(), ...missed.keys()])].sort();
    if (symbols.length === 0) {
      throw new AdapterError(
        "cargo-mutants",
        "no viable mutants in the report — nothing was measured",
      );
    }

    const entries: RunEntry[] = symbols.map((symbol) => {
      const hit = caught.get(symbol) ?? 0;
      const survived = missed.get(symbol) ?? 0;
      const viable = hit + survived;
      return {
        symbol,
        // The TOOL's own classification, not a threshold. A surviving mutant is
        // a demonstrated gap in the suite; calling it "fail" reports what
        // cargo-mutants found. Deciding whether a score is ACCEPTABLE is the
        // auditor's and the consumer gate's job, never this adapter's.
        outcome: survived === 0 ? "pass" : "fail",
        // `viable` is never 0: `symbols` is the union of the caught and missed
        // keys, so every symbol here has at least one mutant on one side. A
        // zero-guard would be a branch no input can reach.
        score: hit / viable,
        // The measurement is named where it is recorded (agent-ix/quoin#138).
        // cargo-mutants produces a mutation score and nothing else, so the
        // adapter states it and the auditor never has to guess from the tool
        // string what the number means.
        metric: MUTATION_SCORE_METRIC,
      };
    });

    // No evidenceKind: naming one here would mint a fourth copy of a
    // vocabulary that already exists in the catalog, the Test Matrix and the
    // suite registry. The suite declares its kind.
    return { entries };
  },
};
