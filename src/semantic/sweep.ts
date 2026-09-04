/**
 * Legacy Properties-form detection and the advisory sweep (FR-074, issue #293).
 *
 * Quire emits the per-artifact `semantic.legacy-properties-form` warning at
 * validation time (quire-rs#388). This module is Quoin's side: the same
 * classifier, run over a corpus root by `quoin semantic sweep`, producing the
 * report that `semantic.legacy_forms: error` must cite before a module may
 * promote the warning (the advisory sweep of quoin#291 runs it at scale).
 *
 * The classifier reads Markdown as text: it never rewrites an artifact.
 */

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

export type PropertiesForm =
  "typed-table" | "free-column-table" | "bullet-list" | "sysml-fence" | "none";

export const TYPED_HEADER = ["Field", "Type", "Multiplicity", "Constraints"];

export interface FormFinding {
  /** Repo-relative artifact path. */
  path: string;
  form: PropertiesForm;
  /** 1-based line of the first Properties block, when one exists. */
  line?: number;
  /** Advisory diagnostic for legacy forms (FR-074). */
  diagnostic?: {
    code: "semantic.legacy-properties-form";
    severity: "warning";
    form: "bullet-list" | "free-column-table";
    line: number;
    migration: "typed-table";
  };
}

interface Block {
  kind: "table" | "list" | "sysml-fence";
  line: number;
  header?: string[];
}

function parseHeader(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

/** Find the Properties section (a `## Properties` heading up to the next `##`) and its first block. */
export function classifyProperties(markdown: string): {
  form: PropertiesForm;
  line?: number;
  blocks: Block[];
} {
  const lines = markdown.split("\n");
  let start = -1;
  for (let i = 0; i < lines.length; i += 1) {
    if (/^##\s+Properties\s*$/.test(lines[i] ?? "")) {
      start = i;
      break;
    }
  }
  if (start < 0) return { form: "none", blocks: [] };
  const blocks: Block[] = [];
  let inFence: { line: number; language: string } | undefined;
  for (let i = start + 1; i < lines.length; i += 1) {
    const text = lines[i] ?? "";
    if (/^##\s/.test(text) && !inFence) break;
    if (inFence) {
      if (/^```/.test(text)) inFence = undefined;
      continue;
    }
    const fence = /^```\s*([A-Za-z0-9:_-]*)/.exec(text);
    if (fence) {
      inFence = { line: i + 1, language: fence[1] ?? "" };
      if (inFence.language === "sysml")
        blocks.push({ kind: "sysml-fence", line: i + 1 });
      continue;
    }
    if (/^\s*\|/.test(text)) {
      const previous = blocks[blocks.length - 1];
      if (
        !previous ||
        previous.kind !== "table" ||
        previous.line + blocks.length < i
      ) {
        // A new table starts on a header row followed by a separator row.
        const next = lines[i + 1] ?? "";
        if (/^\s*\|?\s*:?-{3,}/.test(next)) {
          blocks.push({
            kind: "table",
            line: i + 1,
            header: parseHeader(text),
          });
        }
      }
      continue;
    }
    if (/^\s*[-*]\s+\S/.test(text)) {
      const previous = blocks[blocks.length - 1];
      if (!previous || previous.kind !== "list")
        blocks.push({ kind: "list", line: i + 1 });
    }
  }
  const first = blocks[0];
  if (!first) return { form: "none", line: start + 1, blocks };
  if (first.kind === "sysml-fence")
    return { form: "sysml-fence", line: first.line, blocks };
  if (first.kind === "list")
    return { form: "bullet-list", line: first.line, blocks };
  const typed =
    first.header !== undefined &&
    first.header.length === TYPED_HEADER.length &&
    first.header.every((cell, index) => cell === TYPED_HEADER[index]);
  return {
    form: typed ? "typed-table" : "free-column-table",
    line: first.line,
    blocks,
  };
}

/** Classify one artifact and, for a legacy form, attach the FR-074 warning. */
export function classifyArtifact(path: string, markdown: string): FormFinding {
  const { form, line } = classifyProperties(markdown);
  const finding: FormFinding = {
    path,
    form,
    ...(line !== undefined ? { line } : {}),
  };
  if (
    (form === "bullet-list" || form === "free-column-table") &&
    line !== undefined
  ) {
    finding.diagnostic = {
      code: "semantic.legacy-properties-form",
      severity: "warning",
      form,
      line,
      migration: "typed-table",
    };
  }
  return finding;
}

export interface SweepReport {
  package: string;
  version: string;
  generatedAt: string;
  corpus: { repository: string; revision: string }[];
  counts: {
    artifacts: number;
    forms: Record<PropertiesForm, number>;
    legacy: { "bullet-list": number; "free-column-table": number };
  };
  findings: FormFinding[];
}

function* markdownFiles(root: string): Generator<string> {
  for (const entry of readdirSync(root).sort()) {
    if (entry === "node_modules" || entry.startsWith(".")) continue;
    const full = join(root, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) yield* markdownFiles(full);
    else if (entry.endsWith(".md")) yield full;
  }
}

/** Walk one corpus root and classify every Markdown artifact under `spec/`-shaped trees. */
export function sweepCorpus(
  roots: { root: string; repository: string; revision: string }[],
  identity: { package: string; version: string },
  now = new Date(),
): SweepReport {
  const findings: FormFinding[] = [];
  const forms: Record<PropertiesForm, number> = {
    "typed-table": 0,
    "free-column-table": 0,
    "bullet-list": 0,
    "sysml-fence": 0,
    none: 0,
  };
  for (const { root, repository } of roots) {
    for (const file of markdownFiles(root)) {
      const finding = classifyArtifact(
        `${repository}:${relative(root, file)}`,
        readFileSync(file, "utf8"),
      );
      forms[finding.form] += 1;
      findings.push(finding);
    }
  }
  return {
    package: identity.package,
    version: identity.version,
    generatedAt: now.toISOString(),
    corpus: roots.map(({ repository, revision }) => ({ repository, revision })),
    counts: {
      artifacts: findings.length,
      forms,
      legacy: {
        "bullet-list": forms["bullet-list"],
        "free-column-table": forms["free-column-table"],
      },
    },
    findings,
  };
}

/** The migration text the authoring pack shows once per semantic module (FR-074). */
export const LEGACY_MIGRATION_EXAMPLE = [
  "Properties migration (FR-074): the typed table is the authored form.",
  "  before:  | Column | Type | Constraints |",
  "           | id | UUID | PK |",
  "  after:   | Field | Type | Multiplicity | Constraints |",
  "           | id | UUID | 1 | identity |",
  "  Legacy bullet lists and free-column tables validate at warning until the",
  "  module records a sweep report and sets semantic.legacy_forms: error.",
].join("\n");
