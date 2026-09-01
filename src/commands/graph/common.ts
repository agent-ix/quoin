import { Flags } from "@oclif/core";

import type { GraphAnalysis } from "../../graph-analysis/index.js";
import {
  loadGraphAnalysisInput,
  renderGraphAnalysis,
  renderGraphAnalysisJson,
} from "../../graph-analysis/index.js";

export const graphInputFlags = {
  repo: Flags.string({
    description: "Repository root for retained Quoin state.",
    default: ".",
  }),
  export: Flags.string({
    description: "Existing Quire assurance-v1 JSON artifact.",
    required: true,
  }),
  premises: Flags.string({
    description: "Accepted assurance format/module/schema premises JSON.",
    required: true,
  }),
  audit: Flags.string({
    description: "Source-bound FR-032 audit-envelope JSON.",
    required: true,
  }),
  json: Flags.boolean({ description: "Emit canonical JSON." }),
};

export function loadGraphFlags(flags: {
  repo: string;
  export: string;
  premises: string;
  audit: string;
}) {
  return loadGraphAnalysisInput({
    repo: flags.repo,
    exportPath: flags.export,
    premisesPath: flags.premises,
    auditPath: flags.audit,
  });
}

export function graphOutput(report: GraphAnalysis, json: boolean): string {
  return (
    json ? renderGraphAnalysisJson(report) : renderGraphAnalysis(report)
  ).trimEnd();
}
