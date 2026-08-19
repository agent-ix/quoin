/**
 * Reading a bundle's declared-vocabulary claims (FR-037).
 *
 * **On quoin reading documents at all.** quire is the parser, and quoin does not
 * reimplement it — every structural question about a document goes to
 * `quire validate` / `extract` / `coverage`. What is read here is narrower: the
 * leading `---` block, as YAML, plus the raw body text.
 *
 * The alternative was one `quire extract` subprocess per document. `extract`
 * takes a single `<DOC>` and a `--module`, so a bundle sweep is N spawns to
 * answer a question about a handful of frontmatter keys. That is the wrong trade
 * for a check meant to run in CI.
 *
 * The better fix is upstream and is filed: if the FR-059 diagnostic classified
 * each value as owned / excused / unowned, quoin would need no reader at all.
 * Until then this stays deliberately dumb — no archetype resolution, no schema
 * validation, no link walking. If the frontmatter does not parse, the document
 * contributes nothing and says so.
 */

import { readdirSync, readFileSync } from "node:fs";
import { join, relative, sep } from "node:path";

import { parse as parseYaml } from "yaml";

import type { VocabularyDeclaration } from "./declarations.js";
import type { DocumentClaims } from "./assess.js";

/** `---\n…\n---\n` at the head of the file, plus everything after it. */
const FRONTMATTER = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/;

export interface BundleRead {
  documents: DocumentClaims[];
  /** Documents whose frontmatter could not be parsed, and why. */
  unreadable: Array<{ path: string; reason: string }>;
}

/**
 * Read every markdown document under `bundleRoot` for one declaration's fields.
 *
 * Both the claim field and the justified-absence field accept a scalar or a
 * list, because `quality_attribute: security` and
 * `quality_attributes_not_applicable: [safety, compliance]` are both ordinary
 * authoring — the same two shapes the engine accepts, for the same reason.
 */
export function readBundleClaims(
  bundleRoot: string,
  declaration: VocabularyDeclaration,
): BundleRead {
  const documents: DocumentClaims[] = [];
  const unreadable: Array<{ path: string; reason: string }> = [];

  for (const path of markdownUnder(bundleRoot)) {
    const rel = relative(bundleRoot, path).split(sep).join("/");
    let raw: string;
    try {
      raw = readFileSync(path, "utf8");
    } catch (cause) {
      unreadable.push({ path: rel, reason: reasonOf(cause) });
      continue;
    }
    const match = FRONTMATTER.exec(raw);
    // No frontmatter is not an error here: an index or a README claims nothing.
    if (!match) continue;

    let frontmatter: Record<string, unknown>;
    try {
      frontmatter = (parseYaml(match[1]) ?? {}) as Record<string, unknown>;
    } catch (cause) {
      // Reported, not skipped silently: a document whose frontmatter does not
      // parse may be the one carrying the exclusion, and dropping it would turn
      // a broken excuse into a clean bundle.
      unreadable.push({ path: rel, reason: reasonOf(cause) });
      continue;
    }
    if (!frontmatter || typeof frontmatter !== "object") continue;

    const claims = stringsAt(frontmatter[declaration.field]);
    const excuses = declaration.justifiedAbsenceField
      ? stringsAt(frontmatter[declaration.justifiedAbsenceField])
      : [];
    if (claims.length === 0 && excuses.length === 0) continue;

    documents.push({ path: rel, claims, excuses, body: match[2] });
  }

  return { documents, unreadable };
}

function markdownUnder(root: string): string[] {
  let entries: string[];
  try {
    entries = readdirSync(root, { recursive: true, encoding: "utf8" });
  } catch {
    // An absent bundle root reads as an empty bundle. The command reports the
    // root it looked in, so this is legible rather than mysterious.
    return [];
  }
  return entries
    .filter((entry) => entry.endsWith(".md"))
    .map((entry) => join(root, entry))
    .sort();
}

/** A frontmatter value as a list of strings, from either the scalar or list form. */
function stringsAt(value: unknown): string[] {
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) {
    return value.filter((v): v is string => typeof v === "string");
  }
  return [];
}

function reasonOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
