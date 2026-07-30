/**
 * Shared deprecation notice for the legacy `quoin plugin` topic.
 *
 * `plugin` was renamed to `module` (FR-017) to free the `plugin` namespace for
 * the oclif sense of installable command packages. The alias is retained for at
 * least one minor cycle; every alias subcommand emits this warning to stderr
 * before forwarding to its `module` counterpart.
 */
export const PLUGIN_DEPRECATION_NOTICE =
  "warning: `quoin plugin` is deprecated and will be removed; use `quoin module` instead.";
