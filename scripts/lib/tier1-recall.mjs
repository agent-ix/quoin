/** Detection recall over seeded failures, partitioned by level, mode and language. */
export function detectionRecall(corpora, findings, gapCount) {
  const groups = new Map();
  const keyOf = (corpus) => `${corpus.mode}\0${corpus.language}`;

  for (const corpus of corpora) {
    if (corpus.kind !== "failure" || corpus.findable === false) continue;
    const declared = corpus.defects.filter(
      (defect) => defect.findable !== false,
    );
    // A findable failure with no positive detector label is an honest L1 miss,
    // not an absent denominator. This is the exact shape of the known
    // findable-but-undetected corpus gaps.
    const labels = declared.length
      ? declared
      : [
          {
            id: `${corpus.name}-unclaimed`,
            family: null,
            findable: true,
            location: null,
            actionable_fragments: [],
          },
        ];
    const key = keyOf(corpus);
    const group = groups.get(key) ?? {
      mode: corpus.mode,
      language: corpus.language,
      population: 0,
      reached: { L1: 0, L2: 0, L3: 0 },
      misses: { L1: [], L2: [], L3: [] },
    };

    for (const label of labels) {
      group.population += 1;
      const candidates = findings.filter(
        (finding) =>
          finding.corpus === corpus.name && finding.family === label.family,
      );
      const l1 = candidates.length > 0;
      const l2 =
        l1 &&
        Boolean(label.location) &&
        candidates.some((finding) => lociMatch(finding, label.location));
      const fragments = label.actionable_fragments ?? [];
      const l3 =
        l2 &&
        fragments.length > 0 &&
        candidates.some(
          (finding) =>
            lociMatch(finding, label.location) &&
            fragments.every((fragment) =>
              `${finding.message ?? ""}\n${finding.evidence ?? ""}`.includes(
                fragment,
              ),
            ),
        );
      for (const [level, reached] of [
        ["L1", l1],
        ["L2", l2],
        ["L3", l3],
      ]) {
        if (reached) group.reached[level] += 1;
        else group.misses[level].push(corpus.name);
      }
    }
    groups.set(key, group);
  }

  return [...groups.values()]
    .sort(
      (a, b) =>
        a.mode.localeCompare(b.mode) || a.language.localeCompare(b.language),
    )
    .flatMap((group) =>
      ["L1", "L2", "L3"].map((level) => ({
        mode: group.mode,
        language: group.language,
        level,
        reached: group.reached[level],
        population: group.population,
        rate: ratio(group.reached[level], group.population),
        gap_count: gapCount,
        misses: [...new Set(group.misses[level])].sort(),
      })),
    );
}

/** One-way ratchet: lower recall regresses; higher recall asks for re-baseline. */
export function recallVerdicts(rows, baselineRows = []) {
  const key = (row) => `${row.mode}\0${row.language}\0${row.level}`;
  const before = new Map(baselineRows.map((row) => [key(row), row]));
  return rows.map((row) => {
    const baseline = before.get(key(row));
    if (!baseline) return { ...row, baseline: null, verdict: "new" };
    if (baseline.population !== row.population) {
      return {
        ...row,
        baseline: baseline.rate,
        verdict: "incomparable",
        why: `population moved from ${baseline.population} to ${row.population}`,
      };
    }
    return {
      ...row,
      baseline: baseline.rate,
      verdict:
        row.rate > baseline.rate
          ? "improved"
          : row.rate < baseline.rate
            ? "regressed"
            : "held",
    };
  });
}

/** A recall gate is clean only after every observed partition is retained. */
export function recallGateFails(verdicts) {
  return verdicts.some((verdict) => verdict.verdict !== "held");
}

function lociMatch(finding, expected) {
  if (!expected) return false;
  const match = /^(.*):(\d+)$/.exec(String(expected));
  const expectedPath = match ? match[1] : String(expected);
  const expectedLine = match ? Number(match[2]) : null;
  const actualPath = finding.path ?? finding.document ?? finding.file ?? null;
  if (!actualPath) return false;
  if (
    !String(actualPath).endsWith(expectedPath) &&
    !expectedPath.endsWith(String(actualPath))
  ) {
    return false;
  }
  return (
    expectedLine === null ||
    typeof finding.line !== "number" ||
    finding.line === expectedLine
  );
}

function ratio(numerator, denominator) {
  return denominator === 0
    ? null
    : Number((numerator / denominator).toFixed(3));
}
