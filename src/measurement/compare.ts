import type {
  ComparisonReason,
  MeasurementCollection,
  MeasurementComparison,
  MeasurementObservation,
} from "./types.js";

/** Comparability only. This layer deliberately emits no quality verdict. */
export function compareMeasurementCollections(
  before: MeasurementCollection,
  after: MeasurementCollection,
): MeasurementComparison[] {
  const left = index(before.observations);
  const right = index(after.observations);
  const keys = new Set([...left.keys(), ...right.keys()]);
  return [...keys].sort(compare).map((key) => {
    const a = left.get(key);
    const b = right.get(key);
    const sample = a ?? (b as MeasurementObservation);
    if (!a || !b || a.state === "not_computed" || b.state === "not_computed") {
      return {
        metric: sample.metric,
        dimensions: sample.dimensions ?? {},
        before: a?.value ?? null,
        after: b?.value ?? null,
        delta: null,
        status: "not_computed",
        reasons: [],
      };
    }
    const reasons: ComparisonReason[] = [];
    if (a.definitionVersion !== b.definitionVersion) {
      reasons.push({
        code: "definition_changed",
        blocking: true,
        message: `${a.metric}: definition moved ${a.definitionVersion} -> ${b.definitionVersion}`,
      });
    }
    if (before.configDigest !== after.configDigest) {
      reasons.push({
        code: "configuration_changed",
        blocking: true,
        message: `${a.metric}: producer configuration moved ${before.configDigest} -> ${after.configDigest}`,
      });
    }
    if (incomplete(a) || incomplete(b)) {
      reasons.push({
        code: "incomplete_population",
        blocking: true,
        message: `${a.metric}: ratio matched none of a non-empty examined population, or collection marked it incomplete`,
      });
    }
    if (populationKey(a) !== populationKey(b)) {
      reasons.push({
        code: "population_changed",
        blocking: false,
        message: `${a.metric}: population changed; delta is shown with this warning`,
      });
    }
    if (before.toolVersion !== after.toolVersion) {
      reasons.push({
        code: "tool_changed",
        blocking: false,
        message: `${a.metric}: tool moved ${before.toolVersion} -> ${after.toolVersion}`,
      });
    }
    const blocked = reasons.some((reason) => reason.blocking);
    return {
      metric: a.metric,
      dimensions: a.dimensions ?? {},
      before: a.value,
      after: b.value,
      delta: blocked ? null : (b.value as number) - (a.value as number),
      status: blocked ? "incomparable" : "comparable",
      reasons,
    };
  });
}

function incomplete(observation: MeasurementObservation): boolean {
  if (observation.population?.complete !== undefined) {
    return !observation.population.complete;
  }
  return (
    observation.shape === "ratio" &&
    (observation.population?.examined ?? 0) > 0 &&
    observation.population?.matched === 0
  );
}

function index(
  observations: MeasurementObservation[],
): Map<string, MeasurementObservation> {
  return new Map(
    observations.map((observation) => [keyOf(observation), observation]),
  );
}

function keyOf(observation: MeasurementObservation): string {
  return `${observation.metric}\0${JSON.stringify(
    Object.entries(observation.dimensions ?? {}).sort(([a], [b]) =>
      compare(a, b),
    ),
  )}`;
}

function populationKey(observation: MeasurementObservation): string {
  return JSON.stringify(observation.population ?? null);
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
