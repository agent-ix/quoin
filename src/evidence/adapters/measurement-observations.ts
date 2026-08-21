/** Pure raw-output adapters for generic measurement observations (FR-045). */

import { createHash } from "node:crypto";
import { isAbsolute, relative, sep } from "node:path";

import { AdapterError } from "./types.js";

export interface MeasurementObservation {
  subject: { kind: string; id: string };
  path?: string;
  value: number;
  unit?: string;
}

export interface MeasurementAdapterResult {
  observations: MeasurementObservation[];
  /** Anything omitted or uncertain. The command refuses to store these. */
  limitations: string[];
}

export interface MeasurementAdapter {
  readonly name: string;
  readonly summary: string;
  parse(raw: string, sourceRoot: string): MeasurementAdapterResult;
}

interface RcaSpace {
  name?: unknown;
  kind?: unknown;
  spaces?: unknown;
  metrics?: unknown;
}

interface RcaUnit {
  name?: unknown;
  spaces?: unknown;
}

function normalizedPath(adapter: string, value: unknown, root: string): string {
  if (typeof value !== "string" || value === "") {
    throw new AdapterError(adapter, "an observation has no source path");
  }
  let path = value;
  if (isAbsolute(path)) path = relative(root, path);
  path = path.split(sep).join("/").replace(/^\.\//, "");
  if (path === ".." || path.startsWith("../") || isAbsolute(path)) {
    throw new AdapterError(
      adapter,
      `source path is outside --repo and cannot be normalized: ${value}`,
    );
  }
  return path;
}

function finite(adapter: string, value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new AdapterError(adapter, `${label} is not a finite number`);
  }
  return value;
}

function object(
  adapter: string,
  value: unknown,
  label: string,
): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new AdapterError(adapter, `${label} is not an object`);
  }
  return value as Record<string, unknown>;
}

function array(adapter: string, value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) {
    throw new AdapterError(adapter, `${label} is not an array`);
  }
  return value;
}

function json(adapter: string, raw: string): unknown {
  try {
    return JSON.parse(raw) as unknown;
  } catch (cause) {
    throw new AdapterError(
      adapter,
      `input is not JSON: ${(cause as Error).message}`,
    );
  }
}

function unique(
  adapter: string,
  observations: MeasurementObservation[],
): MeasurementAdapterResult {
  const seen = new Set<string>();
  for (const observation of observations) {
    const key = JSON.stringify(observation.subject);
    if (seen.has(key)) {
      throw new AdapterError(
        adapter,
        `duplicate stable subject ${observation.subject.kind}:${observation.subject.id}`,
      );
    }
    seen.add(key);
  }
  return { observations, limitations: [] };
}

export const normalizedMeasurementAdapter: MeasurementAdapter = {
  name: "observations",
  summary: 'Normalized {"observations": [{subject, path?, value, unit?}]}.',
  parse(raw: string): MeasurementAdapterResult {
    const envelope = object(this.name, json(this.name, raw), "input");
    const limitations =
      envelope.limitations === undefined
        ? []
        : array(this.name, envelope.limitations, "limitations").map((item) => {
            if (typeof item !== "string" || item === "") {
              throw new AdapterError(
                this.name,
                "limitations must be non-empty strings",
              );
            }
            return item;
          });
    const observations = array(
      this.name,
      envelope.observations,
      "observations",
    ).map((rawObservation): MeasurementObservation => {
      const item = object(this.name, rawObservation, "observation");
      const subject = object(this.name, item.subject, "observation.subject");
      if (
        typeof subject.kind !== "string" ||
        subject.kind === "" ||
        typeof subject.id !== "string" ||
        subject.id === ""
      ) {
        throw new AdapterError(
          this.name,
          "observation subject needs kind and id",
        );
      }
      if (
        item.path !== undefined &&
        (typeof item.path !== "string" || item.path === "")
      ) {
        throw new AdapterError(
          this.name,
          "observation path must be a non-empty string",
        );
      }
      if (
        item.unit !== undefined &&
        (typeof item.unit !== "string" || item.unit === "")
      ) {
        throw new AdapterError(
          this.name,
          "observation unit must be a non-empty string",
        );
      }
      return {
        subject: { kind: subject.kind, id: subject.id },
        ...(item.path === undefined ? {} : { path: item.path }),
        value: finite(this.name, item.value, "observation value"),
        ...(item.unit === undefined ? {} : { unit: item.unit }),
      };
    });
    unique(this.name, observations);
    return { observations, limitations };
  },
};

export const rustCodeAnalysisAdapter: MeasurementAdapter = {
  name: "rust-code-analysis-cyclomatic",
  summary:
    "rust-code-analysis JSONL; named function cyclomatic sums only (no impl/class aggregates).",
  parse(raw: string, sourceRoot: string): MeasurementAdapterResult {
    const observations: MeasurementObservation[] = [];
    for (const [index, line] of raw.split(/\r?\n/).entries()) {
      if (line.trim() === "") continue;
      let parsed: unknown;
      try {
        parsed = JSON.parse(line) as unknown;
      } catch (cause) {
        throw new AdapterError(
          this.name,
          `line ${index + 1} is not JSON: ${(cause as Error).message}`,
        );
      }
      const unit = object(this.name, parsed, `line ${index + 1}`) as RcaUnit;
      const path = normalizedPath(this.name, unit.name, sourceRoot);
      const visit = (rawSpaces: unknown, parents: string[]): void => {
        for (const rawSpace of array(
          this.name,
          rawSpaces ?? [],
          `${path} spaces`,
        )) {
          const space = object(
            this.name,
            rawSpace,
            `${path} space`,
          ) as RcaSpace;
          const name = typeof space.name === "string" ? space.name : "";
          const nextParents =
            name === "" || name === "<anonymous>"
              ? parents
              : [...parents, name];
          if (
            space.kind === "function" &&
            name !== "" &&
            name !== "<anonymous>"
          ) {
            const metrics = object(
              this.name,
              space.metrics,
              `${path}#${name} metrics`,
            );
            const cyclomatic = object(
              this.name,
              metrics.cyclomatic,
              `${path}#${name} cyclomatic`,
            );
            const qualified = nextParents.join("::");
            observations.push({
              subject: { kind: "function", id: `${path}#${qualified}` },
              path,
              value: finite(
                this.name,
                cyclomatic.sum,
                `${path}#${qualified} sum`,
              ),
              unit: "control-flow-path-count",
            });
          }
          visit(space.spaces ?? [], nextParents);
        }
      };
      visit(unit.spaces ?? [], []);
    }
    return unique(this.name, observations);
  },
};

function csvRows(adapter: string, raw: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let quoted = false;
  for (let index = 0; index < raw.length; index += 1) {
    const character = raw[index];
    if (character === '"') {
      if (quoted && raw[index + 1] === '"') {
        field += '"';
        index += 1;
      } else {
        quoted = !quoted;
      }
    } else if (character === "," && !quoted) {
      row.push(field);
      field = "";
    } else if ((character === "\n" || character === "\r") && !quoted) {
      if (character === "\r" && raw[index + 1] === "\n") index += 1;
      row.push(field);
      if (row.some((item) => item !== "")) rows.push(row);
      row = [];
      field = "";
    } else {
      field += character;
    }
  }
  if (quoted) throw new AdapterError(adapter, "unterminated quoted CSV field");
  row.push(field);
  if (row.some((item) => item !== "")) rows.push(row);
  return rows;
}

export const lizardAdapter: MeasurementAdapter = {
  name: "lizard-cyclomatic",
  summary: "Lizard --csv function cyclomatic observations.",
  parse(raw: string, sourceRoot: string): MeasurementAdapterResult {
    const observations = csvRows(this.name, raw).map((row, index) => {
      if (row.length < 11) {
        throw new AdapterError(
          this.name,
          `CSV row ${index + 1} has ${row.length} columns, expected 11`,
        );
      }
      const path = normalizedPath(this.name, row[6], sourceRoot);
      const name = row[7];
      if (name === "" || name === "(anonymous)") {
        throw new AdapterError(
          this.name,
          `CSV row ${index + 1} has no stable named symbol`,
        );
      }
      const value = Number(row[1]);
      return {
        subject: { kind: "function", id: `${path}#${name}` },
        path,
        value: finite(this.name, value, `CSV row ${index + 1} complexity`),
        unit: "control-flow-path-count",
      };
    });
    return unique(this.name, observations);
  },
};

export const radonAdapter: MeasurementAdapter = {
  name: "radon-cyclomatic",
  summary: "Radon cc --json named function and method observations.",
  parse(raw: string, sourceRoot: string): MeasurementAdapterResult {
    const parsed = object(this.name, json(this.name, raw), "input");
    const observations: MeasurementObservation[] = [];
    const visit = (
      path: string,
      rawBlocks: unknown,
      parents: string[],
    ): void => {
      for (const rawBlock of array(this.name, rawBlocks, `${path} blocks`)) {
        const block = object(this.name, rawBlock, `${path} block`);
        const name = typeof block.name === "string" ? block.name : "";
        const type = typeof block.type === "string" ? block.type : "";
        const nextParents = name === "" ? parents : [...parents, name];
        if ((type === "function" || type === "method") && name !== "") {
          observations.push({
            subject: {
              kind: "function",
              id: `${path}#${nextParents.join("::")}`,
            },
            path,
            value: finite(
              this.name,
              block.complexity,
              `${path}#${name} complexity`,
            ),
            unit: "control-flow-path-count",
          });
        }
        if (block.methods !== undefined)
          visit(path, block.methods, nextParents);
        if (block.closures !== undefined)
          visit(path, block.closures, nextParents);
      }
    };
    for (const [rawPath, blocks] of Object.entries(parsed)) {
      visit(normalizedPath(this.name, rawPath, sourceRoot), blocks, []);
    }
    return unique(this.name, observations);
  },
};

export const gitHotChurnAdapter: MeasurementAdapter = {
  name: "git-hot-churn",
  summary: "git-hot CSV: max live-line churn per current file.",
  parse(raw: string, sourceRoot: string): MeasurementAdapterResult {
    const observations = csvRows(this.name, raw).map((row, index) => {
      if (row.length !== 3) {
        throw new AdapterError(
          this.name,
          `CSV row ${index + 1} needs churn, age, path`,
        );
      }
      const path = normalizedPath(this.name, row[2], sourceRoot);
      return {
        subject: { kind: "source-file", id: path },
        path,
        value: finite(this.name, Number(row[0]), `CSV row ${index + 1} churn`),
        unit: "live-line-change-count",
      };
    });
    return unique(this.name, observations);
  },
};

export const gitHotAgeAdapter: MeasurementAdapter = {
  name: "git-hot-age",
  summary: "git-hot CSV: median live-line age in days per current file.",
  parse(raw: string, sourceRoot: string): MeasurementAdapterResult {
    const observations = csvRows(this.name, raw).map((row, index) => {
      if (row.length !== 3) {
        throw new AdapterError(
          this.name,
          `CSV row ${index + 1} needs churn, age, path`,
        );
      }
      const path = normalizedPath(this.name, row[2], sourceRoot);
      return {
        subject: { kind: "source-file", id: path },
        path,
        value: finite(this.name, Number(row[1]), `CSV row ${index + 1} age`),
        unit: "days",
      };
    });
    return unique(this.name, observations);
  },
};

export const jscpdAdapter: MeasurementAdapter = {
  name: "jscpd-clone-pairs",
  summary: "jscpd JSON report: one duplicated-line observation per clone pair.",
  parse(raw: string, sourceRoot: string): MeasurementAdapterResult {
    const report = object(this.name, json(this.name, raw), "input");
    const observations = array(this.name, report.duplicates, "duplicates").map(
      (rawDuplicate, index): MeasurementObservation => {
        const duplicate = object(
          this.name,
          rawDuplicate,
          `duplicate ${index + 1}`,
        );
        const first = object(
          this.name,
          duplicate.firstFile,
          `duplicate ${index + 1} firstFile`,
        );
        const second = object(
          this.name,
          duplicate.secondFile,
          `duplicate ${index + 1} secondFile`,
        );
        const left = normalizedPath(this.name, first.name, sourceRoot);
        const right = normalizedPath(this.name, second.name, sourceRoot);
        const fragment =
          typeof duplicate.fragment === "string" ? duplicate.fragment : "";
        if (fragment === "") {
          throw new AdapterError(
            this.name,
            `duplicate ${index + 1} has no fragment identity`,
          );
        }
        const pair = [left, right].sort();
        const fragmentDigest = createHash("sha256")
          .update(fragment)
          .digest("hex")
          .slice(0, 16);
        return {
          subject: {
            kind: "clone-pair",
            id: `${pair[0]}|${pair[1]}#${fragmentDigest}`,
          },
          value: finite(
            this.name,
            duplicate.lines,
            `duplicate ${index + 1} lines`,
          ),
          unit: "duplicated-lines",
        };
      },
    );
    return unique(this.name, observations);
  },
};

export const MEASUREMENT_ADAPTERS: readonly MeasurementAdapter[] = [
  normalizedMeasurementAdapter,
  rustCodeAnalysisAdapter,
  lizardAdapter,
  radonAdapter,
  gitHotChurnAdapter,
  gitHotAgeAdapter,
  jscpdAdapter,
];

export const MEASUREMENT_ADAPTER_NAMES = MEASUREMENT_ADAPTERS.map(
  (item) => item.name,
);

export function selectMeasurementAdapter(
  name: string | undefined,
): MeasurementAdapter {
  const selected = name ?? normalizedMeasurementAdapter.name;
  const adapter = MEASUREMENT_ADAPTERS.find((item) => item.name === selected);
  if (!adapter) {
    throw new AdapterError(
      "measurement-adapter",
      `unknown adapter '${selected}'. Available: ${MEASUREMENT_ADAPTER_NAMES.join(", ")}`,
    );
  }
  return adapter;
}
