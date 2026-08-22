// Quality scoring for eval runs (agent-ix/quoin#201, FR-043-AC-2/4/5).
//
// The harness already records tokens, latency and tool calls per scenario —
// so an eval run answered "was it cheap" and could not answer "was it right".
// These three dimensions close that, fed by the seeded-defect labels in
// `evals/fixtures/bench/labels.json`.
//
// The point is not to add numbers. It is that a cheap run producing wrong
// findings currently scores better than an expensive run producing right ones,
// and nothing in the report says so.

/**
 * Finding precision and recall against a labelled corpus (FR-043-AC-2).
 *
 * Reported **per defect family**, never as one figure. A tool that finds every
 * marker mismatch and no vacuous suite has a respectable average and a hole,
 * and the average is what hides it.
 *
 * A finding matching no label is a false positive; a label the run did not
 * match is a miss — unless the label says it was never `findable`, in which
 * case it is excluded from the denominator. That distinction is FR-043-AC-7's
 * whole reason for existing: a scored miss must be distinguishable from a
 * defect nobody claimed was findable.
 */
export function scoreFindings(found, labels) {
  const families = new Map();
  const bucket = (family) => {
    if (!families.has(family)) {
      families.set(family, {
        family,
        truePositives: 0,
        falsePositives: 0,
        misses: 0,
      });
    }
    return families.get(family);
  };

  const expected = labels.filter((l) => l.findable !== false);
  const matched = new Set();

  for (const finding of found) {
    const hit = expected.find(
      (l) => !matched.has(l.id) && l.family === finding.family,
    );
    if (hit) {
      matched.add(hit.id);
      bucket(hit.family).truePositives += 1;
    } else {
      bucket(finding.family).falsePositives += 1;
    }
  }
  for (const label of expected) {
    if (!matched.has(label.id)) bucket(label.family).misses += 1;
  }

  const rows = [...families.values()]
    .map((f) => ({
      ...f,
      precision: ratio(f.truePositives, f.truePositives + f.falsePositives),
      recall: ratio(f.truePositives, f.truePositives + f.misses),
    }))
    .sort((a, b) => a.family.localeCompare(b.family));

  // Labels declared unfindable are reported, not silently dropped: an
  // excluded denominator nobody sees is a denominator nobody can question.
  const excluded = labels.filter((l) => l.findable === false).map((l) => l.id);
  return { families: rows, excluded };
}

/**
 * Actionability: the fraction of findings a reader can act on without opening
 * another tool (FR-043-AC-4).
 *
 * Measured at pass 2 as **15 of 496** — 481 findings that named neither the row
 * they came from nor a line that distinguished them. A finding you cannot act
 * on is a finding nobody acts on, whatever its precision.
 */
export function scoreActionability(found) {
  const actionable = found.filter((f) => hasLocus(f)).length;
  return {
    actionable,
    total: found.length,
    rate: ratio(actionable, found.length),
  };
}

/** A finding is actionable when it names WHERE — a row id or a document line. */
function hasLocus(finding) {
  return Boolean(
    (finding.rowId && String(finding.rowId).trim()) ||
    (typeof finding.line === "number" && finding.line > 0),
  );
}

/**
 * Cost per confirmed insight (FR-043-AC-5), in tokens **and** tool calls.
 *
 * Both, because they are different costs: tokens are the context budget and
 * tool calls are wall-clock and blast radius. A run that reads the whole corpus
 * once and a run that greps it forty times can spend the same tokens.
 *
 * Denominator is CONFIRMED findings — true positives — not findings emitted.
 * Dividing by emitted output rewards a run for producing more of it, which is
 * precisely backwards.
 */
export function scoreCost(metrics, truePositives) {
  const tokens = metrics?.tokenUsage?.total ?? 0;
  const toolCalls = metrics?.toolCalls ?? 0;
  if (truePositives === 0) {
    // Not Infinity and not 0. A run that confirmed nothing has no cost PER
    // insight, and reporting either number would be a claim about efficiency
    // that the run does not support.
    return {
      tokens,
      toolCalls,
      truePositives: 0,
      tokensPer: null,
      toolCallsPer: null,
    };
  }
  return {
    tokens,
    toolCalls,
    truePositives,
    tokensPer: Math.round(tokens / truePositives),
    toolCallsPer: Number((toolCalls / truePositives).toFixed(2)),
  };
}

/** Every quality dimension for one scenario, ready to sit beside the cost columns. */
export function scoreScenario({ found = [], labels = [], metrics = {} }) {
  const findings = scoreFindings(found, labels);
  const truePositives = findings.families.reduce(
    (n, f) => n + f.truePositives,
    0,
  );
  return {
    findings,
    actionability: scoreActionability(found),
    cost: scoreCost(metrics, truePositives),
  };
}

/** `null` rather than 0 when there is no denominator — 0/0 is not 0%. */
function ratio(num, den) {
  return den === 0 ? null : Number((num / den).toFixed(3));
}
