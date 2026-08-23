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
  const claimed = new Set();
  let positional = 0;

  // Pass 0 — declared collateral (#199).
  //
  // Some seeded defects necessarily produce a SECOND, correct finding of a
  // different family. `hollow-denominator` is the measured case: a corpus
  // whose evidence symbols bind nothing makes `coverage.backed` a ratio over
  // an unread population AND makes the binder report `no-symbol-bound`, and
  // the engine cannot separate the two causes at all — `coverage.backed` is
  // the only ratio metric that can go hollow, and its one cause already has a
  // bespoke diagnostic.
  //
  // Scoring that second finding as a false positive would punish the toolchain
  // for being right, and it would be a claim about a family this corpus does
  // not seed. Excluding it silently would be worse: an unscored finding nobody
  // sees is a finding nobody can question. So it is set aside BEFORE either
  // pass and reported by name, on the same footing as `excluded`.
  //
  // A collateral declaration is narrow on purpose: it names the family and the
  // reason, so it cannot quietly absorb an unrelated false positive that
  // happens to fire on the same corpus.
  // ONE finding per declaration, consumed like a label. Without this a single
  // `no-symbol-bound` declaration would absorb every `no-symbol-bound` finding
  // the run emitted — so a toolchain reporting the same consequence five times
  // would score identically to one reporting it once, and four duplicates
  // would vanish from the precision denominator. That is the laundering
  // CR-098's positional pairing was added to stop, reintroduced through a side
  // door.
  const declaredCollateral = labels.flatMap((l) => l.collateral ?? []);
  const spent = new Set();
  const collateral = [];
  const setAside = new Set();
  for (const finding of found) {
    const index = declaredCollateral.findIndex(
      (c, i) =>
        !spent.has(i) &&
        c.family === finding.family &&
        (c.reason === undefined || c.reason === findingReason(finding)),
    );
    if (index === -1) continue;
    spent.add(index);
    setAside.add(finding);
    collateral.push({
      family: finding.family,
      reason: findingReason(finding),
    });
  }
  const scored = found.filter((f) => !setAside.has(f));

  // Pass 1 — positional. A finding that names WHERE is paired to the label at
  // that place. Run first so a locus-less finding cannot consume the label a
  // positioned one should have taken (FR-043-AC-7).
  for (const finding of scored) {
    const locus = locusOf(finding);
    if (!locus) continue;
    const hit = expected.find(
      (l) =>
        !matched.has(l.id) &&
        l.family === finding.family &&
        locusMatches(locus, labelLocus(l)),
    );
    if (hit) {
      matched.add(hit.id);
      claimed.add(finding);
      positional += 1;
      bucket(hit.family).truePositives += 1;
    }
  }

  // Pass 2 — family only, where there is no position to compare on.
  //
  // Matching on family alone was the ONLY rule before, and it laundered
  // duplicates: two findings of one family both scored true even when one
  // pointed somewhere no defect was seeded, reporting a toolchain as more
  // precise than it is.
  //
  // So a finding that DID name a place and matched no label there is a false
  // positive, and may only fall back to labels that name no place themselves —
  // otherwise pass 2 hands it the very label pass 1 refused it. A finding with
  // no locus is unconstrained, because it made no positional claim to be wrong
  // about.
  for (const finding of scored) {
    if (claimed.has(finding)) continue;
    const positioned = locusOf(finding) !== null;
    const hit = expected.find(
      (l) =>
        !matched.has(l.id) &&
        l.family === finding.family &&
        !(positioned && labelLocus(l) !== null),
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
  // How much of the score was positional. A precision figure built entirely
  // from family-only matches is weaker evidence than the same figure built
  // from findings that named where, and the reader should be able to tell.
  return { families: rows, excluded, positional, collateral };
}

/** The reason a finding carries, under either of the two payload spellings. */
function findingReason(finding) {
  return finding.reason ?? finding.kind ?? null;
}

/** Where a FINDING points, or `null` when it names no place. */
function locusOf(finding) {
  const path = finding.path ?? finding.document ?? finding.file ?? null;
  if (!path) return null;
  return {
    path: String(path),
    line: typeof finding.line === "number" ? finding.line : null,
  };
}

/** Where a LABEL says the defect is: `path:line`, or a bare path. */
function labelLocus(label) {
  if (!label.location) return null;
  const at = /^(.*):(\d+)$/.exec(String(label.location));
  return at
    ? { path: at[1], line: Number(at[2]) }
    : { path: String(label.location), line: null };
}

/**
 * Same place, tolerant of one side carrying a longer path prefix and of a
 * label that names a file without a line.
 *
 * A line is compared only when BOTH sides carry one: a label pinned to
 * `spec/FR-001.md` with no line is a claim about the file, and demanding a
 * line the label never made would turn every such label into a miss.
 */
function locusMatches(finding, label) {
  if (!finding || !label) return false;
  if (
    !finding.path.endsWith(label.path) &&
    !label.path.endsWith(finding.path)
  ) {
    return false;
  }
  if (finding.line !== null && label.line !== null) {
    return finding.line === label.line;
  }
  return true;
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
