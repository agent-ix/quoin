import { readFileSync, readdirSync } from "node:fs";
import { basename, extname, join, relative } from "node:path";

const EXCLUDED = new Set([
  ".git",
  "dist",
  "node_modules",
  "spec",
  "target",
  "vendor",
]);

export interface EmptyGateFinding {
  kind: "gate-that-gates-nothing";
  obligation: string;
  path: string;
  line: number;
  wiredBy: string;
  summary: string;
}

/**
 * Find a declared, wired shell gate that counts forbidden matches but never
 * asserts the count. The three-way join is deliberate: script text alone
 * cannot distinguish a gate from a report (agent-ix/quoin#224).
 */
export function inspectEmptyGates(repo: string): EmptyGateFinding[] {
  const wiring = wiringFiles(repo);
  const findings: EmptyGateFinding[] = [];
  for (const path of shellFiles(repo)) {
    const source = readFileSync(path, "utf8");
    const claim = gateClaim(source);
    if (!claim || !negativeClaim(claim.statement)) continue;
    const repoPath = portable(relative(repo, path));
    const wiredBy = wiring.find((candidate) =>
      referencesScript(readFileSync(candidate, "utf8"), repoPath),
    );
    if (!wiredBy) continue;

    const lines = source.split(/\r?\n/);
    for (const [index, line] of lines.entries()) {
      const pattern = unassertedCountPattern(line);
      if (!pattern || !claimMentions(claim.statement, pattern)) continue;
      const wirePath = portable(relative(repo, wiredBy));
      findings.push({
        kind: "gate-that-gates-nothing",
        obligation: claim.obligation,
        path: repoPath,
        line: index + 1,
        wiredBy: wirePath,
        summary:
          `${claim.obligation} claims “${claim.statement}”, and ${wirePath} ` +
          `wires ${repoPath}, but line ${index + 1} only counts matches for ` +
          `“${pattern}” without asserting zero. The gate succeeds when the ` +
          `forbidden text is present; compare the count to zero or make a ` +
          `match exit non-zero.`,
      });
    }
  }
  return findings.sort(
    (a, b) =>
      compare(a.path, b.path) ||
      a.line - b.line ||
      compare(a.obligation, b.obligation),
  );
}

function gateClaim(
  source: string,
): { obligation: string; statement: string } | null {
  const match =
    /^\s*#\s*(?:gate|check)\s+for\s+([A-Z][A-Z0-9-]+):\s*(.+?)\s*$/im.exec(
      source,
    );
  return match ? { obligation: match[1], statement: match[2] } : null;
}

function negativeClaim(statement: string): boolean {
  return /\b(?:no|never|not|forbid(?:s|den)?|without)\b/i.test(statement);
}

/** Return the searched token when a line merely prints its match count. */
function unassertedCountPattern(line: string): string | null {
  if (/^\s*(?:if|while|until)\b/.test(line)) return null;
  const counter = /\|\s*wc\s+-?l\b/.exec(line);
  if (!/\b(?:grep|rg)\b/.test(line) || !counter) {
    return null;
  }
  const afterCounter = line.slice((counter.index ?? 0) + counter[0].length);
  if (/\|\s*(?:grep|test)\b/.test(afterCounter)) {
    return null;
  }
  if (/(?:==|!=|\b(?:eq|ne|gt|ge|lt|le)\b)/.test(afterCounter)) return null;
  const quoted = /\b(?:grep|rg)\b[^\n]*?["']([^"']+)["']/.exec(line)?.[1];
  return quoted?.trim() || null;
}

function claimMentions(statement: string, pattern: string): boolean {
  const claimWords = identifiers(statement);
  return [...identifiers(pattern)].some(
    (word) => word.length >= 4 && claimWords.has(word),
  );
}

function identifiers(value: string): Set<string> {
  return new Set(value.toLowerCase().match(/[a-z_][a-z0-9_]*/g) ?? []);
}

function referencesScript(source: string, repoPath: string): boolean {
  return (
    source.includes(repoPath) ||
    source.includes(`./${repoPath}`) ||
    source.includes(basename(repoPath))
  );
}

function shellFiles(repo: string): string[] {
  return walk(repo).filter((path) => extname(path) === ".sh");
}

function wiringFiles(repo: string): string[] {
  return walk(repo).filter((path) => {
    const rel = portable(relative(repo, path));
    const name = basename(path).toLowerCase();
    return (
      /^makefile(?:\..+)?$/.test(name) ||
      name === "justfile" ||
      name === "package.json" ||
      /^taskfile\.(?:ya?ml)$/.test(name) ||
      (/^\.github\/workflows\//.test(rel) && /\.ya?ml$/.test(rel))
    );
  });
}

function walk(root: string): string[] {
  const out: string[] = [];
  const visit = (dir: string): void => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory() && EXCLUDED.has(entry.name)) continue;
      const path = join(dir, entry.name);
      if (entry.isDirectory()) visit(path);
      else if (entry.isFile()) out.push(path);
    }
  };
  visit(root);
  return out.sort(compare);
}

function portable(path: string): string {
  return path.split("\\").join("/");
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
