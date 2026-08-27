import { isFindingEnvelope } from "../../evals/lib/finding-envelope.mjs";
import { scoreFindings } from "../../evals/lib/quality.mjs";

/** Find ratio metrics that read none of a non-zero population without explanation. */
export function silentZeros(cases) {
  const violations = [];
  const unread = [];
  for (const corpus of cases) {
    for (const metric of corpus.metrics ?? []) {
      if (metric.shape !== "ratio" || metric.state !== "measured") continue;
      if (Number(metric.matched ?? 0) !== 0) continue;
      if (Number(metric.population ?? 0) === 0) continue;
      const named = (corpus.diagnostics ?? []).some(
        (diagnostic) =>
          diagnostic.value === metric.name ||
          String(diagnostic.message ?? "").includes(metric.name),
      );
      if (named) continue;
      const instance = {
        corpus: corpus.name,
        metric: metric.name,
        value: metric.value ?? null,
        population: Number(metric.population),
        examined: Number(metric.examined ?? 0),
      };
      if (instance.examined > 0) violations.push(instance);
      else unread.push(instance);
    }
  }
  return { violations, unread };
}

/** Flatten case-scoped labels into the scorer's input shape. */
export function flattenLabels(labels) {
  return labels.corpora.flatMap((corpus) =>
    corpus.defects.map((defect) => ({ ...defect, corpus: corpus.name })),
  );
}

/** Fraction of confirmed findings paired at the expected location. */
export function localisationRate(score) {
  const confirmed = score.families.reduce(
    (total, family) => total + family.truePositives,
    0,
  );
  return confirmed === 0
    ? null
    : Number((score.positional / confirmed).toFixed(3));
}

/** Score the same findings and labels partitioned by declared language. */
export function byLanguage(corpora, findings, labels, shapes, adjudication) {
  const languages = [
    ...new Set(corpora.map((corpus) => corpus.language)),
  ].sort();
  return languages.map((language) => {
    const names = new Set(
      corpora
        .filter((corpus) => corpus.language === language)
        .map((corpus) => corpus.name),
    );
    return {
      language,
      corpora: names.size,
      families: scoreFindings(
        findings.filter((finding) =>
          names.has(
            isFindingEnvelope(finding)
              ? finding.identity?.case
              : finding.corpus,
          ),
        ),
        labels.filter((label) => names.has(label.corpus)),
        shapes,
        adjudication,
      ).families,
    };
  });
}

/** Build the explicit positive/negative rulings used for advisory precision. */
export function adjudicationOf(
  corpora,
  mapping,
  standing = [],
  retained = null,
) {
  const out = {};
  const standingByFamily = new Map();
  for (const rule of standing) {
    if (rule?.verdict !== "correct" || !rule?.reason) continue;
    const family = familyForReason(mapping, rule.reason);
    if (!family) continue;
    const declarations = standingByFamily.get(family) ?? new Set();
    for (const declaration of rule.declarations ?? []) {
      declarations.add(declaration);
    }
    standingByFamily.set(family, declarations);
  }

  const add = (side, reason, corpus, extra = {}) => {
    const family = familyForReason(mapping, reason);
    if (!family) return;
    const scope = reason.includes("/")
      ? reason.slice(0, reason.indexOf("/"))
      : null;
    out[family] ??= { present: [], absent: [] };
    if (
      !out[family][side].some(
        (item) => item.corpus === corpus && item.scope === scope,
      )
    ) {
      out[family][side].push({ corpus, scope, ...extra });
    }
  };

  for (const corpus of corpora) {
    for (const reason of corpus.rules?.present ?? []) {
      add("present", reason, corpus.name);
    }
    for (const reason of corpus.rules?.absent ?? []) {
      add("absent", reason, corpus.name);
    }
    for (const [family, declarations] of standingByFamily) {
      for (const declaration of declarations) {
        add(
          "present",
          `${declaration}/${mapping.families[family].key}`,
          corpus.name,
          {
            standing: true,
          },
        );
      }
    }
  }
  if (retained) {
    out.__metricVersion = retained.metricVersion;
    for (const [family, findings] of Object.entries(retained.byFamily ?? {})) {
      out[family] ??= { present: [], absent: [] };
      out[family].findings = structuredClone(findings);
    }
  }
  return out;
}

function familyForReason(mapping, reason) {
  const key = reason.includes("/")
    ? reason.slice(reason.indexOf("/") + 1)
    : reason;
  return (
    Object.entries(mapping?.families ?? {}).find(
      ([, value]) => value.key === key,
    )?.[0] ?? null
  );
}
