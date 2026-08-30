// SPDX-License-Identifier: AGPL-3.0-or-later
// Read-only semantic type-fit audit core for agent-ix/quoin#288.

import { createHash } from "node:crypto";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { join, posix } from "node:path";

import YAML from "yaml";

const AXES = [
  "vocabulary",
  "structure",
  "definitionOccurrence",
  "identity",
  "versioning",
  "provenance",
  "lifecycle",
  "relationships",
  "roundTrip",
  "generatedCode",
  "accumulation",
];

const MISSING_CONCEPTS = [
  "run",
  "result",
  "evidence",
  "report",
  "relationship",
  "identity",
  "version",
  "provenance",
  "lifecycle",
];

const OCCURRENCE_SIGNALS = [
  "run",
  "result",
  "evidence",
  "timestamp",
  "observedAt",
  "startedAt",
  "finishedAt",
  "statusHistory",
];

const FOLLOW_UP_BOUNDARIES = [
  "analysis",
  "compiler",
  "code-generation",
  "module-schema",
  "migration",
  "database",
  "api",
  "publication",
  "enforcement",
  "retirement",
];

const ARTIFACT_SCHEMA_VERSIONS = {
  "snapshot.json": "semantic-module-snapshot-v1",
  "inventory.json": "semantic-module-inventory-v1",
  "type-fit.json": "semantic-type-fit-matrix-v1",
  "conflicts.json": "semantic-conflict-ledger-v1",
  "missing-types.json": "semantic-missing-type-ledger-v1",
  "repository-impact.json": "semantic-repository-impact-v1",
  "report.md": "semantic-audit-report-v1",
  "review.md": "semantic-audit-specreview-v1",
};

function ordered(value) {
  if (Array.isArray(value)) return value.map(ordered);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, ordered(value[key])]),
    );
  }
  return value;
}

export function serializeCanonical(value) {
  return `${JSON.stringify(ordered(value), null, 2)}\n`;
}

function digestBytes(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

function digestValue(value) {
  return digestBytes(serializeCanonical(value));
}

function canonicalPath(path) {
  return posix
    .normalize(String(path).replaceAll("\\", "/"))
    .replace(/^\.\//, "");
}

function validSha(value) {
  return typeof value === "string" && /^[0-9a-f]{40}$/.test(value);
}

function fileMap(files = []) {
  return new Map(files.map((file) => [canonicalPath(file.path), file]));
}

function findCaseInsensitive(files, prefix, typeName) {
  const wanted = `${prefix}/${typeName}`.toLowerCase();
  return [...files.keys()].find((path) => {
    const stem = path.replace(/\.[^.]+$/, "").toLowerCase();
    return stem === wanted;
  });
}

function collectKeys(value, keys = new Set()) {
  if (Array.isArray(value)) {
    for (const item of value) collectKeys(item, keys);
  } else if (value && typeof value === "object") {
    for (const [key, child] of Object.entries(value)) {
      keys.add(key);
      collectKeys(child, keys);
    }
  }
  return keys;
}

export function parseMarkdownRecord(path, content, excluded = {}) {
  const normalizedPath = canonicalPath(path);
  if (Object.hasOwn(excluded, normalizedPath)) {
    return {
      path: normalizedPath,
      state: "excluded",
      reason: excluded[normalizedPath],
    };
  }
  if (typeof content !== "string") {
    return {
      path: normalizedPath,
      state: "io-error",
      reason: "content was not readable",
    };
  }
  if (!content.startsWith("---\n") && !content.startsWith("---\r\n")) {
    return {
      path: normalizedPath,
      state: "untyped",
      reason: "no YAML frontmatter",
    };
  }
  const closing = content.match(/\r?\n---\r?\n/);
  if (!closing || closing.index === undefined) {
    return {
      path: normalizedPath,
      state: "invalid",
      reason: "unterminated YAML frontmatter",
    };
  }
  const yamlStart = content.indexOf("\n") + 1;
  const yamlText = content.slice(yamlStart, closing.index);
  let frontmatter;
  try {
    frontmatter = YAML.parse(yamlText);
  } catch (error) {
    return {
      path: normalizedPath,
      state: "invalid",
      reason: `invalid YAML frontmatter: ${error instanceof Error ? error.message.split("\n")[0] : String(error)}`,
    };
  }
  if (
    !frontmatter ||
    typeof frontmatter !== "object" ||
    typeof frontmatter.type !== "string"
  ) {
    return {
      path: normalizedPath,
      state: "untyped",
      reason: "frontmatter has no string type",
    };
  }
  const keys = collectKeys(frontmatter);
  const signals = [
    ...OCCURRENCE_SIGNALS,
    "lifecycle",
    "relationships",
    "provenance",
    "version",
  ].filter((key) => keys.has(key));
  return {
    path: normalizedPath,
    state: "parsed",
    declaredType: frontmatter.type,
    documentId: typeof frontmatter.id === "string" ? frontmatter.id : null,
    signals,
    frontmatterDigest: digestValue(frontmatter),
  };
}

function surface(present, evidence, value = undefined) {
  return {
    present,
    evidence: present ? evidence : "not declared",
    ...(value === undefined ? {} : { value }),
  };
}

function schemaFor(declaration, moduleFiles) {
  if (
    declaration.structuralKind === "artifact" &&
    declaration.raw.frontmatter_schema_ref
  ) {
    const path = canonicalPath(declaration.raw.frontmatter_schema_ref);
    const file = moduleFiles.get(path);
    if (!file) return { path, parsed: null, missing: true };
    try {
      return {
        path,
        parsed: JSON.parse(file.content),
        digest: digestBytes(file.content),
        missing: false,
      };
    } catch (error) {
      return {
        path,
        parsed: null,
        missing: false,
        invalid: true,
        reason: String(error),
      };
    }
  }
  if (declaration.raw.data_schema !== undefined) {
    return {
      path: "manifest.yaml#data_schema",
      parsed: declaration.raw.data_schema,
      digest: digestValue(declaration.raw.data_schema),
      missing: false,
    };
  }
  return null;
}

function isPlaceholderSchema(schema) {
  const value = schema?.parsed;
  if (!value || typeof value !== "object") return false;
  const keys = Object.keys(value).filter(
    (key) => !["$id", "$schema", "title", "description"].includes(key),
  );
  return keys.length === 1 && keys[0] === "type" && value.type === "object";
}

function containsEncodedBlob(declaration, schema) {
  const rawText = serializeCanonical(declaration.raw).toLowerCase();
  if (/schema_json|json_blob|yaml_blob|payload_json|free.?form/.test(rawText))
    return true;
  const properties = schema?.parsed?.properties;
  if (!properties || typeof properties !== "object") return false;
  return Object.entries(properties).some(
    ([name, value]) =>
      /(payload|schema|json|yaml|markdown|content|body)/i.test(name) &&
      value?.type === "string",
  );
}

function axis(status, confidence, evidence, explanation = undefined) {
  return {
    status,
    confidence,
    evidence,
    ...(explanation ? { explanation } : {}),
  };
}

function findingBase(
  id,
  kind,
  evidence,
  rationale,
  nextBoundary,
  reconciliation,
  affected = {},
) {
  return {
    id,
    kind,
    severity: kind === "provenance-conflict" ? "high" : "medium",
    status: "open",
    confidence: "high",
    modules: affected.modules ?? [],
    qualifiedTypes: affected.qualifiedTypes ?? [],
    repositories: affected.repositories ?? [],
    evidence,
    rationale,
    nextBoundary,
    reconciliation,
  };
}

function reconciliationFor(
  architecture,
  plane = "definition",
  owner = "module-repository",
) {
  return {
    plane,
    authority:
      architecture?.authority ??
      "semantic architecture authority record unavailable",
    owner,
    decision:
      architecture?.decision ??
      "semantic architecture decision ledger unavailable",
  };
}

function moduleSnapshot(
  entry,
  module,
  parsedManifest,
  architecture,
  conflicts,
) {
  const inspected = {
    contentDigest: module?.installed?.contentDigest ?? null,
    canonicalContentDigest: module?.sourceContentDigest ?? null,
    manifestName: parsedManifest?.name ?? null,
    manifestVersion:
      parsedManifest?.version == null ? null : String(parsedManifest.version),
    sourcePath: canonicalPath(
      module?.installed?.sourcePath ?? `unresolved/${entry.name}`,
    ),
    sourceCommit: module?.installed?.sourceCommit ?? null,
    clean: module?.installed?.clean ?? null,
  };
  const row = {
    declarationIndex: module?.declarationIndex ?? -1,
    name: entry.name,
    version: entry.version == null ? null : String(entry.version),
    sourceKind: entry.source?.type ?? null,
    canonicalRepository: entry.source?.url ?? null,
    subdirectory: entry.source?.path ?? null,
    requestedRef: entry.source?.ref ?? null,
    resolvedSha: module?.resolvedSha ?? null,
    inspected,
  };
  const mismatches = [];
  if (validSha(row.requestedRef) && row.requestedRef !== row.resolvedSha)
    mismatches.push([
      "requestedRef",
      row.requestedRef,
      "resolvedSha",
      row.resolvedSha,
    ]);
  if (inspected.sourceCommit && inspected.sourceCommit !== row.resolvedSha)
    mismatches.push([
      "resolvedSha",
      row.resolvedSha,
      "sourceCommit",
      inspected.sourceCommit,
    ]);
  if (
    inspected.contentDigest &&
    inspected.canonicalContentDigest &&
    inspected.contentDigest !== inspected.canonicalContentDigest
  )
    mismatches.push([
      "installedContentDigest",
      inspected.contentDigest,
      "canonicalContentDigest",
      inspected.canonicalContentDigest,
    ]);
  if (inspected.manifestName && inspected.manifestName !== entry.name)
    mismatches.push([
      "declaredName",
      entry.name,
      "manifestName",
      inspected.manifestName,
    ]);
  if (
    inspected.manifestVersion &&
    inspected.manifestVersion !== String(entry.version)
  )
    mismatches.push([
      "declaredVersion",
      String(entry.version),
      "manifestVersion",
      inspected.manifestVersion,
    ]);
  for (const [leftName, left, rightName, right] of mismatches) {
    conflicts.push(
      findingBase(
        `PROV-${String(conflicts.length + 1).padStart(3, "0")}`,
        "provenance-conflict",
        [
          `snapshot.modules[${row.declarationIndex}].${leftName}`,
          `snapshot.modules[${row.declarationIndex}].${rightName}`,
        ],
        `${leftName} ${JSON.stringify(left)} disagrees with ${rightName} ${JSON.stringify(right)}`,
        "source-provenance-decision",
        reconciliationFor(architecture, "meta", "quoin"),
        {
          modules: [entry.name],
          repositories: [entry.source?.url].filter(Boolean),
        },
      ),
    );
  }
  if (!validSha(row.resolvedSha)) {
    conflicts.push(
      findingBase(
        `PROV-${String(conflicts.length + 1).padStart(3, "0")}`,
        "provenance-conflict",
        [`snapshot.modules[${row.declarationIndex}].resolvedSha`],
        "module ref did not resolve to a full lowercase commit SHA",
        "source-provenance-decision",
        reconciliationFor(architecture, "meta", "quoin"),
        {
          modules: [entry.name],
          repositories: [entry.source?.url].filter(Boolean),
        },
      ),
    );
  }
  if (!parsedManifest || typeof parsedManifest.name !== "string") {
    conflicts.push(
      findingBase(
        `PROV-${String(conflicts.length + 1).padStart(3, "0")}`,
        "provenance-conflict",
        [`module:${entry.name}/manifest.yaml`],
        "canonical module manifest is unavailable or has no string name",
        "module-source-repair",
        reconciliationFor(architecture, "meta", "module-repository"),
        {
          modules: [entry.name],
          repositories: [entry.source?.url].filter(Boolean),
        },
      ),
    );
  }
  if (!inspected.contentDigest) {
    conflicts.push(
      findingBase(
        `PROV-${String(conflicts.length + 1).padStart(3, "0")}`,
        "provenance-conflict",
        [`snapshot.modules[${row.declarationIndex}].inspected.contentDigest`],
        "installed module content is unavailable",
        "module-installation-repair",
        reconciliationFor(architecture, "meta", "quoin"),
        {
          modules: [entry.name],
          repositories: [entry.source?.url].filter(Boolean),
        },
      ),
    );
  }
  for (const stage of module?.acquisitionErrors ?? []) {
    conflicts.push(
      findingBase(
        `PROV-${String(conflicts.length + 1).padStart(3, "0")}`,
        "provenance-conflict",
        [`snapshot.modules[${row.declarationIndex}].acquisition:${stage}`],
        `module acquisition did not complete at stage ${stage}`,
        "module-source-repair",
        reconciliationFor(architecture, "meta", "module-repository"),
        {
          modules: [entry.name],
          repositories: [entry.source?.url].filter(Boolean),
        },
      ),
    );
  }
  return row;
}

function makeDeclarations(moduleName, parsedManifest, moduleFiles) {
  const rows = [];
  for (const [field, structuralKind] of [
    ["artifact_types", "artifact"],
    ["object_types", "object"],
  ]) {
    const values = Array.isArray(parsedManifest?.[field])
      ? parsedManifest[field]
      : [];
    values.forEach((raw, index) => {
      if (!raw || typeof raw.name !== "string") return;
      const declaration = {
        declarationId: `${moduleName}::${structuralKind}::${raw.name}::${index}`,
        module: moduleName,
        name: raw.name,
        qualifiedName: `${moduleName}::${raw.name}`,
        structuralKind,
        declarationLocation: `manifest.yaml#${field}[${index}]`,
        raw,
      };
      const schema = schemaFor(declaration, moduleFiles);
      const skeletonPath = findCaseInsensitive(
        moduleFiles,
        "skeletons",
        raw.name,
      );
      const relationships = raw.allowed_links;
      const mappings =
        raw.mappings ?? raw.export_mappings ?? raw.serialization_mappings;
      const projections = raw.projections ?? raw.body_extraction;
      rows.push({
        ...declaration,
        schema,
        surfaces: {
          schema: surface(
            Boolean(schema && !schema.missing),
            schema?.path ?? "not declared",
          ),
          skeleton: surface(
            Boolean(skeletonPath),
            skeletonPath ?? "not declared",
          ),
          relationships: surface(
            relationships !== undefined,
            `${declaration.declarationLocation}.allowed_links`,
          ),
          mappings: surface(
            mappings !== undefined,
            `${declaration.declarationLocation}.mappings`,
          ),
          projections: surface(
            projections !== undefined,
            `${declaration.declarationLocation}.${raw.body_extraction ? "body_extraction" : "projections"}`,
          ),
        },
        instances: [],
        observations: [],
      });
    });
  }
  return rows;
}

function assess(declaration, documents, snapshot) {
  const evidence = [`inventory.declarations:${declaration.declarationId}`];
  const instances = documents.filter(
    (doc) =>
      doc.state === "parsed" &&
      doc.module === declaration.module &&
      doc.declaredType.toLowerCase() === declaration.name.toLowerCase(),
  );
  declaration.instances = instances.map((doc) => doc.path);
  if (instances.length === 0) declaration.observations.push("no-instance");
  const placeholder = isPlaceholderSchema(declaration.schema);
  const blob = containsEncodedBlob(declaration, declaration.schema);
  const occurrenceSignals = [
    ...new Set(
      instances
        .flatMap((doc) => doc.signals)
        .filter((signal) => OCCURRENCE_SIGNALS.includes(signal)),
    ),
  ];
  const flags = [];
  if (placeholder) flags.push("placeholder-schema");
  if (blob) flags.push("encoded-structure-blob");
  if (occurrenceSignals.length) flags.push("occurrence-signals");
  if (instances.length === 0) flags.push("no-instance");
  const identityStatus =
    instances.length === 0
      ? "missing"
      : instances.every((doc) => doc.documentId)
        ? "supported"
        : "partial";
  const hasLifecycle = /state|lifecycle|transition/i.test(
    serializeCanonical(declaration.raw),
  );
  const schemaUsable =
    declaration.schema &&
    !declaration.schema.missing &&
    !declaration.schema.invalid &&
    !placeholder;
  const axes = {
    vocabulary: axis("supported", "high", evidence),
    structure: axis(
      blob || placeholder
        ? "partial"
        : declaration.schema
          ? "supported"
          : "missing",
      "high",
      evidence,
    ),
    definitionOccurrence: axis(
      occurrenceSignals.length ? "conflict" : "supported",
      occurrenceSignals.length ? "high" : "medium",
      evidence,
    ),
    identity: axis(
      identityStatus,
      instances.length ? "high" : "medium",
      instances.length
        ? instances.map(
            (doc) => `inventory.documents:${doc.module}/${doc.path}`,
          )
        : evidence,
    ),
    versioning: axis(
      snapshot.version && snapshot.resolvedSha
        ? "supported"
        : snapshot.version
          ? "partial"
          : "missing",
      "high",
      [`snapshot.modules:${snapshot.name}`],
    ),
    provenance: axis(
      snapshot.inspected.sourceCommit ? "supported" : "missing",
      "high",
      [`snapshot.modules:${snapshot.name}`],
    ),
    lifecycle: hasLifecycle
      ? axis("supported", "medium", evidence)
      : axis("not-applicable", "low", [], "no lifecycle behavior is declared"),
    relationships: axis(
      declaration.surfaces.relationships.present ? "supported" : "missing",
      "high",
      evidence,
    ),
    roundTrip: axis(
      schemaUsable && declaration.surfaces.skeleton.present && !blob
        ? "supported"
        : declaration.schema
          ? "partial"
          : "missing",
      "medium",
      evidence,
    ),
    generatedCode: axis(
      schemaUsable && !blob
        ? "supported"
        : declaration.schema
          ? "partial"
          : "missing",
      "high",
      evidence,
    ),
    accumulation: axis(
      occurrenceSignals.length ? "conflict" : "supported",
      occurrenceSignals.length ? "high" : "medium",
      evidence,
    ),
  };
  const statuses = Object.values(axes).map((item) => item.status);
  let disposition = "fits";
  if (statuses.includes("conflict")) disposition = "conflict";
  else if (statuses.includes("missing") || statuses.includes("partial"))
    disposition = "incomplete";
  else if (declaration.surfaces.mappings.present)
    disposition = "fits-with-mapping";
  else if (!declaration.schema && declaration.surfaces.projections.present)
    disposition = "representation-local";
  return {
    qualifiedName: declaration.qualifiedName,
    declarationId: declaration.declarationId,
    module: declaration.module,
    name: declaration.name,
    structuralKind: declaration.structuralKind,
    disposition,
    reason: `derived from ${statuses.join(", ")}`,
    flags,
    axes,
  };
}

function duplicateConflicts(declarations, architecture, startingIndex) {
  const groups = new Map();
  for (const row of declarations) {
    const key = row.name.toLowerCase();
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(row);
  }
  const conflicts = [];
  for (const rows of groups.values()) {
    if (rows.length < 2) continue;
    const shapes = new Set(
      rows.map((row) =>
        digestValue({
          structuralKind: row.structuralKind,
          schema: row.schema?.parsed ?? null,
          raw: row.raw,
        }),
      ),
    );
    if (shapes.size < 2) continue;
    conflicts.push(
      findingBase(
        `CONFLICT-${String(startingIndex + conflicts.length + 1).padStart(3, "0")}`,
        "duplicate-type",
        rows.map((row) => `inventory.declarations:${row.declarationId}`),
        `module-qualified declarations named ${rows[0].name} have incompatible shapes`,
        "module-schema-decision",
        reconciliationFor(architecture),
        {
          modules: [...new Set(rows.map((row) => row.module))],
          qualifiedTypes: rows.map((row) => row.declarationId),
          repositories: [...new Set(rows.map((row) => row.module))],
        },
      ),
    );
  }
  return conflicts;
}

function missingTypeLedger(
  declarations,
  documents,
  snapshotModules,
  architecture,
) {
  const names = new Set(
    declarations.map((row) => row.name.toLowerCase().replaceAll(/[-_ ]/g, "")),
  );
  const declarationText = serializeCanonical(
    declarations.map((row) => row.raw),
  ).toLowerCase();
  return MISSING_CONCEPTS.map((concept, index) => {
    const normalized = concept.replaceAll(/[-_ ]/g, "");
    const directlyNamed = [...names].some(
      (name) => name === normalized || name.endsWith(normalized),
    );
    const structuralEvidence = [];
    if (
      concept === "relationship" &&
      declarations.some((row) => row.surfaces.relationships.present)
    )
      structuralEvidence.push("inventory.declarations:allowed_links");
    if (
      concept === "identity" &&
      documents.some((row) => row.state === "parsed" && row.documentId)
    )
      structuralEvidence.push("inventory.documents:documentId");
    if (
      concept === "version" &&
      snapshotModules.some((row) => row.version && row.resolvedSha)
    )
      structuralEvidence.push("snapshot.modules:version+resolvedSha");
    if (
      concept === "lifecycle" &&
      /state_machine|lifecycle|transitions?_to/.test(declarationText)
    )
      structuralEvidence.push("inventory.declarations:lifecycle-signals");
    const overloaded =
      !directlyNamed &&
      structuralEvidence.length === 0 &&
      new RegExp(`(^|[^a-z])${concept}([^a-z]|$)`, "i").test(declarationText);
    const conceptDisposition =
      directlyNamed || structuralEvidence.length
        ? "represented"
        : overloaded
          ? "overloaded"
          : "absent";
    const evidence = directlyNamed
      ? declarations
          .filter(
            (row) =>
              normalizeConcept(row.name) === normalized ||
              normalizeConcept(row.name).endsWith(normalized),
          )
          .map((row) => `inventory.declarations:${row.declarationId}`)
      : structuralEvidence.length
        ? structuralEvidence
        : overloaded
          ? ["inventory.declarations:embedded-concept-signal"]
          : ["inventory.declarations:complete-name-and-field-census"];
    const rationale = {
      represented: `${concept} is represented in the current module vocabulary or structural contract and still requires semantic-boundary review`,
      overloaded: `${concept} appears only inside another declaration rather than as a dedicated semantic type`,
      absent: `${concept} has no dedicated or structural representation in the default module set`,
    }[conceptDisposition];
    return {
      ...findingBase(
        `MISSING-${String(index + 1).padStart(3, "0")}`,
        "missing-type",
        evidence,
        rationale,
        "core-data-or-module-vocabulary-decision",
        reconciliationFor(
          architecture,
          concept === "run" || concept === "result" || concept === "evidence"
            ? "execution-observation"
            : "definition",
          "filament-core-data/module-repository",
        ),
        { repositories: ["filament-core-data", "quoin/default-modules"] },
      ),
      concept,
      conceptDisposition,
      status:
        conceptDisposition === "absent"
          ? "open"
          : `${conceptDisposition}-review-required`,
      confidence: conceptDisposition === "overloaded" ? "medium" : "high",
    };
  });
}

function repositoryImpact(boundaries, moduleNames) {
  return boundaries.map((repository, index) => {
    const module = moduleNames.includes(repository);
    const impact = module || repository === "quoin" ? "required" : "candidate";
    return {
      id: `IMPACT-${String(index + 1).padStart(3, "0")}`,
      repository,
      impact,
      effort: module ? "medium" : "unknown",
      risk: ["database", "api", "generated-packages", "compiler"].includes(
        repository,
      )
        ? "high"
        : "medium",
      dependency:
        repository === "quoin" ? "audit-owner" : "semantic-audit-findings",
      wave: module ? "module-contract-review" : "gated-follow-up",
      confidence: module ? "high" : "low",
      rationale: module
        ? "the repository owns an audited declaration"
        : "impact depends on accepted findings and a separately gated design",
    };
  });
}

function normalizeConcept(value) {
  return String(value)
    .toLowerCase()
    .replaceAll(/[^a-z0-9]/g, "");
}

function coreDataReconciliation(declarations, census, revision) {
  const records = census?.records ?? [];
  return declarations.map((declaration) => {
    const needle = normalizeConcept(declaration.name);
    const matches = records.filter((record) => {
      const declaredTypes = Array.isArray(record.declaredTypes)
        ? record.declaredTypes
        : [];
      const explicit = declaredTypes.some((name) => {
        const normalized = normalizeConcept(name)
          .replace(/^core/, "")
          .replace(/(record|entry|kind|ref|result)$/, "");
        return (
          needle.length >= 4 &&
          (normalized.includes(needle) || needle.includes(normalized))
        );
      });
      const generic =
        declaration.structuralKind === "artifact"
          ? declaredTypes.some((name) =>
              [
                "coreartifactrecord",
                "coreartifactentry",
                "coreartifactkind",
              ].includes(normalizeConcept(name)),
            )
          : declaredTypes.some(
              (name) => normalizeConcept(name) === "coreobjecttyperecord",
            );
      return explicit || generic;
    });
    return {
      declarationId: declaration.declarationId,
      qualifiedName: declaration.qualifiedName,
      disposition: matches.length ? "mapping" : "unrelated",
      censusRevision: revision,
      censusRecords: matches.map((record) => record.id),
      evidence: matches.map(
        (record) =>
          `${record.id}:${record.source?.path ?? record.producer ?? "census"}`,
      ),
      rationale: matches.length
        ? "the declaration overlaps a current core artifact/object or named contract and requires an explicit mapping rather than a shadow definition"
        : "no current core-data census record names or generically owns this declaration",
    };
  });
}

function denominators(entries, declarations, documents, sourceCounts) {
  const documentCount = sourceCounts.documents;
  const parseStateCount = documents.filter((row) =>
    ["parsed", "invalid", "untyped", "excluded", "io-error"].includes(
      row.state,
    ),
  ).length;
  const declarationSource = sourceCounts.declarations;
  return {
    modules: {
      source: entries.length,
      inventoried: entries.length,
      reconciled: true,
    },
    declarations: {
      source: declarationSource,
      inventoried: declarations.length,
      reconciled: declarationSource === declarations.length,
    },
    contractSurfaces: {
      source: declarationSource * 5,
      inventoried: declarations.reduce(
        (sum, row) => sum + Object.keys(row.surfaces).length,
        0,
      ),
      reconciled: declarations.every(
        (row) => Object.keys(row.surfaces).length === 5,
      ),
    },
    documents: {
      source: documentCount,
      inventoried: documents.length,
      reconciled: documentCount === documents.length,
    },
    parseStates: {
      source: documentCount,
      inventoried: parseStateCount,
      reconciled: documentCount === parseStateCount,
    },
  };
}

function deriveSummary(
  snapshotModules,
  inventory,
  assessments,
  conflicts,
  missingTypes,
  impact,
) {
  const reconciled = Object.values(inventory.denominators).every(
    (row) => row.reconciled,
  );
  const typeFitComplete = assessments.every((row) =>
    ["fits", "fits-with-mapping", "representation-local"].includes(
      row.disposition,
    ),
  );
  const conceptsComplete = missingTypes.every(
    (row) => row.conceptDisposition === "represented",
  );
  return {
    schemaVersion: "semantic-module-type-fit-summary-v1",
    verdict:
      conflicts.length === 0 &&
      reconciled &&
      typeFitComplete &&
      conceptsComplete
        ? "clean"
        : "findings",
    counts: {
      modules: snapshotModules.length,
      declarations: inventory.declarations.length,
      documents: inventory.documents.length,
      conflicts: conflicts.length,
      missingTypes: missingTypes.length,
      repositoryImpacts: impact.length,
    },
  };
}

export function buildSemanticAudit(input) {
  const defaultManifest = YAML.parse(input.defaultModulesText);
  const entries = Array.isArray(defaultManifest?.entries)
    ? defaultManifest.entries
    : [];
  const moduleInputs = new Map(
    (input.modules ?? []).map((module) => [module.declarationIndex, module]),
  );
  const conflicts = [];
  const snapshotModules = [];
  const declarations = [];
  const documents = [];
  const sourceCounts = { declarations: 0, documents: 0 };
  for (let index = 0; index < entries.length; index += 1) {
    const entry = entries[index];
    const module = moduleInputs.get(index);
    let parsedManifest = null;
    try {
      parsedManifest = YAML.parse(module?.manifestText ?? "");
    } catch (error) {
      conflicts.push(
        findingBase(
          `PROV-${String(conflicts.length + 1).padStart(3, "0")}`,
          "provenance-conflict",
          [`module:${entry.name}/manifest.yaml`],
          `module manifest is unreadable: ${String(error)}`,
          "module-source-repair",
          reconciliationFor(input.architecture, "meta", "module-repository"),
          {
            modules: [entry.name],
            repositories: [entry.source?.url].filter(Boolean),
          },
        ),
      );
    }
    sourceCounts.declarations +=
      (Array.isArray(parsedManifest?.artifact_types)
        ? parsedManifest.artifact_types.length
        : 0) +
      (Array.isArray(parsedManifest?.object_types)
        ? parsedManifest.object_types.length
        : 0);
    const snapshot = moduleSnapshot(
      entry,
      module,
      parsedManifest,
      input.architecture,
      conflicts,
    );
    snapshotModules.push(snapshot);
    const files = fileMap(module?.moduleFiles);
    const moduleDeclarations = makeDeclarations(
      entry.name,
      parsedManifest,
      files,
    );
    declarations.push(...moduleDeclarations);
    const corpusFiles = module?.corpusFiles ?? [];
    sourceCounts.documents += corpusFiles.filter((file) =>
      canonicalPath(file.path).toLowerCase().endsWith(".md"),
    ).length;
    for (const file of corpusFiles) {
      if (!canonicalPath(file.path).toLowerCase().endsWith(".md")) continue;
      documents.push({
        module: entry.name,
        ...parseMarkdownRecord(
          file.path,
          file.content,
          module.excludedMarkdown ?? {},
        ),
      });
    }
  }
  const snapshot = {
    schemaVersion: "semantic-module-snapshot-v1",
    timestamp: input.timestamp,
    quoin: input.quoin,
    defaultModulesDigest: digestBytes(input.defaultModulesText),
    tools: input.tools,
    externalEvidence: input.externalEvidence,
    modules: snapshotModules,
  };
  const assessments = declarations.map((declaration) =>
    assess(
      declaration,
      documents,
      snapshotModules.find((row) => row.name === declaration.module),
    ),
  );
  conflicts.push(
    ...duplicateConflicts(declarations, input.architecture, conflicts.length),
  );
  const missingTypes = missingTypeLedger(
    declarations,
    documents,
    snapshotModules,
    input.architecture,
  );
  const impact = repositoryImpact(
    input.repositoryBoundaries ?? [],
    snapshotModules.map((row) => row.name),
  );
  const inventory = {
    schemaVersion: "semantic-module-inventory-v1",
    modules: snapshotModules.map((row) => row.name),
    declarations: declarations.map(({ raw, schema, ...row }) => ({
      ...row,
      schema: schema
        ? {
            path: schema.path,
            digest: schema.digest ?? null,
            missing: Boolean(schema.missing),
            invalid: Boolean(schema.invalid),
          }
        : null,
    })),
    documents,
    denominators: denominators(entries, declarations, documents, sourceCounts),
  };
  const typeFit = {
    schemaVersion: "semantic-type-fit-matrix-v1",
    axes: AXES,
    assessments,
  };
  const reconciliation = {
    schemaVersion: "semantic-audit-reconciliation-v1",
    architectureRevision: input.architecture?.revision ?? null,
    coreData: coreDataReconciliation(
      declarations,
      input.coreDataCensus,
      input.externalEvidence?.coreDataCensusRevision ?? null,
    ),
    shadowContracts: [],
    quire: {
      revision: input.externalEvidence?.quireCorpusRevision ?? null,
      preservesBoundary: true,
      responsibilities: [
        "parse",
        "validate",
        "extract",
        "address",
        "byte-splice",
      ],
      evidence: [
        `agent-ix/quire-rs#385`,
        `agent-ix/quire-corpus@${input.externalEvidence?.quireCorpusRevision ?? "unresolved"}`,
      ],
    },
    followUpBoundaries: FOLLOW_UP_BOUNDARIES.map((name) => ({
      name,
      majorInterference: name !== "analysis",
      gate: name === "analysis" ? null : `${name}-design-and-maintainer-gate`,
    })),
  };
  const summary = deriveSummary(
    snapshotModules,
    inventory,
    assessments,
    conflicts,
    missingTypes,
    impact,
  );
  return {
    schemaVersion: "semantic-module-type-fit-v1",
    snapshot,
    inventory,
    typeFit,
    conflicts,
    missingTypes,
    repositoryImpact: impact,
    reconciliation,
    summary,
  };
}

function reportOf(audit) {
  const findingRows = [...audit.conflicts, ...audit.missingTypes]
    .map(
      (row) =>
        `| ${row.id} | ${row.kind} | ${row.severity} | ${row.status} | ${row.rationale.replaceAll("|", "\\|")} |`,
    )
    .join("\n");
  const dispositionCounts = new Map();
  for (const row of audit.typeFit.assessments)
    dispositionCounts.set(
      row.disposition,
      (dispositionCounts.get(row.disposition) ?? 0) + 1,
    );
  const dispositionRows = [...dispositionCounts.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([name, count]) => `| ${name} | ${count} |`)
    .join("\n");
  const moduleRows = audit.snapshot.modules
    .map((module) => {
      const declarations = audit.inventory.declarations.filter(
        (row) => row.module === module.name,
      );
      const documents = audit.inventory.documents.filter(
        (row) => row.module === module.name,
      );
      const instances = declarations.reduce(
        (sum, row) => sum + row.instances.length,
        0,
      );
      const moduleConflicts = audit.conflicts.filter((row) =>
        row.modules.includes(module.name),
      ).length;
      return `| ${module.name} | ${declarations.length} | ${documents.length} | ${instances} | ${moduleConflicts} |`;
    })
    .join("\n");
  const impactRows = audit.repositoryImpact
    .map(
      (row) =>
        `| ${row.repository} | ${row.impact} | ${row.effort} | ${row.risk} | ${row.wave} | ${row.confidence} |`,
    )
    .join("\n");
  return (
    `# Default-module semantic type-fit audit\n\n` +
    `Canonical schema: \`${audit.schemaVersion}\`  \n` +
    `Verdict: **${audit.summary.verdict}**\n\n` +
    `## Denominators\n\n| Population | Source | Inventoried | Reconciled |\n| --- | ---: | ---: | --- |\n` +
    Object.entries(audit.inventory.denominators)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(
        ([name, row]) =>
          `| ${name} | ${row.source} | ${row.inventoried} | ${row.reconciled ? "yes" : "no"} |`,
      )
      .join("\n") +
    `\n\n## Module census\n\n| Module | Declarations | Markdown paths | Matched instances | Conflicts |\n| --- | ---: | ---: | ---: | ---: |\n${moduleRows}\n` +
    `\n\n## Type dispositions\n\n| Disposition | Declarations |\n| --- | ---: |\n${dispositionRows}\n` +
    `\n\n## Findings\n\n| ID | Kind | Severity | Status | Rationale |\n| --- | --- | --- | --- | --- |\n${findingRows}\n` +
    `\n## Repository impact\n\n| Repository/boundary | Impact | Effort | Risk | Wave | Confidence |\n| --- | --- | --- | --- | --- | --- |\n${impactRows}\n` +
    `\n## Interpretation\n\nThe census is complete for the recorded pins; a \`findings\` verdict means the data cannot be promoted as a conflict-free semantic contract. Every compiler, schema, migration, publication, enforcement, retirement, database, API, CLI, UI, and generated-package recommendation remains a separately gated follow-up.\n`
  );
}

function reviewOf(audit) {
  const rows = [...audit.conflicts, ...audit.missingTypes]
    .map(
      (row, index) =>
        `| FND-${String(46 + index).padStart(3, "0")} | ${row.severity} | ${row.id}: ${row.rationale.replaceAll("|", "\\|")} | ${row.evidence.join(", ")} |`,
    )
    .join("\n");
  const denominators = Object.entries(audit.inventory.denominators)
    .sort(([left], [right]) => left.localeCompare(right))
    .map(
      ([name, row]) =>
        `| ${name} | ${row.source} | ${row.inventoried} | ${row.reconciled ? "yes" : "no"} |`,
    )
    .join("\n");
  return `---\nid: SR-055\ntitle: "Default-module semantic type-fit review"\ntype: SpecReview\nanalysis: architecture-evaluation\nscope: "default-modules.yaml complete pinned corpus"\n---\n\n# Default-module semantic type-fit review\n\n## Summary\n\nGenerated from \`${audit.schemaVersion}\`; verdict **${audit.summary.verdict}**. Canonical findings remain in JSON and this document is their human review projection.\n\n## Denominators\n\n| Population | Source | Inventoried | Reconciled |\n| --- | ---: | ---: | --- |\n${denominators}\n\n## Findings\n\n| ID | Severity | Summary | Refs |\n| --- | --- | --- | --- |\n${rows}\n`;
}

function contentIdentity(audit) {
  const snapshot = structuredClone(audit.snapshot);
  delete snapshot.timestamp;
  return digestValue({
    snapshot,
    inventory: audit.inventory,
    typeFit: audit.typeFit,
    conflicts: audit.conflicts,
    missingTypes: audit.missingTypes,
    repositoryImpact: audit.repositoryImpact,
    reconciliation: audit.reconciliation,
  });
}

function contentIdentityFromArtifacts(parts) {
  const snapshot = structuredClone(parts.snapshot);
  delete snapshot.timestamp;
  return digestValue({
    snapshot,
    inventory: parts.inventory,
    typeFit: parts.typeFit,
    conflicts: parts.conflicts,
    missingTypes: parts.missingTypes,
    repositoryImpact: parts.repositoryImpact,
    reconciliation: parts.reconciliation,
  });
}

export function createArtifactFiles(audit) {
  const files = new Map([
    ["snapshot.json", serializeCanonical(audit.snapshot)],
    ["inventory.json", serializeCanonical(audit.inventory)],
    ["type-fit.json", serializeCanonical(audit.typeFit)],
    [
      "conflicts.json",
      serializeCanonical({
        schemaVersion: "semantic-conflict-ledger-v1",
        conflicts: audit.conflicts,
      }),
    ],
    [
      "missing-types.json",
      serializeCanonical({
        schemaVersion: "semantic-missing-type-ledger-v1",
        missingTypes: audit.missingTypes,
      }),
    ],
    [
      "repository-impact.json",
      serializeCanonical({
        schemaVersion: "semantic-repository-impact-v1",
        impacts: audit.repositoryImpact,
        reconciliation: audit.reconciliation,
      }),
    ],
    ["report.md", reportOf(audit)],
    ["review.md", reviewOf(audit)],
  ]);
  const manifest = {
    schemaVersion: "semantic-audit-artifact-manifest-v1",
    auditSchemaVersion: audit.schemaVersion,
    runTimestamp: audit.snapshot.timestamp,
    contentIdentity: contentIdentity(audit),
    counts: audit.summary.counts,
    artifacts: [...files.entries()].map(([path, content]) => ({
      path,
      schemaVersion: ARTIFACT_SCHEMA_VERSIONS[path],
      digest: digestBytes(content),
      bytes: Buffer.byteLength(content),
    })),
  };
  files.set("manifest.json", serializeCanonical(manifest));
  return files;
}

export function verifyArtifactFiles(files) {
  const required = [
    "snapshot.json",
    "inventory.json",
    "type-fit.json",
    "conflicts.json",
    "missing-types.json",
    "repository-impact.json",
    "report.md",
    "review.md",
    "manifest.json",
  ];
  for (const path of required)
    if (!files.has(path)) throw new Error(`missing canonical artifact ${path}`);
  if (files.size !== required.length)
    throw new Error("artifact set contains an unreferenced file");
  const manifest = JSON.parse(files.get("manifest.json"));
  if (manifest.schemaVersion !== "semantic-audit-artifact-manifest-v1")
    throw new Error("unsupported artifact manifest schema");
  if (manifest.auditSchemaVersion !== "semantic-module-type-fit-v1")
    throw new Error("unsupported semantic audit schema");
  if (manifest.artifacts.length !== required.length - 1)
    throw new Error("artifact manifest count disagrees with required set");
  const artifactPaths = manifest.artifacts.map((record) => record.path);
  if (
    new Set(artifactPaths).size !== artifactPaths.length ||
    serializeCanonical([...artifactPaths].sort()) !==
      serializeCanonical(required.slice(0, -1).sort())
  )
    throw new Error("artifact manifest paths disagree with required set");
  for (const record of manifest.artifacts) {
    const content = files.get(record.path);
    if (content === undefined)
      throw new Error(`manifest references missing artifact ${record.path}`);
    if (digestBytes(content) !== record.digest)
      throw new Error(`artifact digest disagrees for ${record.path}`);
    if (Buffer.byteLength(content) !== record.bytes)
      throw new Error(`artifact byte count disagrees for ${record.path}`);
    if (record.schemaVersion !== ARTIFACT_SCHEMA_VERSIONS[record.path])
      throw new Error(`artifact schema version disagrees for ${record.path}`);
  }
  const snapshot = JSON.parse(files.get("snapshot.json"));
  const inventory = JSON.parse(files.get("inventory.json"));
  const typeFit = JSON.parse(files.get("type-fit.json"));
  const conflicts = JSON.parse(files.get("conflicts.json"));
  const missing = JSON.parse(files.get("missing-types.json"));
  const impact = JSON.parse(files.get("repository-impact.json"));
  const actualCounts = {
    modules: inventory.modules.length,
    declarations: inventory.declarations.length,
    documents: inventory.documents.length,
    conflicts: conflicts.conflicts.length,
    missingTypes: missing.missingTypes.length,
    repositoryImpacts: impact.impacts.length,
  };
  if (serializeCanonical(actualCounts) !== serializeCanonical(manifest.counts))
    throw new Error("artifact manifest count disagrees with canonical data");
  const recomputedIdentity = contentIdentityFromArtifacts({
    snapshot,
    inventory,
    typeFit,
    conflicts: conflicts.conflicts,
    missingTypes: missing.missingTypes,
    repositoryImpact: impact.impacts,
    reconciliation: impact.reconciliation,
  });
  if (recomputedIdentity !== manifest.contentIdentity)
    throw new Error(
      "artifact manifest content identity disagrees with canonical data",
    );
  if (manifest.runTimestamp !== snapshot.timestamp)
    throw new Error("artifact manifest run timestamp disagrees with snapshot");
  const summary = deriveSummary(
    snapshot.modules,
    inventory,
    typeFit.assessments,
    conflicts.conflicts,
    missing.missingTypes,
    impact.impacts,
  );
  const reconstructed = {
    schemaVersion: manifest.auditSchemaVersion,
    snapshot,
    inventory,
    typeFit,
    conflicts: conflicts.conflicts,
    missingTypes: missing.missingTypes,
    repositoryImpact: impact.impacts,
    reconciliation: impact.reconciliation,
    summary,
  };
  const expectedReport = reportOf(reconstructed);
  if (files.get("report.md") !== expectedReport) {
    const actualReport = files.get("report.md");
    const offset = [...actualReport].findIndex(
      (character, index) => character !== expectedReport[index],
    );
    throw new Error(
      `report.md disagrees with canonical audit data at byte ${offset}`,
    );
  }
  const expectedReview = reviewOf(reconstructed);
  if (files.get("review.md") !== expectedReview) {
    const actualReview = files.get("review.md");
    const offset = [...actualReview].findIndex(
      (character, index) => character !== expectedReview[index],
    );
    throw new Error(
      `review.md disagrees with canonical audit data at byte ${offset}`,
    );
  }
  return {
    valid: true,
    contentIdentity: manifest.contentIdentity,
    counts: actualCounts,
  };
}

export function writeArtifactFiles(outputRoot, files) {
  verifyArtifactFiles(files);
  if (existsSync(outputRoot)) {
    if (!lstatSync(outputRoot).isDirectory())
      throw new Error("audit output root is not a directory");
    const unexpected = readdirSync(outputRoot).filter(
      (path) => !files.has(path),
    );
    if (unexpected.length)
      throw new Error(
        `output directory contains unreferenced artifacts: ${unexpected.join(", ")}`,
      );
  }
  mkdirSync(outputRoot, { recursive: true });
  for (const [path, content] of files) {
    if (canonicalPath(path) !== path || path.includes("/"))
      throw new Error(`unsafe artifact path ${path}`);
    const target = join(outputRoot, path);
    if (existsSync(target) && !lstatSync(target).isFile())
      throw new Error(`audit artifact target is not a regular file: ${path}`);
    writeFileSync(target, content, {
      encoding: "utf8",
      mode: 0o644,
    });
  }
}

export function freshCensus(recordedSnapshot, currentSnapshot) {
  const recorded = structuredClone(recordedSnapshot);
  const current = structuredClone(currentSnapshot);
  delete recorded.timestamp;
  delete current.timestamp;
  const differences = [];
  if (serializeCanonical(recorded) !== serializeCanonical(current))
    differences.push("snapshot content changed");
  return {
    fresh: differences.length === 0,
    signoffBlocked: differences.length > 0,
    differences,
  };
}

export function isAllowedAuditPath(path) {
  const normalized = canonicalPath(path);
  return (
    normalized === "tests/semantic-module-type-fit.test.ts" ||
    normalized.startsWith("scripts/semantic-module-type-fit/") ||
    normalized === "scripts/semantic-module-type-fit.mjs" ||
    normalized === "scripts/lib/semantic-module-type-fit.mjs" ||
    normalized === ".prettierignore" ||
    normalized.startsWith("analysis/semantic-module-type-fit/") ||
    normalized.startsWith("plan/PLAN-003-semantic-module-type-fit/") ||
    normalized.startsWith("spec/reviews/288-semantic-type-fit/") ||
    /^reviews\/2026-08-30-semantic-module-type-fit-(code-review|gap-analysis)\.md$/.test(
      normalized,
    ) ||
    [
      "spec/usecase/US-014-audit-default-module-semantic-fit.md",
      "spec/functional/FR-051-snapshot-semantic-audit-scope.md",
      "spec/functional/FR-052-inventory-default-module-corpus.md",
      "spec/functional/FR-053-score-semantic-type-fit.md",
      "spec/functional/FR-054-publish-semantic-audit-artifacts.md",
      "spec/functional/FR-055-reconcile-semantic-audit-findings.md",
      "spec/non-functional/NFR-015-complete-reproducible-semantic-audit.md",
      "spec/non-functional/NFR-016-read-only-semantic-audit.md",
      "spec/usecase/index.md",
      "spec/functional/index.md",
      "spec/non-functional/index.md",
      "spec/spec.md",
      "spec/matrix.md",
      "spec/log.md",
    ].includes(normalized)
  );
}
