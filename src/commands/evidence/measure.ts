import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  MEASUREMENT_ADAPTER_NAMES,
  selectMeasurementAdapter,
  writeMeasurementCollection,
  type MeasurementCollection,
  type MeasurementObservation,
} from "../../evidence/index.js";

export default class EvidenceMeasure extends QuoinCommand {
  static summary =
    "Transcribe policy-free observations into an atomic measurement collection.";
  static description = `Reads a completed tool report and writes one atomic collection of logical
MeasurementRecords. This command never runs the producer and never decides whether a value is
good. The authored plan and definition version supply that meaning; comparison policy is
a separate caller-owned decision.

Collection fails closed when the adapter reports a limitation, produces no observations,
or disagrees with --expected-count. A missing parser or empty traversal therefore cannot
become a clean baseline.`;

  static examples = [
    "quoin evidence measure --plan MP-001 --definition rust-structure-v1 --repository agent-ix/service --revision $(git rev-parse HEAD) --tool rust-code-analysis --tool-version 0.0.25 --configuration-digest sha256:abc123 --environment linux-x64 --adapter rust-code-analysis-cyclomatic-file-distribution --results metrics.jsonl",
  ];

  static flags = {
    plan: Flags.string({
      description: "Authored MeasurementPlan id.",
      required: true,
    }),
    definition: Flags.string({
      description: "Versioned measure definition selected by the plan.",
      required: true,
    }),
    repository: Flags.string({
      description: "Stable logical repository identity stored in every record.",
      required: true,
    }),
    revision: Flags.string({
      description: "Source revision the producer observed.",
      required: true,
    }),
    tool: Flags.string({ description: "Producer name.", required: true }),
    "tool-version": Flags.string({
      description: "Producer version.",
      required: true,
    }),
    "configuration-digest": Flags.string({
      description:
        "Algorithm-prefixed digest of the complete producer configuration.",
      required: true,
    }),
    environment: Flags.string({
      description: "Comparable execution-environment identity.",
      required: true,
    }),
    attribute: Flags.string({
      description: "Environment attribute as key=value; repeatable.",
      multiple: true,
    }),
    sampling: Flags.string({ description: "Sampling design identity." }),
    "sample-count": Flags.string({ description: "Positive sample count." }),
    unit: Flags.string({
      description: "Unit for observations whose adapter does not declare one.",
    }),
    adapter: Flags.string({
      description: `Raw observation reader (${MEASUREMENT_ADAPTER_NAMES.join(", ")}).`,
      options: [...MEASUREMENT_ADAPTER_NAMES],
      default: "observations",
    }),
    results: Flags.string({
      description: "Completed producer output; `-` reads stdin.",
      required: true,
    }),
    "expected-count": Flags.string({
      description:
        "Exact observation population expected from this collection.",
    }),
    reference: Flags.string({
      description:
        "Durable reference to the raw output whose digest is stored.",
    }),
    repo: Flags.string({
      description: "Repository root for path normalization and store writes.",
      default: ".",
    }),
    timestamp: Flags.string({
      description: "UTC collection time. Defaults to now.",
    }),
    json: Flags.boolean({ description: "Emit the outcome as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceMeasure);
    const raw =
      flags.results === "-"
        ? readFileSync(0, "utf8")
        : readFileSync(flags.results, "utf8");
    const adapter = selectMeasurementAdapter(flags.adapter);
    const result = adapter.parse(raw, resolve(flags.repo));
    if (result.limitations.length > 0) {
      this.error(
        `measurement collection is incomplete: ${result.limitations.join("; ")}`,
        { exit: 2 },
      );
    }
    if (result.observations.length === 0) {
      this.error(
        "measurement collection produced zero observations; record an explicit aggregate zero rather than treating empty output as clean",
        { exit: 2 },
      );
    }

    const expected = parseCount(
      flags["expected-count"],
      "--expected-count",
      true,
    );
    if (expected !== undefined && expected !== result.observations.length) {
      this.error(
        `measurement collection is incomplete: expected ${expected} observations, parsed ${result.observations.length}`,
        { exit: 2 },
      );
    }
    const sampleCount = parseCount(
      flags["sample-count"],
      "--sample-count",
      false,
    );
    if ((flags.sampling === undefined) !== (sampleCount === undefined)) {
      this.error("--sampling and --sample-count must be supplied together", {
        exit: 2,
      });
    }

    const attributes = parseAttributes(flags.attribute ?? []);
    const collectedAt = flags.timestamp ?? new Date().toISOString();
    const rawDigest = `sha256:${createHash("sha256").update(raw).digest("hex")}`;
    const observations = [...result.observations]
      .sort(compareObservations)
      .map((observation) => {
        const unit = observation.unit ?? flags.unit;
        if (!unit) {
          this.error(
            `${observation.subject.kind}:${observation.subject.id} has no unit; supply --unit or use an adapter that declares one`,
            { exit: 2 },
          );
        }
        validateNormalizedPath(observation.path);
        return {
          subject: observation.subject,
          ...(observation.path === undefined ? {} : { path: observation.path }),
          value: observation.value,
          unit,
          ...(observation.distribution === undefined
            ? {}
            : { distribution: observation.distribution }),
        };
      });

    const collection: MeasurementCollection = {
      schemaVersion: 1,
      plan: { id: flags.plan, definitionVersion: flags.definition },
      repository: flags.repository,
      sourceRevision: flags.revision,
      tool: {
        name: flags.tool,
        version: flags["tool-version"],
        configurationDigest: flags["configuration-digest"],
      },
      environment: {
        id: flags.environment,
        ...(Object.keys(attributes).length === 0 ? {} : { attributes }),
      },
      ...(flags.sampling === undefined
        ? {}
        : { sampling: { id: flags.sampling, sampleCount: sampleCount! } }),
      collectedAt,
      rawEvidence: {
        digest: rawDigest,
        ...(flags.reference === undefined
          ? {}
          : { reference: flags.reference }),
      },
      observations,
    };
    const path = writeMeasurementCollection(flags.repo, collection);
    if (flags.json) {
      this.log(
        JSON.stringify({ adapter: adapter.name, collection, path }, null, 2),
      );
      return;
    }
    this.log(
      `recorded ${observations.length} observation(s) with ${adapter.name}`,
    );
    this.log(`  ${path}`);
  }
}

function parseCount(
  raw: string | undefined,
  flag: string,
  allowZero: boolean,
): number | undefined {
  if (raw === undefined) return undefined;
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value < (allowZero ? 0 : 1)) {
    throw new Error(
      `${flag} must be ${allowZero ? "a non-negative" : "a positive"} integer`,
    );
  }
  return value;
}

function parseAttributes(values: string[]): Record<string, string> {
  const attributes: Record<string, string> = {};
  for (const value of values) {
    const separator = value.indexOf("=");
    if (separator < 1 || separator === value.length - 1) {
      throw new Error(`--attribute must be key=value, got '${value}'`);
    }
    const key = value.slice(0, separator);
    if (attributes[key] !== undefined) {
      throw new Error(`duplicate --attribute key '${key}'`);
    }
    attributes[key] = value.slice(separator + 1);
  }
  return attributes;
}

function validateNormalizedPath(path: string | undefined): void {
  if (
    path !== undefined &&
    (path === ".." || path.startsWith("../") || isAbsolute(path))
  ) {
    throw new Error(
      `measurement scope path must be repository-relative: ${path}`,
    );
  }
}

function compareObservations(
  left: MeasurementObservation,
  right: MeasurementObservation,
): number {
  const a = `${left.subject.kind}\0${left.subject.id}`;
  const b = `${right.subject.kind}\0${right.subject.id}`;
  return a === b ? 0 : a < b ? -1 : 1;
}
