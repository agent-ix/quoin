import type { Finding } from "../types.js";
import { AdapterError } from "./types.js";
import type { FindingResult } from "./sarif.js";

/**
 * `<name>: OK` / `<name>: FAIL — <reason> (<AC-ID>, …)`
 *
 * The convention `quire-rs/scripts/audits/README.md` states and every script
 * there follows: *"All scripts SHALL exit 0 on success and non-zero on
 * violation, with descriptive stderr output."*
 */
const LINE = /^([A-Za-z0-9._-]+):\s*(OK|FAIL)\b\s*(?:[—-]\s*)?(.*)$/;

/**
 * Acceptance-criterion ids named in a trailing parenthetical.
 *
 * Trailing punctuation is allowed after the closing paren because the real
 * scripts write one: `check_no_schemars: FAIL — 'schemars' present in
 * Cargo.lock (FR-003-AC-4).` An anchor demanding end-of-line found nothing,
 * and the finding lost the very criterion that makes it a discharge rather
 * than a complaint.
 */
const TRACE_IDS = /\(([^)]*)\)[.\s]*$/;
const AC_ID = /\b((?:FR|NFR|StR|US)-\d+-AC-\d+|TC-\d+)\b/g;

function traceIdsOf(message: string): string[] | undefined {
  const tail = TRACE_IDS.exec(message);
  if (!tail) return undefined;
  const ids = [...tail[1].matchAll(AC_ID)].map((m) => m[1]);
  return ids.length > 0 ? ids : undefined;
}

/**
 * Architecture-conformance audit scripts → findings.
 *
 * **Why this is the format worth reading for architecture conformance.**
 * Specs and ADRs declare boundaries — "engine/surfaces split", "no solver dep
 * in core", "thin CLI" — that rot silently unless something enforces them. The
 * audit-script pattern is the worked prior art (`quire-rs/scripts/audits/`, a
 * README mapping each script to the criteria it audits), and this reads its
 * output without asking anyone to change it.
 *
 * **The join is literal here, which it is not for most scanners.** FR-034
 * described a scanner rule id ⇄ obligation id as *"a verification method, not a
 * generic scan"*. An audit script already prints the criterion in its failure
 * line — `check_no_schemars: FAIL — 'schemars' present (FR-003-AC-4)` — so the
 * obligation it discharges is stated by the tool rather than mapped after the
 * fact.
 *
 * **A script reporting `OK` is a rule that ran and passed**, which is why
 * `rulesEvaluated` counts every line of either kind. A file with no recognised
 * line means no audit ran, and that is vacuous (FR-034): the check found
 * nothing because it looked for nothing.
 */
export function parseAuditScript(raw: string): FindingResult {
  const findings: Finding[] = [];
  let evaluated = 0;
  for (const line of raw.split("\n")) {
    const match = LINE.exec(line.trim());
    if (!match) continue;
    const [, name, verdict, rest] = match;
    evaluated += 1;
    if (verdict === "OK") continue;
    const message = rest.trim();
    const ids = traceIdsOf(message);
    findings.push({
      ruleId: name,
      severity: "violation",
      ...(message === "" ? {} : { message }),
      ...(ids === undefined ? {} : { traceIds: ids }),
    });
  }
  if (evaluated === 0) {
    throw new AdapterError(
      "audit-script",
      "no `<name>: OK` or `<name>: FAIL` line found — is this audit-script output?",
    );
  }
  return { findings, tool: "audit-script", rulesEvaluated: evaluated };
}
