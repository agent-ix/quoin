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
  const { documents, unreadable } = readBundleFrontmatter(bundleRoot);
  return { documents: claimsFor(documents, declaration), unreadable };
}

/**
 * Project already-read documents onto one declaration.
 *
 * Split out from {@link readBundleClaims} so a caller with several declarations
 * walks the bundle **once**. The combined form re-read every file per
 * declaration, which NFR-011-M-2 budgets at one pass per invocation.
 */
export function claimsFor(
  read: BundleDocument[],
  declaration: VocabularyDeclaration,
): DocumentClaims[] {
  const documents: DocumentClaims[] = [];
  for (const doc of read) {
    const { path: rel, frontmatter } = doc;
    const claims = stringsAt(frontmatter[declaration.field]);
    const excuses = declaration.justifiedAbsenceField
      ? stringsAt(frontmatter[declaration.justifiedAbsenceField])
      : [];
    if (claims.length === 0 && excuses.length === 0) continue;

    documents.push({ path: rel, claims, excuses, body: doc.body });
  }
  return documents;
}

/** One document's frontmatter and body, as read from the bundle. */
export interface BundleDocument {
  /** Path, relative to the bundle root. */
  path: string;
  frontmatter: Record<string, unknown>;
  body: string;
}

export interface FrontmatterRead {
  documents: BundleDocument[];
  unreadable: Array<{ path: string; reason: string }>;
}

/** Instrumentation emitted by the owned bundle reader (NFR-011). */
export type BundleReadEvent =
  { kind: "pass"; root: string } | { kind: "document"; path: string };

export type BundleReadObserver = (event: BundleReadEvent) => void;

/**
 * Every document under `bundleRoot` that carries parseable frontmatter.
 *
 * One pass and one reader, shared by the completeness sweep (FR-037) and the
 * assurance view (FR-040). Two readers over the same files would drift, and the
 * second would be written by whoever needed a field the first did not expose —
 * which is how a repository ends up with two answers to "what does this
 * document declare".
 */
export function readBundleFrontmatter(
  bundleRoot: string,
  observe?: BundleReadObserver,
): FrontmatterRead {
  const documents: BundleDocument[] = [];
  const unreadable: Array<{ path: string; reason: string }> = [];

  observe?.({ kind: "pass", root: bundleRoot });
  for (const path of markdownUnder(bundleRoot)) {
    const rel = relative(bundleRoot, path).split(sep).join("/");
    observe?.({ kind: "document", path: rel });
    let raw: string;
    try {
      raw = readFileSync(path, "utf8");
    } catch (cause) {
      unreadable.push({ path: rel, reason: reasonOf(cause) });
      continue;
    }
    const match = FRONTMATTER.exec(raw);
    // No frontmatter is not an error: an index or a README declares nothing.
    if (!match) continue;

    let frontmatter: Record<string, unknown>;
    try {
      frontmatter = (parseYaml(match[1]) ?? {}) as Record<string, unknown>;
    } catch (cause) {
      // Reported, not skipped silently: a document whose frontmatter does not
      // parse may be the one carrying the exclusion or the edge, and dropping it
      // would turn a broken declaration into a clean bundle.
      unreadable.push({ path: rel, reason: reasonOf(cause) });
      continue;
    }
    if (!frontmatter || typeof frontmatter !== "object") continue;
    documents.push({ path: rel, frontmatter, body: match[2] });
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
