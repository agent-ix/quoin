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
export * from "./measurement/index.js";
export * from "./change-assurance/index.js";
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
