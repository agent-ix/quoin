import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

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

  const findingsFor = (quire, corpusRoot, module, mapping, quoin) => {
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
          message: diagnostic.message ?? "",
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
          message: suspicion.message ?? "",
          evidence: suspicion.evidence ?? "",
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

    const auditFamilies = bySource("audit.findings");
    if (auditFamilies.length > 0) {
      if (!quoin) {
        throw new Error(
          "bench-tier1: the mapping declares audit.findings but no quoin CLI was supplied",
        );
      }
      const audit = auditFixture({
        quire,
        quoin,
        corpusRoot,
        module,
        single,
        env,
        obligations: payload.obligations ?? [],
      });
      for (const [family, definition] of auditFamilies) {
        for (const finding of audit.findings ?? []) {
          if (finding.kind !== definition.key) continue;
          findings.push({
            family,
            reason: finding.kind,
            path: finding.path ?? null,
            line: typeof finding.line === "number" ? finding.line : null,
            message: finding.summary ?? "",
          });
        }
      }
    }

    const quoinValidateFamilies = bySource("quoin.validate");
    if (quoinValidateFamilies.length > 0) {
      if (!quoin) {
        throw new Error(
          "bench-tier1: the mapping declares quoin.validate but no quoin CLI was supplied",
        );
      }
      const result = must(
        execute(process.execPath, [
          quoin,
          "validate",
          "--repo",
          corpusRoot,
          "--json",
        ]),
        "quoin validate",
        corpusRoot,
      );
      let validated;
      try {
        validated = JSON.parse(result.stdout);
      } catch {
        throw new Error(
          `bench-tier1: quoin validate produced no JSON for ${corpusRoot}`,
        );
      }
      for (const [family, definition] of quoinValidateFamilies) {
        for (const finding of validated.findings ?? []) {
          if (finding.kind !== definition.key) continue;
          findings.push({
            family,
            reason: finding.kind,
            path: finding.path ?? null,
            line: typeof finding.line === "number" ? finding.line : null,
            message: finding.summary ?? "",
          });
        }
      }
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
      findings.push({
        family,
        reason,
        path,
        line: Number(line),
        message: record.message,
      });
    }

    return {
      findings,
      metrics: payload.metrics ?? [],
      diagnostics: payload.diagnostics ?? [],
      untrackedSymbols: payload.untracked_symbols ?? [],
    };
  };

  /**
   * Exercise source inspection through the same store-backed command path a
   * consumer CI uses. The run result is a controlled-fixture premise, not a
   * claim about the corpus repository: it exists only inside this temporary
   * git repository and is derived from the obligations Quire just emitted.
   */
  const auditFixture = ({
    quire,
    quoin,
    corpusRoot,
    module,
    single,
    env,
    obligations,
  }) => {
    const scratch = mkdtempSync(join(tmpdir(), "quoin-tier1-audit-"));
    const repo = join(scratch, "repo");
    try {
      cpSync(corpusRoot, repo, { recursive: true });
      must(execute("git", ["init", "-q", repo]), "git init", corpusRoot);
      must(execute("git", ["-C", repo, "add", "."]), "git add", corpusRoot);
      must(
        execute("git", [
          "-C",
          repo,
          "-c",
          "user.name=Quoin Tier 1",
          "-c",
          "user.email=tier1@example.invalid",
          "commit",
          "-qm",
          "controlled fixture",
        ]),
        "git commit",
        corpusRoot,
      );
      const head = must(
        execute("git", ["-C", repo, "rev-parse", "HEAD"]),
        "git rev-parse",
        corpusRoot,
      ).stdout.trim();

      // Quoin intentionally resolves `quire` by name. Put the explicitly
      // selected benchmark binary at that name so the audit leg cannot drift
      // to a different installed engine.
      const toolBin = join(scratch, "bin");
      mkdirSync(toolBin);
      symlinkSync(resolve(quire), join(toolBin, "quire"));
      const commandEnv = {
        ...(env ?? {}),
        CI: "1",
        PATH: `${toolBin}:${process.env.PATH ?? ""}`,
      };
      const moduleArgs = single ? ["--module", module] : [];
      const invoke = (args, name) =>
        must(
          execute(process.execPath, [quoin, ...args], commandEnv),
          name,
          corpusRoot,
        );

      // Bind the symbols the production source inspector actually observed.
      // A suite-wide placeholder would let one mock contaminate unrelated
      // bindings and make the controlled benchmark reward a false positive.
      const inspected = invoke(
        [
          "evidence",
          "inspect-mocks",
          "--repo",
          repo,
          "--suite",
          "SUITE-TIER1",
          "--commit",
          head,
          "--dry-run",
          "--json",
        ],
        "quoin evidence inspect-mocks --dry-run",
      );
      let inspection;
      try {
        inspection = JSON.parse(inspected.stdout);
      } catch {
        throw new Error(
          `bench-tier1: quoin evidence inspect-mocks produced no JSON for ${corpusRoot}`,
        );
      }
      const inspectedSymbols = [
        ...new Set(
          (inspection.injections ?? []).map((injection) => injection.symbol),
        ),
      ].sort();
      const results = join(scratch, "run.json");
      writeFileSync(
        results,
        JSON.stringify({
          entries: (inspectedSymbols.length > 0
            ? inspectedSymbols
            : ["tier1::controlled_fixture"]
          ).map((symbol) => ({
            symbol,
            outcome: "pass",
            traceIds: obligations.map((obligation) => obligation.id),
          })),
        }),
      );

      invoke(
        [
          "evidence",
          "record",
          "--repo",
          repo,
          "--suite",
          "SUITE-TIER1",
          "--commit",
          head,
          "--tool",
          "tier1 controlled fixture",
          "--timestamp",
          "2000-01-01T00:00:00Z",
          "--results",
          results,
          ...moduleArgs,
          "--json",
        ],
        "quoin evidence record",
      );
      invoke(
        [
          "evidence",
          "inspect-mocks",
          "--repo",
          repo,
          "--suite",
          "SUITE-TIER1",
          "--commit",
          head,
          "--timestamp",
          "2000-01-01T00:00:00Z",
          "--json",
        ],
        "quoin evidence inspect-mocks",
      );
      const audited = invoke(
        ["evidence", "audit", "--repo", repo, ...moduleArgs, "--json"],
        "quoin evidence audit",
      );
      try {
        return JSON.parse(audited.stdout);
      } catch {
        throw new Error(
          `bench-tier1: quoin evidence audit produced no JSON for ${corpusRoot}`,
        );
      }
    } finally {
      rmSync(scratch, { recursive: true, force: true });
    }
  };

  const must = (result, operation, corpusRoot) => {
    if (result.ok) return result;
    throw new Error(
      `bench-tier1: ${operation} failed for ${corpusRoot}\n${result.stderr.trim()}`,
    );
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
