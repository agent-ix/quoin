export { main, isVersionRequest } from "./cli.js";
export { packageVersion, resolveVersion } from "./version.js";
export {
  loadCatalog,
  defaultModuleRoots,
  filamentModulesDir,
  findCatalogEntry,
} from "./catalog.js";
export {
  ensureDefaultModules,
  defaultModulesManifest,
  installModule,
  listModules,
  removeModule,
  type InstalledModule,
} from "./core/modules.js";
// The seven integrity symbols this barrel published — `IntegrityError`,
// `assertDigest`, `blake3Hex`, `canonicalBytes`, `canonicalizeJcs`,
// `digestValue`, `parseStrictJson` — are gone at quoin#504, by owner ruling:
// no TypeScript is retained for the sole purpose of keeping an export alive.
// They are decided by `quoin-store` now, and a consumer that wanted them is a
// consumer with a porting ticket of its own rather than a reason to keep a
// second implementation. Nothing in the ecosystem imported them.
export {
  CHANGE_ASSURANCE_SCHEMA_NAMES,
  changeAssuranceSchemaPath,
  readChangeAssuranceSchema,
  type ChangeAssuranceSchemaName,
} from "./core/change-assurance-schemas.js";
export {
  createAuthoringPack,
  formatAuthoringPack,
  parseTypeList,
} from "./write.js";
export { QuoinCommand } from "./base.js";
export { FlowCommand } from "./flow-command.js";
export {
  resolveOrg,
  originOrg,
  unresolvedOrgMessage,
  type OrgSource,
  type ResolvedOrg,
} from "./core/org.js";
// `ixSchema` is the named export a host's init hook looks for (ix-cli-core
// FR-014); it must stay reachable from the package main.
export {
  ixSchema,
  QuoinConfigSchema,
  QUOIN_PLUGIN_ID,
  QUOIN_ENV_BINDINGS,
  type QuoinConfig,
} from "./config-schema.js";
