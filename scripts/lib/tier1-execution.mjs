import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";

const VALIDATE_LINE =
  /^(?<path>.+?): line (?<line>\d+): (?<rest>.*) \[(?<reason>[a-z-]+)\]$/;

/** Stateful subprocess boundary for one Tier-1 invocation. */
export function createTier1Executor() {
  let toolCalls = 0;

  const execute = (bin, args, extraEnv) => {
    toolCalls += 1;
    try {
      const stdout = execFileSync(bin, args, {
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
        stdio: ["ignore", "pipe", "pipe"],
        env: extraEnv ? { ...process.env, ...extraEnv } : process.env,
      });
      return { ok: true, stdout, stderr: "" };
    } catch (error) {
      return {
        ok: false,
        stdout: String(error.stdout ?? ""),
        stderr: String(error.stderr ?? error.message ?? ""),
      };
    }
  };

  const coverage = (quire, corpusRoot, module) => {
    const single = existsSync(join(module, "manifest.yaml"));
    const args = single
      ? ["coverage", "--scope", corpusRoot, "--module", module, "--json"]
      : ["coverage", "--scope", corpusRoot, "--json"];
    const env = single ? undefined : { IX_FILAMENT_MODULES_PATH: module };
    const result = execute(quire, args, env);
    try {
      return { payload: JSON.parse(result.stdout), single, env };
    } catch {
      throw new Error(
        `bench-tier1: \`quire coverage\` produced no JSON for ${corpusRoot}; ` +
          `refusing to read an unreadable run as a clean run\n${result.stderr.trim()}`,
      );
    }
  };

  const findingsFor = (quire, corpusRoot, module, mapping) => {
    const findings = [];
    const bySource = (source) =>
      Object.entries(mapping.families).filter(
        ([, value]) => value.source === source,
      );
    const { payload, single, env } = coverage(quire, corpusRoot, module);

    for (const [family, definition] of bySource("coverage.diagnostics")) {
      for (const diagnostic of payload.diagnostics ?? []) {
        if (diagnostic.reason !== definition.key) continue;
        findings.push({
          family,
          reason: diagnostic.reason,
          path: diagnostic.path ?? null,
          line: typeof diagnostic.line === "number" ? diagnostic.line : null,
          declaration: diagnostic.declaration ?? null,
        });
      }
    }
    for (const [family, definition] of bySource("coverage.suspicions")) {
      for (const suspicion of payload.suspicions ?? []) {
        if (suspicion.kind !== definition.key) continue;
        findings.push({
          family,
          reason: suspicion.kind,
          path: suspicion.path ?? null,
          line: typeof suspicion.line === "number" ? suspicion.line : null,
        });
      }
    }
    const metrics = new Map(
      (payload.metrics ?? []).map((metric) => [metric.name, metric]),
    );
    for (const [family, definition] of bySource("coverage.metrics")) {
      const metric = metrics.get(definition.key);
      if (!metric || metric.state !== "measured") continue;
      findings.push({
        family,
        reason: definition.key,
        path: null,
        line: null,
        metric: definition.key,
        value: Number(metric.value),
      });
    }

    const validateArgs = single
      ? [
          "validate",
          "--diagnostics-format",
          "json",
          "--scope",
          corpusRoot,
          "--module",
          module,
          "spec/*.md",
        ]
      : [
          "validate",
          "--diagnostics-format",
          "json",
          "--scope",
          corpusRoot,
          "spec/*.md",
        ];
    const validated = execute(quire, validateArgs, env);
    const byReason = new Map(
      bySource("validate.findings").map(([family, definition]) => [
        definition.key,
        family,
      ]),
    );
    for (const raw of validated.stderr.split("\n")) {
      const text = raw.trim();
      if (!text.startsWith("{")) continue;
      let record;
      try {
        record = JSON.parse(text);
      } catch {
        continue;
      }
      if (record.kind !== "ValidationError") continue;
      const parsed = VALIDATE_LINE.exec(record.message);
      if (!parsed) {
        if (/^.+: line \d+:/.test(record.message)) {
          throw new Error(
            `bench-tier1: could not parse a validate finding from ${record.message}`,
          );
        }
        continue;
      }
      const { path, line, reason } = parsed.groups;
      const family = byReason.get(reason);
      if (!family) continue;
      const contains = mapping.families[family]?.contains;
      if (contains && !record.message.includes(contains)) continue;
      findings.push({ family, reason, path, line: Number(line) });
    }

    return {
      findings,
      metrics: payload.metrics ?? [],
      diagnostics: payload.diagnostics ?? [],
    };
  };

  const rawReasons = (quire, corpusRoot, module) => {
    const { payload } = coverage(quire, corpusRoot, module);
    return new Set([
      ...(payload.diagnostics ?? []).map((item) => item.reason),
      ...(payload.suspicions ?? []).map((item) => item.kind),
    ]);
  };

  const assertEngine = (quire) => {
    const probe = execute(quire, ["--version"]);
    const version = probe.stdout.trim();
    if (!version.includes("engine")) {
      throw new Error(
        `bench-tier1: ${quire} reports ${JSON.stringify(version)} without an engine version`,
      );
    }
    return version;
  };

  return {
    execute,
    findingsFor,
    rawReasons,
    assertEngine,
    toolCalls: () => toolCalls,
  };
}
