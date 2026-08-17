/**
 * The evidence auditor (FR-032) — suspect links, stale evidence, vacuity.
 *
 * Reads the store and reports. Runs nothing (ADR-0011 invariant 1): an auditor
 * that could re-run a suite could also make a finding disappear by re-running it.
 */

export {
  audit,
  delta,
  ratchet,
  type AuditInput,
  type AuditReport,
  type Finding,
  type Severity,
} from "./audit.js";
