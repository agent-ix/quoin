/**
 * The Properties-form census and the L3 gap (FR-088).
 *
 * The classifier is `classifyArtifact` from `src/semantic/sweep.ts`. A second
 * implementation would drift from the first and the census would then measure
 * a rule nothing enforces, so this module imports rather than reimplements.
 *
 * A legacy form is one advisory `unsupported-representation` finding and never
 * a conformance failure: every measured module declares `legacy_forms:
 * warning`, so failing a document for a form its own module admits would be
 * this measurement inventing a rule.
 *
 * The field-level dimension is recorded `could-not-run` for every document
 * while the toolchain shows no semantic-extraction surface. That is not a pass
 * and not a failure; it is a check that did not happen, and it says so.
 */

import { readFileSync } from "node:fs";

import { classifyArtifact, type PropertiesForm } from "../semantic/sweep.js";

export interface CensusRow {
  readonly path: string;
  readonly form: PropertiesForm | "not-applicable";
  /** Legacy forms raise one advisory finding each. Never a failure. */
  readonly advisory: boolean;
  /** The field-level dimension, while no extraction surface exists. */
  readonly fieldLevel: "could-not-run";
  readonly citation: string;
}

export interface Census {
  /** Stated in documents. The unit travels with the number. */
  readonly unit: "documents";
  readonly total: number;
  readonly byForm: Readonly<Record<string, number>>;
  readonly advisoryFindings: number;
  /** Documents with no `## Properties` heading, kept out of both sides. */
  readonly notApplicable: number;
  readonly fieldLevelCitation: string;
  readonly rows: readonly CensusRow[];
}

export class CensusError extends Error {}

/**
 * Asserts the census population and the enumerated population are one, element
 * by element. Two walks of the same tree diverge silently the moment either
 * one gains a filter, and then two rates are published over two populations
 * that share a name.
 */
export function assertSamePopulation(
  census: readonly string[],
  enumerated: readonly string[],
): void {
  if (census.length !== enumerated.length) {
    throw new CensusError(
      `census walked ${census.length} documents but enumeration counted ${enumerated.length}; ` +
        `the two populations must be one list, not two walks`,
    );
  }
  for (let i = 0; i < census.length; i += 1) {
    if (census[i] !== enumerated[i]) {
      throw new CensusError(
        `census and enumeration diverge at index ${i}: ${census[i]} vs ${enumerated[i]}`,
      );
    }
  }
}

const FIELD_LEVEL_CITATION = "agent-ix/quire-rs#392";

export function propertiesCensus(documents: readonly string[]): Census {
  const rows: CensusRow[] = [];
  const byForm: Record<string, number> = {};
  let advisoryFindings = 0;
  let notApplicable = 0;

  for (const path of documents) {
    let markdown: string;
    try {
      markdown = readFileSync(path, "utf8");
    } catch {
      // An unreadable document is the enumeration's finding, not the census's.
      continue;
    }

    const finding = classifyArtifact(path, markdown);
    const form = finding.form;

    // No `## Properties` heading: the document is outside this rate entirely,
    // and counting it on either side would move the rate without measuring
    // anything.
    const applicable = form !== "none";
    if (!applicable) notApplicable += 1;

    const advisory =
      form === "bullet-list" || form === "free-column-table";
    if (advisory) advisoryFindings += 1;

    const key = applicable ? String(form) : "not-applicable";
    byForm[key] = (byForm[key] ?? 0) + 1;

    rows.push({
      path,
      form: applicable ? form : "not-applicable",
      advisory,
      fieldLevel: "could-not-run",
      citation: FIELD_LEVEL_CITATION,
    });
  }

  return {
    unit: "documents",
    total: rows.length,
    byForm,
    advisoryFindings,
    notApplicable,
    fieldLevelCitation: FIELD_LEVEL_CITATION,
    rows,
  };
}

/**
 * The rate denominator excludes `not-applicable` on both sides. A document
 * with no Properties heading is neither conforming nor legacy, and folding it
 * into either side reports a rate about documents the rule does not reach.
 */
export function conformingShare(census: Census): {
  applicable: number;
  conforming: number;
  share: number | null;
} {
  const applicable = census.total - census.notApplicable;
  const conforming = applicable - census.advisoryFindings;
  return {
    applicable,
    conforming,
    share: applicable === 0 ? null : conforming / applicable,
  };
}
