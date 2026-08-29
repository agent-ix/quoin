import { readFileSync, readdirSync } from "node:fs";
import { extname, join, relative } from "node:path";

import type { MockInjection, MockInspectionRecord } from "./types.js";
import { listRecordedSuites, readMockInspection } from "./store.js";

const SOURCE_EXTENSIONS = new Set([".rs", ".py", ".ts", ".tsx", ".js", ".jsx"]);
const EXCLUDED_DIRECTORIES = new Set([
  ".git",
  "dist",
  "node_modules",
  "spec",
  "target",
  "vendor",
]);

interface TestMarker {
  at: number;
  end: number;
  symbol: string;
}

/** Exact-commit inspection input for the pure auditor. */
export function mockInspectionInput(
  repo: string,
  commit: string | undefined,
): { injections: MockInjection[]; suites: string[] } {
  if (!commit) return { injections: [], suites: [] };
  const records = listRecordedSuites(repo)
    .map((suite) => readMockInspection(repo, suite, commit))
    .filter(
      (record): record is MockInspectionRecord =>
        record !== null && record.commit === commit,
    );
  return {
    suites: records.map((record) => record.suite).sort(compare),
    injections: records
      .flatMap((record) => record.injections)
      .sort(
        (a, b) =>
          compare(a.suite, b.suite) ||
          compare(a.path ?? "", b.path ?? "") ||
          (a.line ?? 0) - (b.line ?? 0),
      ),
  };
}

/**
 * Inspect test source for explicit stand-in constructors.
 *
 * This is deliberately narrow. It recognizes names declaring their intent
 * (`Mock*`, `Fake*`, `Stub*`) and explicit permissive factories such as
 * `Confirmation::allow()`. The auditor independently decides whether that
 * identifier overlaps the behaviour an obligation claims to verify.
 */
export function inspectMockInjections(
  repo: string,
  suite: string,
): MockInjection[] {
  const out: MockInjection[] = [];
  for (const path of sourceFiles(repo)) {
    const source = readFileSync(path, "utf8");
    const markers = testMarkers(source, extname(path));
    if (markers.length === 0) continue;

    const call =
      /\b([A-Z][A-Za-z0-9_]*(?:(?:::|\.)[A-Za-z_][A-Za-z0-9_]*)+)\s*\(/g;
    for (const match of source.matchAll(call)) {
      const identifier = match[1];
      if (!looksLikeStandIn(identifier)) continue;
      const marker = nearestMarker(markers, match.index ?? -1);
      if (!marker) continue;
      out.push({
        suite,
        symbol: marker.symbol,
        injects: [identifier],
        path: relative(repo, path),
        line: source.slice(0, match.index).split("\n").length,
      });
    }
  }

  const merged = new Map<string, MockInjection>();
  for (const item of out) {
    const key = `${item.path ?? ""}:${item.line ?? 0}:${item.symbol}`;
    const prior = merged.get(key);
    if (prior) {
      prior.injects = [...new Set([...prior.injects, ...item.injects])].sort();
    } else {
      merged.set(key, { ...item, injects: [...item.injects].sort() });
    }
  }
  return [...merged.values()].sort(
    (a, b) =>
      compare(a.path ?? "", b.path ?? "") ||
      (a.line ?? 0) - (b.line ?? 0) ||
      compare(a.symbol, b.symbol),
  );
}

function sourceFiles(root: string): string[] {
  const out: string[] = [];
  const walk = (dir: string): void => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory() && EXCLUDED_DIRECTORIES.has(entry.name)) continue;
      const path = join(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.isFile() && SOURCE_EXTENSIONS.has(extname(path)))
        out.push(path);
    }
  };
  walk(root);
  return out.sort(compare);
}

function testMarkers(source: string, extension: string): TestMarker[] {
  const markers: TestMarker[] = [];
  const patterns =
    extension === ".rs"
      ? [
          /#\s*\[\s*test\s*\][\s\S]{0,240}?\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g,
        ]
      : extension === ".py"
        ? [/(?:^|\n)\s*(?:async\s+)?def\s+(test_[A-Za-z0-9_]*)\s*\(/g]
        : [
            /\b(?:test|it)\s*\(\s*["'`]([^"'`]+)["'`]/g,
            /\b(?:async\s+)?function\s+(test[A-Za-z0-9_]*)\s*\(/g,
          ];
  for (const pattern of patterns) {
    for (const match of source.matchAll(pattern)) {
      const matchAt = match.index ?? 0;
      // The Python pattern consumes the preceding newline. Starting the span
      // there makes pythonFunctionEnd see that same newline as the end of the
      // signature and terminate before the function body. Anchor on the
      // captured function name instead.
      const at =
        extension === ".py"
          ? matchAt + match[0].lastIndexOf(match[1])
          : matchAt;
      const end =
        extension === ".py"
          ? pythonFunctionEnd(source, at)
          : bracedBodyEnd(source, at + match[0].length);
      markers.push({ at, end, symbol: match[1] });
    }
  }
  return markers.sort((a, b) => a.at - b.at);
}

function nearestMarker(markers: TestMarker[], at: number): TestMarker | null {
  let found: TestMarker | null = null;
  for (const marker of markers) {
    if (marker.at > at) break;
    if (at <= marker.end) found = marker;
  }
  return found;
}

function pythonFunctionEnd(source: string, at: number): number {
  const lineStart = source.lastIndexOf("\n", at) + 1;
  const indentation = /^\s*/.exec(source.slice(lineStart))?.[0].length ?? 0;
  let next = source.indexOf("\n", at);
  if (next < 0) return source.length;
  next += 1;
  while (next < source.length) {
    const end = source.indexOf("\n", next);
    const lineEnd = end < 0 ? source.length : end;
    const line = source.slice(next, lineEnd);
    if (line.trim() && !line.trimStart().startsWith("#")) {
      const current = /^\s*/.exec(line)?.[0].length ?? 0;
      if (current <= indentation) return next - 1;
    }
    next = lineEnd + 1;
  }
  return source.length;
}

/** Find the closing brace while ignoring braces inside strings and comments. */
function bracedBodyEnd(source: string, from: number): number {
  const open = source.indexOf("{", from);
  if (open < 0 || open - from > 500) return from;
  let depth = 0;
  let quote: string | null = null;
  let escaped = false;
  let lineComment = false;
  let blockComment = false;
  for (let index = open; index < source.length; index += 1) {
    const character = source[index];
    const next = source[index + 1] ?? "";
    if (lineComment) {
      if (character === "\n") lineComment = false;
      continue;
    }
    if (blockComment) {
      if (character === "*" && next === "/") {
        blockComment = false;
        index += 1;
      }
      continue;
    }
    if (quote) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === quote) quote = null;
      continue;
    }
    if (character === "/" && next === "/") {
      lineComment = true;
      index += 1;
    } else if (character === "/" && next === "*") {
      blockComment = true;
      index += 1;
    } else if (character === '"' || character === "'" || character === "`") {
      quote = character;
    } else if (character === "{") {
      depth += 1;
    } else if (character === "}" && --depth === 0) {
      return index;
    }
  }
  return source.length;
}

function looksLikeStandIn(identifier: string): boolean {
  const parts = identifier.split(/::|\./);
  const type = parts[0];
  const method = parts.at(-1) ?? "";
  return (
    /^(?:mock|fake|stub|spy)/i.test(type) ||
    /^(?:allow|approve|approved|bypass|fake|mock|stub|succeed|success)$/i.test(
      method,
    )
  );
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
