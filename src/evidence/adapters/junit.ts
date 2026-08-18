import type { RunEntry } from "../types.js";
import {
  AdapterError,
  type AdapterResult,
  type EvidenceAdapter,
} from "./types.js";

/**
 * The `<testcase>` attributes and child elements this adapter reads.
 *
 * Deliberately a hand-written scan rather than a full XML parse. The subset is
 * fixed and small, the input is machine-written, and a dependency-free reader
 * keeps `quoin evidence record` free of a parser whose failure modes would
 * become quoin's.
 */
const TESTCASE = /<testcase\b([^>]*?)(\/>|>([\s\S]*?)<\/testcase\s*>)/g;
const ATTRIBUTE = /([\w:.-]+)\s*=\s*"([^"]*)"/g;

function attributes(fragment: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const match of fragment.matchAll(ATTRIBUTE)) {
    out.set(match[1], decode(match[2]));
  }
  return out;
}

/** The five predefined XML entities. Attribute values carry no others. */
function decode(text: string): string {
  return text
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&apos;", "'")
    .replaceAll("&amp;", "&");
}

/**
 * `classname` + `name` → the qualified name the symbol extractor produces.
 *
 * This is the join, and it is the whole job. A tool's own test name is not a
 * symbol identity: `quire` records `tests::tc001`, JUnit records
 * `classname="tests" name="tc001"`. Anything that skipped this would write a
 * store whose entries match no declared symbol, and every obligation would
 * read as unmatched while looking recorded.
 *
 * Separator is `::` because that is what the Rust and Python extractors emit
 * for `container::member`. A `classname` that already contains the name — some
 * runners repeat it — is not doubled.
 */
export function qualifiedName(classname: string, name: string): string {
  const container = classname.trim();
  const member = name.trim();
  if (container === "") return member;
  if (member === "") return container;
  if (container === member) return member;
  if (container.endsWith(`::${member}`) || container.endsWith(`.${member}`)) {
    return container.replaceAll(".", "::");
  }
  return `${container.replaceAll(".", "::")}::${member}`;
}

/** Trace ids named in a `<testcase>`'s own properties, if any. */
const PROPERTY = /<property\b([^>]*)\/?>/g;

function traceIds(body: string): string[] | undefined {
  const ids: string[] = [];
  for (const match of body.matchAll(PROPERTY)) {
    const attrs = attributes(match[1]);
    const key = attrs.get("name");
    const value = attrs.get("value");
    if (key === undefined || value === undefined) continue;
    if (key !== "trace" && key !== "traceIds" && key !== "trace_ids") continue;
    for (const id of value.split(/[,\s]+/)) {
      if (id !== "") ids.push(id);
    }
  }
  return ids.length > 0 ? ids : undefined;
}

/**
 * A `<testcase>`'s outcome, read from its children.
 *
 * Order matters and is not arbitrary: `error` outranks `failure` outranks
 * `skipped`. A case carrying both an error and a failure is reported as the
 * more severe, because the milder reading is the one that would let a broken
 * run look merely red.
 */
function outcome(body: string): RunEntry["outcome"] {
  if (/<error\b/.test(body)) return "error";
  if (/<failure\b/.test(body)) return "fail";
  if (/<skipped\b/.test(body)) return "skip";
  return "pass";
}

export const junitAdapter: EvidenceAdapter = {
  name: "junit",
  summary: "JUnit XML (<testsuite>/<testcase>), as emitted by most runners.",
  tools: ["junit", "pytest", "jest", "vitest", "gotestsum", "surefire"],
  parse(raw: string): AdapterResult {
    const entries: RunEntry[] = [];
    for (const match of raw.matchAll(TESTCASE)) {
      const attrs = attributes(match[1]);
      const body = match[3] ?? "";
      const symbol = qualifiedName(
        attrs.get("classname") ?? "",
        attrs.get("name") ?? "",
      );
      if (symbol === "") continue;
      const ids = traceIds(body);
      entries.push({
        symbol,
        outcome: outcome(body),
        ...(ids === undefined ? {} : { traceIds: ids }),
      });
    }
    if (entries.length === 0) {
      throw new AdapterError(
        "junit",
        "no <testcase> elements found — is this JUnit XML?",
      );
    }
    // No evidenceKind: unit, integration and e2e suites all emit JUnit, so the
    // format does not say which this was. The suite registry does.
    return { entries };
  },
};
