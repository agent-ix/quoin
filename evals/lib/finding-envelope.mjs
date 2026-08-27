/**
 * Cross-producer finding contract (agent-ix/quoin#255).
 *
 * Adapters copy only fields a producer actually emitted.  A missing field is
 * therefore represented as unavailable; it is never reconstructed from a
 * message, a neighbouring coordinate, or benchmark knowledge.
 */
export const FINDING_ENVELOPE_VERSION = "finding-envelope-v2";

const SOURCE_CLASSES = new Set(["quire", "quoin", "external-observation"]);
const STATES = new Set(["available", "unavailable", "not_applicable"]);

export function available(value) {
  return { state: "available", value };
}

export function unavailable(reason) {
  return { state: "unavailable", reason };
}

export function notApplicable(reason) {
  return { state: "not_applicable", reason };
}

/** Normalize one Quire diagnostic, suspicion, metric, or validation record. */
export function normalizeQuireFinding(raw, context = {}) {
  return normalizeFinding(raw, { ...context, sourceClass: "quire" });
}

/** Normalize one Quoin validation or evidence-audit record. */
export function normalizeQuoinFinding(raw, context = {}) {
  return normalizeFinding(raw, { ...context, sourceClass: "quoin" });
}

/** Normalize a retained observation without attributing it to Quire/Quoin. */
export function normalizeExternalObservation(raw, context = {}) {
  return normalizeFinding(raw, {
    ...context,
    sourceClass: "external-observation",
  });
}

export function normalizeFinding(raw, context = {}) {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("finding envelope: raw finding must be an object");
  }
  const sourceClass = context.sourceClass ?? raw.sourceClass;
  if (!SOURCE_CLASSES.has(sourceClass)) {
    throw new Error(
      `finding envelope: unsupported source class ${JSON.stringify(sourceClass)}`,
    );
  }
  const producer = nonBlank(context.producer ?? raw.producer);
  if (!producer) throw new Error("finding envelope: producer is required");
  const kind = nonBlank(context.kind ?? raw.kind ?? raw.reason);
  if (!kind) throw new Error("finding envelope: finding kind is required");

  const locusValue = compact({
    path: nonBlank(raw.path ?? raw.document ?? raw.file),
    line: positiveInteger(raw.line),
    column: positiveInteger(raw.column),
    endLine: positiveInteger(raw.endLine ?? raw.end_line),
    endColumn: positiveInteger(raw.endColumn ?? raw.end_column),
    rowId: nonBlank(raw.rowId ?? raw.row_id),
  });
  const nextMoveValue = explicitNextMove(raw);

  return {
    schemaVersion: FINDING_ENVELOPE_VERSION,
    source: {
      class: sourceClass,
      producer,
      channel: nonBlank(context.channel ?? raw.channel) ?? "unspecified",
      ...(nonBlank(context.version ?? raw.producerVersion)
        ? { version: nonBlank(context.version ?? raw.producerVersion) }
        : {}),
    },
    kind,
    identity: compact({
      family: nonBlank(context.family ?? raw.family),
      case: nonBlank(context.corpus ?? raw.corpus),
      language: nonBlank(context.language ?? raw.language),
      declaration: nonBlank(context.declaration ?? raw.declaration),
    }),
    evaluation: compact({
      metric: nonBlank(context.metric ?? raw.metric),
      value: context.value ?? raw.value,
    }),
    subject: slot(raw.subject, "producer emitted no affected subject"),
    locus:
      Object.keys(locusValue).length > 0
        ? available(locusValue)
        : slot(raw.locus, "producer emitted no locus"),
    causalEvidence: slot(
      raw.causalEvidence ??
        raw.causal_evidence ??
        raw.evidence ??
        raw.message ??
        raw.summary,
      "producer emitted no causal evidence",
    ),
    changeTarget: slot(
      raw.changeTarget ?? raw.change_target,
      "producer emitted no change target",
    ),
    nextMove:
      nextMoveValue === undefined
        ? slot(
            raw.nextMove ?? raw.next_move,
            "producer emitted no remedy or diagnostic step",
          )
        : available(nextMoveValue),
    raw: raw.rawProducerOutput ?? raw,
  };
}

/** Fail closed on malformed envelopes before a grader consumes them. */
export function validateFindingEnvelope(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("finding envelope: envelope must be an object");
  }
  if (value.schemaVersion !== FINDING_ENVELOPE_VERSION) {
    throw new Error(
      `finding envelope: unsupported schemaVersion ${JSON.stringify(value.schemaVersion)}`,
    );
  }
  if (
    !SOURCE_CLASSES.has(value.source?.class) ||
    !nonBlank(value.source?.producer)
  ) {
    throw new Error("finding envelope: source class and producer are required");
  }
  if (!nonBlank(value.kind)) {
    throw new Error("finding envelope: finding kind is required");
  }
  for (const field of [
    "subject",
    "locus",
    "causalEvidence",
    "changeTarget",
    "nextMove",
  ]) {
    validateSlot(value[field], field);
  }
  if (!("raw" in value)) {
    throw new Error("finding envelope: raw producer output is required");
  }
  return value;
}

export function isFindingEnvelope(value) {
  return value?.schemaVersion === FINDING_ENVELOPE_VERSION;
}

function slot(value, missingReason) {
  if (value === undefined || value === null || value === "") {
    return unavailable(missingReason);
  }
  if (typeof value === "object" && !Array.isArray(value) && "state" in value) {
    validateSlot(value, "producer field");
    return structuredClone(value);
  }
  return available(value);
}

function validateSlot(value, field) {
  if (!value || typeof value !== "object" || !STATES.has(value.state)) {
    throw new Error(
      `finding envelope: ${field} has an invalid availability state`,
    );
  }
  if (value.state === "available" && value.value === undefined) {
    throw new Error(`finding envelope: ${field} is available without a value`);
  }
  if (value.state !== "available" && !nonBlank(value.reason)) {
    throw new Error(
      `finding envelope: ${field} ${value.state} requires a reason`,
    );
  }
}

function explicitNextMove(raw) {
  const remedy = nonBlank(raw.remedy);
  if (remedy) return { kind: "remedy", text: remedy };
  const diagnostic = nonBlank(
    raw.nextDiagnosticStep ?? raw.next_diagnostic_step,
  );
  if (diagnostic) return { kind: "diagnostic", text: diagnostic };
  return undefined;
}

function compact(value) {
  return Object.fromEntries(
    Object.entries(value).filter(([, item]) => item !== undefined),
  );
}

function nonBlank(value) {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function positiveInteger(value) {
  return Number.isInteger(value) && value > 0 ? value : undefined;
}
