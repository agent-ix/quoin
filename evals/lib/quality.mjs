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

import {
  isFindingEnvelope,
  validateFindingEnvelope,
} from "./finding-envelope.mjs";

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
export function scoreFindings(found, labels, shapes = {}, adjudication = {}) {
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
  //
  // ONE finding per declaration was not enough: it must also be ONE CASE.
  // Measured over a 34-case corpus (agent-ix/quoin#238), five cases declared
  // `hollow-denominator` collateral and the run emitted exactly five
  // `hollow-denominator` findings — one of which was the seeded, labelled
  // defect of a SIXTH case. All five were absorbed, and that family scored
  // recall 0.00 for the whole run while the per-language cut of the same run
  // scored 1.00. The declaring case is carried onto the entry here so the
  // predicate can require it.
  const declaredCollateral = labels.flatMap((l) =>
    (l.collateral ?? []).map((c) => ({ ...c, corpus: l.corpus })),
  );
  const spent = new Set();
  const collateral = [];
  const setAside = new Set();
  for (const finding of found) {
    const index = declaredCollateral.findIndex(
      (c, i) =>
        !spent.has(i) &&
        c.family === findingFamily(finding) &&
        (c.reason === undefined || c.reason === findingReason(finding)) &&
        // Compared only when BOTH sides name a case. A caller that scores a
        // flat finding list with no corpus attribution — the eval harness
        // does — keeps the old, looser pairing rather than losing collateral
        // suppression entirely.
        (c.corpus === undefined ||
          findingCase(finding) === undefined ||
          c.corpus === findingCase(finding)),
    );
    if (index === -1) continue;
    spent.add(index);
    setAside.add(finding);
    collateral.push({
      family: findingFamily(finding),
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
        l.family === findingFamily(finding) &&
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
        l.family === findingFamily(finding) &&
        !(positioned && labelLocus(l) !== null),
    );
    if (hit) {
      matched.add(hit.id);
      bucket(hit.family).truePositives += 1;
    } else {
      bucket(findingFamily(finding)).falsePositives += 1;
    }
  }
  for (const label of expected) {
    if (!matched.has(label.id)) bucket(label.family).misses += 1;
  }

  const rows = [...families.values()]
    .map((f) => {
      // An ADVISORY family is not scored by the defect-shaped FP definition.
      // It is a corpus-level observation — one finding per corpus whenever a
      // shape holds — so it fires on nearly every fixture, correctly, and
      // "a finding on a case that does not seed this family" counts each
      // correct firing against it.
      //
      // Measured before this: `catch-all-universal` fired on 10 of 21 cases,
      // every one verified correct, and scored precision 0.167 — a number that
      // read as "wrong five times in six" while being wrong zero times in
      // twelve (agent-ix/quoin#234).
      //
      // What #234 shipped was a bare `null`, and `ratchet()` skips a null
      // metric — so reclassifying a family to `advisory` deleted its precision
      // in silence, for the cost of one string in one JSON file. That was then
      // done a SECOND time, to `archetype-matches-nothing` at 3 TP / 296 FP =
      // 0.01, and no run objected (agent-ix/quoin#245).
      //
      // So the null is now SCOPED and ACCOUNTED. Precision is computed over
      // the cases the corpus has actually ruled on, and every firing nobody
      // ruled on is counted and published rather than folded into a blank.
      const advisory = shapes[f.family] === "advisory";
      const scoped = advisory
        ? scopedPrecision(f.family, scored, adjudication)
        : null;
      return {
        ...f,
        precision: advisory
          ? scoped.precision
          : ratio(f.truePositives, f.truePositives + f.falsePositives),
        shape: shapes[f.family] ?? "defect",
        recall: ratio(f.truePositives, f.truePositives + f.misses),
        ...(advisory ? { precision_basis: scoped } : {}),
      };
    })
    .sort((a, b) => a.family.localeCompare(b.family));

  // Labels declared unfindable are reported, not silently dropped: an
  // excluded denominator nobody sees is a denominator nobody can question.
  const excluded = labels.filter((l) => l.findable === false).map((l) => l.id);
  // How much of the score was positional. A precision figure built entirely
  // from family-only matches is weaker evidence than the same figure built
  // from findings that named where, and the reader should be able to tell.
  return { families: rows, excluded, positional, collateral };
}

/**
 * Precision for an ADVISORY family, over the cases the corpus has ruled on.
 *
 * A case's `expect.yaml` states a reason under `diagnostic_reasons` (it must
 * fire here) or `absent_diagnostic_reasons` (it must stay silent here). Those
 * are the only two places this corpus says anything about an advisory. So:
 *
 *   true positive   fired on a case that declared the reason PRESENT
 *   false positive  fired on a case that declared the reason ABSENT
 *   unadjudicated   fired on a case that declared neither
 *
 * A ruling may be SCOPED to one declaration — `test-case/...` — and then it
 * governs only findings that declaration raised. `agent-ix/quire-rs#304` made
 * `archetype-matches-nothing` fire for several declarations at once, so on a
 * three-file fixture it fires correctly for `inspection`, `suite`,
 * `nfr-acceptance-criterion` and `stakeholder-validation-criterion` — none of
 * which the fixture is about. Reading a scoped ruling as a ruling on the bare
 * token counted those four as false positives and produced a precision of
 * 0.556 that no reading of the corpus supports.
 *
 * READ THE COVERAGE, NOT THE RATE. `absent_diagnostic_reasons` is graded by
 * `qa-corpus`'s own `make ci`, so a false positive here would already have
 * turned that gate red — which means this rate cannot fall independently of a
 * gate one repository over, and a 1.00 is not evidence the detector is right.
 * What this function actually reports is `unadjudicated`: the number of times
 * the toolchain fired and nobody has ruled on whether it should have. On the
 * baseline that opened agent-ix/quoin#245 that count was 313 of 316 for
 * `archetype-matches-nothing`, and it was invisible.
 *
 * `null`, never 0, when nothing has been adjudicated: not-measured and zero are
 * different claims, and a family nobody has ruled on has no precision at all.
 */
function scopedPrecision(family, scored, adjudication) {
  const ruling = adjudication[family] ?? { present: [], absent: [] };
  // A ruling governs a finding when they name the same case AND the ruling is
  // either unscoped or names the declaration that raised it.
  const matching = (rules, finding) =>
    rules.find(
      (r) =>
        r.corpus === findingCase(finding) &&
        (r.scope === null || r.scope === findingDeclaration(finding)),
    );
  let truePositives = 0;
  let falsePositives = 0;
  let unadjudicated = 0;
  // Of the true positives, how many were ruled by a STANDING sentence about
  // the whole corpus rather than by the fixture's own `expect.yaml`. Reported
  // because they are different strengths of evidence: the first measurement
  // after `archetype-matches-nothing` gained a standing ruling read precision
  // 1.00 over 323 firings, of which 3 were per-case and 320 came from one
  // sentence. Averaging those into a single figure is how a rate stops meaning
  // what its name says.
  let byStanding = 0;
  for (const finding of scored) {
    if (findingFamily(finding) !== family) continue;
    // A finding with no case attribution cannot be ruled on at all. Counted as
    // unadjudicated rather than dropped: an unscored finding nobody sees is a
    // finding nobody can question, which is the rule the collateral pass is
    // held to a few lines up.
    if (findingCase(finding) === undefined) {
      unadjudicated += 1;
      continue;
    }
    if (matching(ruling.absent ?? [], finding)) {
      falsePositives += 1;
      continue;
    }
    const hit = matching(ruling.present ?? [], finding);
    if (hit) {
      truePositives += 1;
      if (hit.standing) byStanding += 1;
    } else {
      unadjudicated += 1;
    }
  }
  return {
    precision: ratio(truePositives, truePositives + falsePositives),
    truePositives,
    falsePositives,
    unadjudicated,
    byStanding,
    // The rulings that exist AT ALL, whether or not the family fired under
    // them. A family can be adjudicated on ten cases and fire under none of
    // them, and that is a different state from one nobody has written a rule
    // for.
    rulings: (ruling.present ?? []).length + (ruling.absent ?? []).length,
  };
}

/** The reason a finding carries, under either of the two payload spellings. */
function findingReason(finding) {
  return finding.kind ?? finding.reason ?? null;
}

function findingFamily(finding) {
  return isFindingEnvelope(finding) ? finding.identity?.family : finding.family;
}

function findingCase(finding) {
  return isFindingEnvelope(finding) ? finding.identity?.case : finding.corpus;
}

function findingDeclaration(finding) {
  return isFindingEnvelope(finding)
    ? finding.identity?.declaration
    : finding.declaration;
}

/** Where a FINDING points, or `null` when it names no place. */
function locusOf(finding) {
  if (isFindingEnvelope(finding)) {
    const locus = finding.locus;
    if (locus.state !== "available") return null;
    const path = locus.value?.path;
    if (!path) return null;
    return {
      path: String(path),
      line: typeof locus.value?.line === "number" ? locus.value.line : null,
    };
  }
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
    definitionVersion: "finding.actionability-v1",
    numerator: actionable,
    denominator: found.length,
    exclusions: [],
    namedMisses: [],
    actionable,
    total: found.length,
    rate: ratio(actionable, found.length),
  };
}

/**
 * Actionability v2: correct subject/locus, causal evidence, a concrete change
 * target, and either a remedy or a safe next diagnostic step (#254).
 *
 * `not_applicable` removes a record from this metric and remains named in the
 * exclusions. `unavailable` stays in the denominator and is a named miss: not
 * emitted is evidence about the producer, not permission to shrink the score.
 */
export function scoreActionabilityV2(found) {
  const namedMisses = [];
  const exclusions = [];
  let numerator = 0;
  let denominator = 0;

  found.forEach((finding, index) => {
    validateFindingEnvelope(finding);
    const id = findingIdentity(finding, index);
    const subjectOrLocus =
      finding.subject.state === "available" ||
      finding.locus.state === "available";
    const required = [
      ["causal_evidence", finding.causalEvidence],
      ["change_target", finding.changeTarget],
      ["next_move", finding.nextMove],
    ];
    const notApplicable = required
      .filter(([, value]) => value.state === "not_applicable")
      .map(([field, value]) => ({ field, reason: value.reason }));
    const locusNotApplicable =
      finding.subject.state === "not_applicable" &&
      finding.locus.state === "not_applicable";
    if (locusNotApplicable) {
      notApplicable.unshift({
        field: "subject_or_locus",
        reason: `${finding.subject.reason}; ${finding.locus.reason}`,
      });
    }
    if (notApplicable.length > 0) {
      exclusions.push({ id, fields: notApplicable });
      return;
    }

    denominator += 1;
    const missing = [];
    if (!subjectOrLocus) {
      missing.push({
        field: "subject_or_locus",
        state: "unavailable",
        reason: `${finding.subject.reason}; ${finding.locus.reason}`,
      });
    }
    for (const [field, value] of required) {
      if (value.state !== "available") {
        missing.push({ field, state: value.state, reason: value.reason });
      }
    }
    if (missing.length === 0) numerator += 1;
    else namedMisses.push({ id, missing });
  });

  return {
    definitionVersion: "finding.actionability-v2",
    numerator,
    denominator,
    exclusions,
    namedMisses,
    rate: ratio(numerator, denominator),
  };
}

const SPECIFIC_PROPERTIES = new Set([
  "round-trip",
  "idempotence",
  "ordering",
  "invariant",
  "error-case",
  "lifecycle",
  "concurrency",
]);

/** Span-presence grounding over structured `quire properties --json` output. */
export function scoreSpanGrounding(inputs) {
  const namedMisses = [];
  const exclusions = [];
  const malformed = [];
  const producerVersions = new Set();
  const spanStates = Object.fromEntries(
    ["domain", "precondition", "oracle"].map((field) => [
      field,
      { available: 0, unavailable: 0, missing: 0, malformed: 0 },
    ]),
  );
  let numerator = 0;
  let denominator = 0;

  for (const input of inputs) {
    const payload = input?.payload;
    const producerVersion =
      input?.producerVersion ??
      (payload?.engine
        ? `${payload.engine.cli ?? "unknown-cli"} (engine ${payload.engine.engine ?? "unknown"})`
        : null);
    if (producerVersion) producerVersions.add(producerVersion);
    if (!payload || !Array.isArray(payload.documents)) {
      malformed.push({
        case: input?.case ?? "unknown-case",
        reason: "properties payload has no documents array",
      });
      continue;
    }

    payload.documents.forEach((document, documentIndex) => {
      if (!Array.isArray(document?.criteria)) {
        malformed.push({
          case: input?.case ?? "unknown-case",
          document: document?.document ?? `document-${documentIndex + 1}`,
          reason: "properties document has no criteria array",
        });
        return;
      }
      document.criteria.forEach((criterion, criterionIndex) => {
        const id = criterionIdentity(
          input,
          document,
          criterion,
          criterionIndex,
        );
        if (!SPECIFIC_PROPERTIES.has(criterion?.property)) {
          exclusions.push({
            id,
            state: "not_applicable",
            reason: `property ${JSON.stringify(criterion?.property ?? null)} is outside the specific-shape population`,
          });
          return;
        }

        denominator += 1;
        const spans = Object.fromEntries(
          ["domain", "precondition", "oracle"].map((field) => {
            const observed = classifySpan(criterion, field);
            spanStates[field][observed.state] += 1;
            return [field, observed];
          }),
        );
        if (Object.values(spans).every((span) => span.state === "available")) {
          numerator += 1;
        } else {
          namedMisses.push({
            id,
            case: input?.case ?? null,
            document: document.document ?? null,
            rowId: criterion.row_id ?? null,
            line: criterion.line ?? null,
            property: criterion.property,
            spans,
          });
        }
      });
    });
  }

  return {
    definitionVersion: "property.span-grounding-v1",
    numerator,
    denominator,
    exclusions,
    namedMisses,
    malformed,
    spanStates,
    producerVersions: [...producerVersions].sort(),
    rate: ratio(numerator, denominator),
  };
}

/** Exact controlled-locus scoring, kept separate from span presence. */
export function scoreGroundingQuality(payload, labelSet) {
  const malformed = [];
  const criteria = new Map();
  for (const document of payload?.documents ?? []) {
    for (const criterion of document?.criteria ?? []) {
      if (!criterion?.row_id) continue;
      if (criteria.has(criterion.row_id)) {
        malformed.push({
          rowId: criterion.row_id,
          reason: "duplicate controlled criterion row id",
        });
      } else {
        criteria.set(criterion.row_id, criterion);
      }
    }
  }

  const correctness = qualityAxis("property.span-correctness-v1");
  const safeRefusal = qualityAxis("property.safe-refusal-v1");
  const tradeoff = {
    correctSpans: 0,
    wrongSpans: 0,
    safeRefusals: 0,
    unexpectedRefusals: 0,
    unsafeEmissions: 0,
  };

  for (const label of labelSet?.cases ?? []) {
    const criterion = criteria.get(label.rowId) ?? null;
    if (label.expectedRefusal) {
      safeRefusal.denominator += 1;
      const family = familyAxis(safeRefusal, label.family);
      family.denominator += 1;
      const observed = observedSpans(criterion);
      const emitted = Object.values(observed).some((span) => span !== null);
      const reasons = Array.isArray(criterion?.signals)
        ? criterion.signals
        : [];
      const passed =
        criterion !== null &&
        !emitted &&
        reasons.includes(label.expectedRefusal);
      if (passed) {
        safeRefusal.numerator += 1;
        family.numerator += 1;
        tradeoff.safeRefusals += 1;
      } else {
        if (emitted) tradeoff.unsafeEmissions += 1;
        const miss = {
          id: label.id,
          family: label.family,
          rowId: label.rowId,
          expected: { refusal: label.expectedRefusal },
          observed: { spans: observed, signals: reasons },
          outcome: emitted ? "unsafe-emission" : "unjustified-refusal",
        };
        safeRefusal.namedMisses.push(miss);
        family.namedMisses.push(miss);
      }
      continue;
    }

    correctness.denominator += 1;
    const family = familyAxis(correctness, label.family);
    family.denominator += 1;
    const expected = expectedSpans(criterion, label, malformed);
    const observed = observedSpans(criterion);
    const emitted = Object.values(observed).some((span) => span !== null);
    const passed =
      expected !== null &&
      ["domain", "precondition", "oracle"].every((field) =>
        sameSpan(expected[field], observed[field]),
      );
    if (passed) {
      correctness.numerator += 1;
      family.numerator += 1;
      tradeoff.correctSpans += 1;
    } else {
      if (emitted) tradeoff.wrongSpans += 1;
      else tradeoff.unexpectedRefusals += 1;
      const miss = {
        id: label.id,
        family: label.family,
        rowId: label.rowId,
        expected,
        observed,
        outcome: emitted ? "wrong-span" : "unexpected-refusal",
      };
      correctness.namedMisses.push(miss);
      family.namedMisses.push(miss);
    }
  }

  finishQualityAxis(correctness);
  finishQualityAxis(safeRefusal);
  return {
    definitionVersion:
      labelSet?.definitionVersion ?? "property.grounding-loci-v1",
    correctness,
    safeRefusal,
    tradeoff,
    malformed,
    producerVersions: payload?.engine
      ? [
          `${payload.engine.cli ?? "unknown-cli"} (engine ${payload.engine.engine ?? "unknown"})`,
        ]
      : [],
  };
}

function qualityAxis(definitionVersion) {
  return {
    definitionVersion,
    numerator: 0,
    denominator: 0,
    rate: null,
    namedMisses: [],
    families: [],
    _families: new Map(),
  };
}

function familyAxis(axis, name) {
  if (!axis._families.has(name)) {
    const family = {
      family: name,
      numerator: 0,
      denominator: 0,
      rate: null,
      namedMisses: [],
    };
    axis._families.set(name, family);
    axis.families.push(family);
  }
  return axis._families.get(name);
}

function finishQualityAxis(axis) {
  axis.rate = ratio(axis.numerator, axis.denominator);
  axis.families.sort((a, b) => a.family.localeCompare(b.family));
  for (const family of axis.families) {
    family.rate = ratio(family.numerator, family.denominator);
  }
  delete axis._families;
}

function expectedSpans(criterion, label, malformed) {
  if (!criterion || typeof criterion.statement !== "string") {
    malformed.push({
      id: label.id,
      rowId: label.rowId,
      reason: "controlled criterion or statement is unavailable",
    });
    return null;
  }
  const out = {};
  for (const field of ["domain", "precondition", "oracle"]) {
    const text = label.expected?.[field] ?? null;
    if (text === null) {
      out[field] = null;
      continue;
    }
    const start = criterion.statement.indexOf(text);
    const repeated =
      start >= 0 && criterion.statement.indexOf(text, start + 1) >= 0;
    if (start < 0 || repeated) {
      malformed.push({
        id: label.id,
        rowId: label.rowId,
        field,
        reason:
          start < 0
            ? "expected span text is absent from the statement"
            : "expected span text is not unique in the statement",
      });
      return null;
    }
    out[field] = { start, end: start + text.length, text };
  }
  return out;
}

function observedSpans(criterion) {
  return Object.fromEntries(
    ["domain", "precondition", "oracle"].map((field) => [
      field,
      criterion?.[field] ?? null,
    ]),
  );
}

function sameSpan(expected, observed) {
  if (expected === null || observed === null) return expected === observed;
  return (
    expected.start === observed.start &&
    expected.end === observed.end &&
    expected.text === observed.text
  );
}

function classifySpan(criterion, field) {
  if (!Object.hasOwn(criterion, field)) {
    return { state: "missing", reason: `criterion has no ${field} field` };
  }
  const value = criterion[field];
  if (value === null) {
    return {
      state: "unavailable",
      reason: `producer emitted no ${field} span`,
    };
  }
  if (
    !value ||
    typeof value !== "object" ||
    !Number.isInteger(value.start) ||
    !Number.isInteger(value.end) ||
    value.start < 0 ||
    value.end < value.start ||
    typeof value.text !== "string"
  ) {
    return {
      state: "malformed",
      reason: `producer emitted malformed ${field} span`,
    };
  }
  return { state: "available", value };
}

function criterionIdentity(input, document, criterion, index) {
  return [
    input?.case ?? "unknown-case",
    document?.document ?? "unknown-document",
    criterion?.row_id ?? criterion?.line ?? `criterion-${index + 1}`,
  ].join(":");
}

/** A finding is actionable when it names WHERE — a row id or a document line. */
function hasLocus(finding) {
  if (isFindingEnvelope(finding)) {
    if (finding.locus.state !== "available") return false;
    return Boolean(finding.locus.value?.rowId || finding.locus.value?.line);
  }
  return Boolean(
    (finding.rowId && String(finding.rowId).trim()) ||
    (typeof finding.line === "number" && finding.line > 0),
  );
}

function findingIdentity(finding, index) {
  const parts = [
    finding.source.producer,
    finding.source.channel,
    finding.identity?.case,
    finding.identity?.family,
    finding.kind,
  ].filter(Boolean);
  return parts.length > 0 ? parts.join(":") : `finding-${index + 1}`;
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
  // ABSENT IS NOT ZERO. `?? 0` here published "0 tokens" for a runner that
  // spends none because it runs no model at all — tier 1 shells out to `quire`
  // and `quoin` and never calls one. A zero is a measurement claiming the run
  // was free; `null` says nobody counted, which is the same rule `ratio` is
  // held to two functions down (agent-ix/quoin#243).
  const tokens = metrics?.tokenUsage?.total ?? null;
  const toolCalls = metrics?.toolCalls ?? null;
  // THE TWO HALVES ARE COMPUTED SEPARATELY. Tier 1 knows its tool calls exactly
  // and has no token count at all, and an all-or-nothing return threw away the
  // half it could report. Each side is null only when its own input is.
  //
  // Not Infinity and not 0 when nothing was confirmed: a run that confirmed
  // nothing has no cost PER insight, and either number would be a claim about
  // efficiency the run does not support.
  const per = (total) =>
    total === null || truePositives === 0 ? null : total / truePositives;
  const tokensPer = per(tokens);
  const toolCallsPer = per(toolCalls);
  return {
    tokens,
    toolCalls,
    truePositives,
    tokensPer: tokensPer === null ? null : Math.round(tokensPer),
    toolCallsPer:
      toolCallsPer === null ? null : Number(toolCallsPer.toFixed(2)),
  };
}

/** Every quality dimension for one scenario, ready to sit beside the cost columns. */
export function scoreScenario({
  found = [],
  labels = [],
  metrics = {},
  properties = [],
}) {
  const findings = scoreFindings(found, labels);
  const truePositives = findings.families.reduce(
    (n, f) => n + f.truePositives,
    0,
  );
  return {
    findings,
    actionability: scoreActionability(found),
    spanGrounding: scoreSpanGrounding(properties),
    cost: scoreCost(metrics, truePositives),
  };
}

/** `null` rather than 0 when there is no denominator — 0/0 is not 0%. */
function ratio(num, den) {
  return den === 0 ? null : Number((num / den).toFixed(3));
}
