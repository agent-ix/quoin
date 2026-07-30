import { existsSync, statSync } from "node:fs";
import { resolve } from "node:path";

import {
  type SpecCatalog,
  type SpecCatalogEntry,
  findCatalogEntry,
} from "./catalog.js";
import { UNRESOLVED_ORG_MESSAGE, type OrgSource, resolveOrg } from "./org.js";

/** How each org source is named in the rendered pack. */
const ORG_SOURCE_LABEL: Record<OrgSource, string> = {
  flag: "--org",
  env: "QUOIN_ORG",
  config: "stored config",
  git: "git remote",
  none: "nothing",
};

export interface AuthoringContract {
  name: string;
  kind: "artifact" | "object";
  moduleName: string;
  moduleRoot: string;
  schemaPath?: string;
  skeletonPath?: string;
}

export interface AuthoringPack {
  repoRoot: string;
  /** Authoring organization, absent when no source yielded one (FR-025). */
  org?: string;
  orgSource: OrgSource;
  types: AuthoringContract[];
  validation: {
    command: string;
    scope: string;
    globs: string[];
  };
}

export function parseTypeList(value: string | string[] | undefined): string[] {
  const values = Array.isArray(value) ? value : value ? [value] : [];
  return values
    .flatMap((item) => item.split(","))
    .map((item) => item.trim())
    .filter(Boolean);
}

export function createAuthoringPack(
  catalog: SpecCatalog,
  repoDir: string,
  typeNames: string[],
  options: { org?: string } = {},
): AuthoringPack {
  if (typeNames.length === 0) {
    throw new Error("write requires --types <type[,type...]>");
  }
  const repoRoot = resolve(repoDir);
  if (!existsSync(repoRoot) || !statSync(repoRoot).isDirectory()) {
    throw new Error(`write repo_dir is not a directory: ${repoDir}`);
  }

  const types = typeNames.map((name) => {
    const entry = findCatalogEntry(catalog, name);
    if (!entry) {
      throw new Error(
        `catalog type not found: ${name}\nAvailable types: ${catalog.entries
          .map((candidate) => candidate.name)
          .sort((a, b) => a.localeCompare(b))
          .join(", ")}`,
      );
    }
    return toAuthoringContract(entry);
  });

  const { org, source } = resolveOrg(repoRoot, { flag: options.org });

  return {
    repoRoot,
    ...(org ? { org } : {}),
    orgSource: source,
    types,
    validation: {
      command: `quire validate --scope ${shellQuote(repoRoot)} "spec/**/*.md"`,
      scope: repoRoot,
      globs: ["spec/**/*.md"],
    },
  };
}

export function formatAuthoringPack(pack: AuthoringPack): string {
  const lines = [
    "quoin write",
    "",
    `Repo: ${pack.repoRoot}`,
    pack.org
      ? `Org: ${pack.org} (from ${ORG_SOURCE_LABEL[pack.orgSource]})`
      : `Org: unresolved — ${UNRESOLVED_ORG_MESSAGE}`,
    "",
    "Authoring contracts:",
  ];
  for (const type of pack.types) {
    lines.push(`- ${type.name} (${type.kind})`);
    lines.push(`  module: ${type.moduleName}`);
    lines.push(`  module_root: ${type.moduleRoot}`);
    if (type.skeletonPath) lines.push(`  skeleton: ${type.skeletonPath}`);
    if (type.schemaPath) lines.push(`  schema: ${type.schemaPath}`);
    if (!type.skeletonPath && !type.schemaPath) {
      lines.push("  contract: manifest only");
    }
  }
  lines.push("");
  lines.push(
    "Author files directly in the repo, then validate changed spec files.",
  );
  lines.push(`Validate scope: ${pack.validation.scope}`);
  lines.push(`Validate command: ${pack.validation.command}`);
  return lines.join("\n");
}

function toAuthoringContract(entry: SpecCatalogEntry): AuthoringContract {
  return {
    name: entry.name,
    kind: entry.kind,
    moduleName: entry.moduleName,
    moduleRoot: entry.moduleRoot,
    ...(entry.schemaPath ? { schemaPath: entry.schemaPath } : {}),
    ...(entry.skeletonPath ? { skeletonPath: entry.skeletonPath } : {}),
  };
}

function shellQuote(value: string): string {
  if (/^[A-Za-z0-9_./:-]+$/.test(value)) return value;
  return `'${value.replaceAll("'", "'\\''")}'`;
}
