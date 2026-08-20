/**
 * The join between the catalog and the fact set (agent-ix/quoin#128).
 *
 * `quoin advise` matches two things written by two different people in two
 * different places:
 *
 *   - the CATALOG — module data, declaring the values that should trigger each
 *     verification method (`applicability.characteristics`);
 *   - the FACT SET — engine code, producing values from an obligation.
 *
 * Nothing compared them, and the failure was silent in both directions.
 * `matchRules` skips an unknown *axis* deliberately (FR-054-CON-2 leaves the
 * axis set open), but an unknown *value* on a known axis simply never matches.
 * `inconclusive` is already a legitimate advisor outcome, so an advisor that
 * could never recommend `integration-testing` looked exactly like one that had
 * nothing to say.
 *
 * Measured when this file was written: 60 values declared, 20 producible, and
 * **7 of 33 methods unreachable by any statement ever written**.
 *
 * This is the check that makes the class impossible to reintroduce. Fixing the
 * 40 without it is a cleanup; with it, it is a rule.
 */

import { describe, expect, it } from "vitest";

import {
  loadMethodCatalog,
  mintableCharacteristics,
} from "../src/advisor/index.js";

/**
 * Values the catalog may declare that no statement is expected to produce,
 * each with the reason it is exempt.
 *
 * Empty, and it should stay that way. An entry here is a method that cannot be
 * recommended, which is a catalog claim nothing can satisfy — the thing this
 * check exists to surface. Add one only with a reason a reader can act on.
 */
const EXEMPT = new Map<string, string>([
  // ── the fact lives in data `advise()` is not given ──
  [
    "cross-repo-boundary",
    "a fact about the trace graph — does this requirement link to a document " +
      "in another repository. advise() receives one obligation's statement, " +
      "not the graph. Needs a fact source, not a regex.",
  ],
  [
    "grammar-declared",
    "a fact about the bundle — is a grammar declared anywhere in it. Same " +
      "shape as cross-repo-boundary: not in the sentence.",
  ],
  [
    "instrumented",
    "a fact about the test environment (is the subject running under an IAST " +
      "agent), not about the requirement.",
  ],

  // ── the fact lives in the code, which quoin never reads ──
  [
    "decidable",
    "a property of the requirement's underlying logic. No author writes the " +
      "word, so a regex would either never fire or fire on something else — " +
      "the CR-014 failure, where membership had to be judged rather than read.",
  ],
  [
    "no-independent-oracle",
    "a property of testability, not of the wording. Same reasoning as " +
      "`decidable`. Note this is NOT `no-executable-oracle`, which is " +
      "producible and does have a regex.",
  ],

  // ── already exists under another name ──
  [
    "judgement-required",
    "a second spelling of `no-executable-oracle`, which is producible and " +
      "whose regex (review|judgement|readable|documented) already covers this " +
      "exact sense. One phrase, one characteristic — declaring both means the " +
      "two names stop meaning different things.",
  ],
]);

/**
 * Methods that cannot be recommended, each with the reason.
 *
 * **Empty, and that is the whole point of the file.** It held 7 when this check
 * was written (agent-ix/quoin#128) and 1 after that ticket — `concolic-execution`,
 * keyed only on `path-sensitive` and `hard-to-reach-branch`, properties of the
 * implementation's control flow that no specification states.
 *
 * agent-ix/quoin#158 re-keyed it on what people actually reach for it for: a
 * magic-value comparison, grammar-shaped input, mandated path coverage,
 * constant-time code, reference equivalence, and the two evidence-side facts.
 * Every catalog method is now reachable by some obligation.
 *
 * Declared rather than silenced, if it ever fills again. An entry here is a
 * catalog claim nothing can satisfy, and the honest options are to give it a
 * fact source or retire it — both decisions, not oversights. A method arriving
 * in this state without being listed fails the check.
 */
const UNREACHABLE_METHODS = new Map<string, string>();

describe("the catalog and the fact set agree", () => {
  // TC-247
  it("declares no characteristic that nothing can produce", () => {
    const catalog = loadMethodCatalog();
    const mintable = mintableCharacteristics();

    const asked = new Map<string, string[]>();
    for (const method of catalog.methods) {
      for (const value of method.applicability.characteristics ?? []) {
        asked.set(value, [...(asked.get(value) ?? []), method.id]);
      }
    }

    const unreachable = [...asked.entries()]
      .filter(([value]) => !mintable.has(value) && !EXEMPT.has(value))
      .map(([value, methods]) => `${value}  <- ${methods.sort().join(", ")}`)
      .sort();

    expect(
      unreachable,
      "the catalog asks for these and no code path produces them, so the " +
        "methods listed beside each can never be recommended",
    ).toEqual([]);
  });

  // TC-251
  it("carries no exemption for a value the catalog no longer declares", () => {
    // Without this the map becomes a place things go to be forgotten. A value
    // retired from the catalog leaves its excuse behind, and the next reader
    // finds a documented reason for something that no longer exists — which is
    // worse than no reason at all, because it looks considered.
    //
    // Caught three on the day it was written: `suite-quality-unknown` (renamed
    // to `fault-detection-unmeasured`), `path-sensitive` and
    // `hard-to-reach-branch` (dropped when `concolic-execution` was re-keyed on
    // things a spec states — agent-ix/quoin#158).
    const catalog = loadMethodCatalog();
    const declared = new Set(
      catalog.methods.flatMap((m) => m.applicability.characteristics ?? []),
    );

    const stale = [...EXEMPT.keys()].filter((v) => !declared.has(v)).sort();

    expect(
      stale,
      "these are exempted from the join check and no method asks for them; " +
        "the exemption is stale and should go with the value",
    ).toEqual([]);
  });

  // TC-248
  it("leaves no method unreachable by every statement", () => {
    // The number that matters. Not "how many values are missing" — how many
    // METHODS the advisor is incapable of recommending, which is what an author
    // actually loses.
    const catalog = loadMethodCatalog();
    const mintable = mintableCharacteristics();

    const unreachable = catalog.methods
      .filter((m) => {
        const axes = Object.keys(m.applicability);
        // Only methods keyed on `characteristics` ALONE. One keyed on a second
        // axis can still be reached through that axis, so it is not stranded.
        if (axes.length !== 1 || axes[0] !== "characteristics") return false;
        const values = m.applicability.characteristics ?? [];
        return (
          !values.some((v) => mintable.has(v)) && !UNREACHABLE_METHODS.has(m.id)
        );
      })
      .map(
        (m) =>
          `${m.id}  keyed on [${m.applicability.characteristics?.join(", ")}]`,
      )
      .sort();

    expect(
      unreachable,
      "these methods are keyed only on characteristics, and not one of those " +
        "values is producible — no requirement can ever elicit them",
    ).toEqual([]);
  });
});
