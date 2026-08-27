import { isFindingEnvelope } from "../../evals/lib/finding-envelope.mjs";

/** Detection recall over failures, partitioned by level, mode, language, and family. */
export function detectionRecall(corpora, findings, gapCount, payloads = []) {
  const groups = new Map();
  const payloadByCorpus = new Map(payloads.map((item) => [item.name, item]));
  const groupFor = (corpus, family) => {
    const key = `${corpus.mode}\0${corpus.language}\0${family}`;
    const group = groups.get(key) ?? {
      mode: corpus.mode,
      language: corpus.language,
      family,
      population: 0,
      reached: { L1: 0, L2: 0, L3: 0 },
      misses: { L1: [], L2: [], L3: [] },
    };
    groups.set(key, group);
    return group;
  };

  for (const corpus of corpora) {
    if (corpus.kind !== "failure" || corpus.findable === false) continue;
    const declared = corpus.defects.filter(
      (defect) => defect.findable !== false,
    );
    const observation = directObservation(
      corpus.observations,
      payloadByCorpus.get(corpus.name),
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
    // Some payload channels are already located findings but are not
    // diagnostic families. Count the exact assertion the corpus grades rather
    // than turning it into an unlabeled L1 miss.
    if (declared.length === 0 && observation) {
      const group = groupFor(corpus, "direct-observation");
      group.population += 1;
      for (const [level, reached] of [
        ["L1", observation.l1],
        ["L2", observation.l2],
        ["L3", false],
      ]) {
        if (reached) group.reached[level] += 1;
        else group.misses[level].push(corpus.name);
      }
      continue;
    }

    for (const label of labels) {
      const family = label.family ?? "unclaimed";
      const group = groupFor(corpus, family);
      group.population += 1;
      const candidates = findings.filter(
        (finding) =>
          findingCase(finding) === corpus.name &&
          findingFamily(finding) === label.family,
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
              findingText(finding).includes(fragment),
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
  }

  return [...groups.values()]
    .sort(
      (a, b) =>
        a.mode.localeCompare(b.mode) ||
        a.language.localeCompare(b.language) ||
        a.family.localeCompare(b.family),
    )
    .flatMap((group) =>
      ["L1", "L2", "L3"].map((level) => ({
        mode: group.mode,
        language: group.language,
        family: group.family,
        level,
        reached: group.reached[level],
        population: group.population,
        rate: ratio(group.reached[level], group.population),
        gap_count: gapCount,
        misses: [...new Set(group.misses[level])].sort(),
      })),
    );
}

/** Named L2/L3 misses with the evidence needed to assign each repair. */
export function localityMissInventory(corpora, findings, payloads = []) {
  const payloadByCorpus = new Map(payloads.map((item) => [item.name, item]));
  const out = [];

  for (const corpus of corpora) {
    if (corpus.kind !== "failure" || corpus.findable === false) continue;
    const declared = corpus.defects.filter(
      (defect) => defect.findable !== false,
    );
    const observation = directObservation(
      corpus.observations,
      payloadByCorpus.get(corpus.name),
    );
    if (declared.length === 0 && observation) {
      out.push({
        id: `${corpus.name}:direct-observation`,
        case: corpus.name,
        mode: corpus.mode,
        language: corpus.language,
        family: "direct-observation",
        producer: ["quire:coverage.untracked_symbols"],
        expectedLocus: expectedObservationLoci(corpus.observations),
        observedLocus: observedObservationLoci(
          payloadByCorpus.get(corpus.name),
        ),
        missingLevels: [
          ...(!observation.l1 ? ["L1"] : []),
          ...(!observation.l2 ? ["L2"] : []),
          "L3",
        ],
        rootCause: !observation.l1
          ? "the producer emitted no direct observation for the controlled case"
          : !observation.l2
            ? "the payload was present but did not equal the controlled expected locus"
            : "the direct observation has no controlled actionable-fragment contract",
        disposition: deferredDisposition(),
      });
      continue;
    }

    const labels = declared.length
      ? declared
      : [
          {
            id: `${corpus.name}-unclaimed`,
            family: null,
            location: null,
            actionable_fragments: [],
          },
        ];
    for (const label of labels) {
      const candidates = findings.filter(
        (finding) =>
          findingCase(finding) === corpus.name &&
          findingFamily(finding) === label.family,
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
              findingText(finding).includes(fragment),
            ),
        );
      if (l2 && l3) continue;
      const missingLevels = [
        ...(!l1 ? ["L1"] : []),
        ...(!l2 ? ["L2"] : []),
        ...(!l3 ? ["L3"] : []),
      ];
      out.push({
        id: `${corpus.name}:${label.id}`,
        case: corpus.name,
        mode: corpus.mode,
        language: corpus.language,
        family: label.family ?? "unclaimed",
        producer: [
          ...new Set(candidates.map(findingProducer).filter(Boolean)),
        ].sort(),
        expectedLocus: label.location ?? null,
        observedLocus: [
          ...new Set(candidates.map(findingLocus).filter(Boolean)),
        ].sort(),
        missingLevels,
        rootCause: !l1
          ? "the producer emitted no finding in the expected family"
          : !l2
            ? "the family finding did not match the controlled expected locus"
            : "the located finding omitted one or more controlled causal fragments",
        disposition: deferredDisposition(),
      });
    }
  }
  return out.sort((a, b) => a.id.localeCompare(b.id));
}

function directObservation(expected = {}, payload = {}) {
  const expectedUntracked = expected?.untracked_symbols ?? [];
  if (expectedUntracked.length) {
    const actual = (payload?.untrackedSymbols ?? []).map((item) => ({
      symbol: item.symbol,
      trace_id: item.trace_id,
      path: item.path,
    }));
    return {
      l1: actual.length > 0,
      l2: JSON.stringify(actual) === JSON.stringify(expectedUntracked),
    };
  }

  return null;
}

/** One-way ratchet: lower recall regresses; higher recall asks for re-baseline. */
export function recallVerdicts(rows, baselineRows = []) {
  const key = (row) =>
    `${row.mode}\0${row.language}\0${row.family ?? "legacy-group"}\0${row.level}`;
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
  const normalizedLocus = isFindingEnvelope(finding)
    ? finding.locus.state === "available"
      ? finding.locus.value
      : {}
    : finding;
  const actualPath =
    normalizedLocus.path ??
    normalizedLocus.document ??
    normalizedLocus.file ??
    null;
  if (!actualPath) return false;
  if (
    !String(actualPath).endsWith(expectedPath) &&
    !expectedPath.endsWith(String(actualPath))
  ) {
    return false;
  }
  return (
    expectedLine === null ||
    typeof normalizedLocus.line !== "number" ||
    normalizedLocus.line === expectedLine
  );
}

function findingCase(finding) {
  return isFindingEnvelope(finding) ? finding.identity?.case : finding.corpus;
}

function findingFamily(finding) {
  return isFindingEnvelope(finding) ? finding.identity?.family : finding.family;
}

function findingText(finding) {
  if (!isFindingEnvelope(finding)) {
    return `${finding.message ?? ""}\n${finding.evidence ?? ""}`;
  }
  return finding.causalEvidence.state === "available"
    ? typeof finding.causalEvidence.value === "string"
      ? finding.causalEvidence.value
      : JSON.stringify(finding.causalEvidence.value)
    : "";
}

function findingProducer(finding) {
  return isFindingEnvelope(finding)
    ? `${finding.source?.producer ?? "unknown"}:${finding.source?.channel ?? "unknown"}`
    : (finding.producer ?? null);
}

function findingLocus(finding) {
  const locus = isFindingEnvelope(finding)
    ? finding.locus.state === "available"
      ? finding.locus.value
      : null
    : finding;
  const path = locus?.path ?? locus?.document ?? locus?.file ?? null;
  if (!path) return null;
  return typeof locus.line === "number"
    ? `${path}:${locus.line}`
    : String(path);
}

function expectedObservationLoci(observations = {}) {
  return [
    ...new Set((observations.untracked_symbols ?? []).map((item) => item.path)),
  ].sort();
}

function observedObservationLoci(payload = {}) {
  return [
    ...new Set((payload.untrackedSymbols ?? []).map((item) => item.path)),
  ].sort();
}

function deferredDisposition() {
  return {
    state: "deferred",
    reason:
      "retained after the #236/#341 implementation slice; no additional producer repair is claimed by #257",
  };
}

function ratio(numerator, denominator) {
  return denominator === 0
    ? null
    : Number((numerator / denominator).toFixed(3));
}
