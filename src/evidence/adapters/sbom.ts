import type { RunEntry } from "../types.js";
import {
  AdapterError,
  type AdapterResult,
  type EvidenceAdapter,
} from "./types.js";

/**
 * CycloneDX and SPDX inventories → run entries, one per component (FR-041).
 *
 * **What an SBOM leaf in an assurance case contains, and why that settles the
 * record shape.** `agent-ix/quoin#116` deferred this until FR-040 existed,
 * because the question was whether a supply-chain obligation is discharged by
 * an SBOM's *presence* or by its *contents* — and those imply different records.
 *
 * FR-040 answered it. A GSN evidence leaf renders one line per obligation:
 * statement, supported-or-open, and the auditor's reason. It does not render
 * component lists, and could not usefully — a case is reviewed by a person, and
 * a thousand-row inventory in an argument is not an argument.
 *
 * So the claim an SBOM supports is **"a complete inventory was produced at this
 * commit"**, which is a run record. Contents-level judgement — *component X at
 * version Y has advisory Z* — is finding-shaped and already lands through
 * `cargo-audit` and SARIF (FR-034). There is no third record type here.
 *
 * **One entry per component, deliberately.** The alternative — a single entry
 * with the count in `score` — makes an empty SBOM indistinguishable from a
 * healthy one without a new check. As entries, an inventory listing nothing
 * produces no entries, and `vacuous-evidence` reports it with no new machinery:
 * a tool that ran and found nothing is exactly what that check exists to name.
 */
export function parseSbom(raw: string): AdapterResult {
  let document: unknown;
  try {
    document = JSON.parse(raw);
  } catch (cause) {
    throw new AdapterError(
      "sbom",
      `not JSON: ${cause instanceof Error ? cause.message : String(cause)}. ` +
        `CycloneDX and SPDX both have JSON serialisations; the tag-value SPDX ` +
        `form is not read here.`,
    );
  }
  if (!document || typeof document !== "object") {
    throw new AdapterError("sbom", "expected a JSON object");
  }
  const root = document as Record<string, unknown>;

  // No `tool` here: run-shaped adapters do not report one, and the record takes
  // it from `--tool` at record time. Reading the generating tool out of the
  // document would be useful provenance and is not worth widening a contract
  // every other adapter shares.
  return {
    entries: cycloneDxEntries(root) ?? spdxEntries(root) ?? unrecognised(root),
  };
}

/** CycloneDX: `bomFormat: "CycloneDX"` with a `components` array. */
function cycloneDxEntries(root: Record<string, unknown>): RunEntry[] | null {
  if (String(root.bomFormat ?? "") !== "CycloneDX") return null;
  const components = Array.isArray(root.components) ? root.components : [];
  return components
    .map((raw) => {
      const c = raw as Record<string, unknown>;
      // `purl` is the stable identity when present; `name@version` otherwise.
      // Neither is invented: a component with no name is a malformed entry and
      // is dropped rather than given one, because a fabricated symbol would
      // bind to nothing and inflate the inventory count that proves the SBOM
      // is not vacuous.
      const purl = typeof c.purl === "string" ? c.purl : null;
      const name = typeof c.name === "string" ? c.name : null;
      if (!purl && !name) return null;
      const version = typeof c.version === "string" ? c.version : null;
      return {
        symbol: purl ?? (version ? `${name}@${version}` : (name as string)),
        outcome: "pass" as const,
      };
    })
    .filter((e): e is RunEntry => e !== null);
}

/** SPDX JSON: `spdxVersion` with a `packages` array. */
function spdxEntries(root: Record<string, unknown>): RunEntry[] | null {
  if (typeof root.spdxVersion !== "string") return null;
  const packages = Array.isArray(root.packages) ? root.packages : [];
  return packages
    .map((raw) => {
      const p = raw as Record<string, unknown>;
      const refs = Array.isArray(p.externalRefs) ? p.externalRefs : [];
      const purl = refs
        .map((r) => r as Record<string, unknown>)
        .find((r) => String(r.referenceType ?? "") === "purl");
      const locator =
        purl && typeof purl.referenceLocator === "string"
          ? purl.referenceLocator
          : null;
      const name = typeof p.name === "string" ? p.name : null;
      if (!locator && !name) return null;
      const version = typeof p.versionInfo === "string" ? p.versionInfo : null;
      return {
        symbol: locator ?? (version ? `${name}@${version}` : (name as string)),
        outcome: "pass" as const,
      };
    })
    .filter((e): e is RunEntry => e !== null);
}

/**
 * Neither format recognised.
 *
 * Rejected rather than returning zero entries. Zero entries means "the SBOM
 * listed nothing", which `vacuous-evidence` reports as a real finding about the
 * consumer's build — and a file this adapter simply could not read must not
 * masquerade as that.
 */
function unrecognised(root: Record<string, unknown>): never {
  const keys = Object.keys(root).slice(0, 6).join(", ");
  throw new AdapterError(
    "sbom",
    `neither a CycloneDX (\`bomFormat\`) nor an SPDX (\`spdxVersion\`) document — ` +
      `top-level keys: ${keys || "(none)"}`,
  );
}

/**
 * `evidenceKind` is left unset, like every other adapter shipped here.
 *
 * The catalog gives `sca-sbom` an evidence kind and the suite registry declares
 * one per suite; asserting a fourth copy from the format would be the drift
 * agent-ix/quoin#114 exists to prevent.
 */
export const sbomAdapter: EvidenceAdapter = {
  name: "sbom",
  summary:
    "CycloneDX or SPDX JSON — one entry per component, so an empty inventory reads as vacuous.",
  tools: ["cyclonedx", "spdx", "syft", "cdxgen", "cargo-cyclonedx"],
  parse: parseSbom,
};
