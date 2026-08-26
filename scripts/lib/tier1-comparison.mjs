/** Compare one observation with its baseline using the declared direction. */
export function compare(direction, observed, baseline) {
  if (direction === "gate-zero") {
    return [observed === 0 ? "held" : "regressed", 0];
  }
  if (baseline === null || baseline === undefined) return ["new", observed];
  const better =
    direction === "higher-is-better"
      ? observed > baseline
      : observed < baseline;
  if (better) return ["improved", observed];
  if (observed === baseline) return ["held", baseline];
  return ["regressed", baseline];
}

/** Identify report inputs that must remain fixed for a meaningful delta. */
export function comparability(report, previous) {
  const reasons = [];
  const unknown = [];
  const check = (field, observed, baseline) => {
    if (baseline === undefined || baseline === null) return unknown.push(field);
    if (JSON.stringify(observed) !== JSON.stringify(baseline)) {
      reasons.push({ field, baseline, observed });
    }
  };
  const languages = (value) =>
    Object.fromEntries(
      (value.by_language ?? []).map((row) => [row.language, row.corpora]),
    );
  if (!previous) return { comparable: true, reasons, unknown };
  check(
    "provenance.declaration.digest",
    report.provenance?.declaration?.digest ?? null,
    previous.provenance?.declaration?.digest ?? null,
  );
  check(
    "provenance.corpus_input",
    report.provenance?.corpus_input ?? report.provenance?.corpus ?? null,
    report.provenance?.corpus_input
      ? previous.provenance?.corpus_input
      : previous.provenance?.corpus,
  );
  check("corpora", report.corpora ?? null, previous.corpora ?? null);
  check(
    "by_language",
    languages(report),
    previous.by_language ? languages(previous) : null,
  );
  return { comparable: reasons.length === 0, reasons, unknown };
}

/** Produce one-way metric verdicts and retain the old baseline on regression. */
export function ratchet(report, previous, dictionary) {
  const out = [];
  const before = new Map((previous?.families ?? []).map((f) => [f.family, f]));
  for (const family of report.families) {
    for (const metric of ["precision", "recall"]) {
      const baselineValue = before.get(family.family)?.[metric] ?? null;
      if (family[metric] === null) {
        if (baselineValue === null || baselineValue === undefined) continue;
        out.push({
          metric: `finding_${metric}`,
          family: family.family,
          observed: null,
          baseline: baselineValue,
          verdict: "regressed",
          why:
            `the baseline measured this family at ${baselineValue} and this ` +
            "run reports no denominator; a metric that stopped being measured has not held",
        });
        continue;
      }
      const [verdict, kept] = compare(
        dictionary.metrics[`finding_${metric}`].direction,
        family[metric],
        baselineValue,
      );
      out.push({
        metric: `finding_${metric}`,
        family: family.family,
        observed: family[metric],
        baseline: kept,
        verdict,
      });
    }
    const unadjudicated = family.precision_basis?.unadjudicated;
    if (unadjudicated !== undefined) {
      const [verdict, kept] = compare(
        "lower-is-better",
        unadjudicated,
        before.get(family.family)?.precision_basis?.unadjudicated ?? null,
      );
      out.push({
        metric: "finding_precision.unadjudicated",
        family: family.family,
        observed: unadjudicated,
        baseline: kept,
        verdict,
      });
    }
  }

  const currentFamilies = new Set(report.families.map((f) => f.family));
  for (const family of previous?.families ?? []) {
    if (currentFamilies.has(family.family)) continue;
    out.push({
      metric: "finding_recall",
      family: family.family,
      observed: null,
      baseline: family.recall,
      verdict: "regressed",
      why: "the baseline scored this family and this run did not report it at all",
    });
  }

  if (report.finding_localisation_rate !== null) {
    const [verdict, kept] = compare(
      "higher-is-better",
      report.finding_localisation_rate,
      previous?.finding_localisation_rate ?? null,
    );
    out.push({
      metric: "finding_localisation_rate",
      family: null,
      observed: report.finding_localisation_rate,
      baseline: kept,
      verdict,
    });
  }

  if (report.actionability && report.actionability.rate !== null) {
    const [verdict, kept] = compare(
      dictionary.metrics.actionability_rate.direction,
      report.actionability.rate,
      previous?.actionability?.rate ?? null,
    );
    out.push({
      metric: "actionability_rate",
      family: null,
      observed: report.actionability.rate,
      baseline: kept,
      verdict,
    });
  }

  const sentinel = report["sentinel.silent_zero"];
  if (sentinel) {
    const [verdict, kept] = compare(
      dictionary.metrics["sentinel.silent_zero"].direction,
      sentinel.count,
      null,
    );
    out.push({
      metric: "sentinel.silent_zero",
      family: null,
      observed: sentinel.count,
      baseline: kept,
      verdict,
      ...(sentinel.count
        ? {
            why:
              "unaccompanied ratio-shaped zeroes: " +
              sentinel.instances
                .map((item) => `${item.corpus}/${item.metric}`)
                .join(", "),
          }
        : {}),
    });
  }

  const { comparable, reasons } = comparability(report, previous);
  if (comparable) return out;
  const why =
    "not compared: " +
    reasons
      .map(
        (reason) =>
          `${reason.field} moved (baseline ${short(reason.baseline)}, this run ${short(reason.observed)})`,
      )
      .join("; ");
  const actualBaseline = (verdict) => {
    const family = before.get(verdict.family);
    if (verdict.metric === "finding_precision")
      return family?.precision ?? null;
    if (verdict.metric === "finding_recall") return family?.recall ?? null;
    if (verdict.metric === "finding_precision.unadjudicated") {
      return family?.precision_basis?.unadjudicated ?? null;
    }
    if (verdict.metric === "finding_localisation_rate") {
      return previous?.finding_localisation_rate ?? null;
    }
    if (verdict.metric === "actionability_rate") {
      return previous?.actionability?.rate ?? null;
    }
    if (verdict.metric === "sentinel.silent_zero") return 0;
    return verdict.baseline;
  };
  return out.map((verdict) => ({
    ...verdict,
    baseline: actualBaseline(verdict),
    verdict: "incomparable",
    why,
  }));
}

function short(value) {
  if (typeof value === "string" && value.startsWith("sha256:")) {
    return `sha256:${value.slice(7, 19)}…`;
  }
  if (typeof value === "string" && /^[0-9a-f]{40}$/.test(value)) {
    return `${value.slice(0, 12)}…`;
  }
  return typeof value === "object" && value !== null
    ? JSON.stringify(value)
    : String(value);
}
