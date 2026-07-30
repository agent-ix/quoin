import { registerPluginSchema } from "@agent-ix/ix-cli-core";
import { z } from "zod";

/**
 * quoin's persistent configuration (FR-027).
 *
 * Strict, as `registerPluginSchema` requires — a non-strict schema is rejected
 * at registration, and strictness is what makes `config set` reject an unknown
 * key instead of silently writing it.
 *
 * Every leaf is optional rather than defaulted: an absent `org` must stay
 * absent, because "nobody said" is a distinct outcome from any value quoin
 * could pick (FR-025).
 */
export const QuoinConfigSchema = z
  .object({
    org: z.string().min(1).optional(),
  })
  .strict();

export type QuoinConfig = z.infer<typeof QuoinConfigSchema>;

/** Config/secrets namespace quoin registers under. */
export const QUOIN_PLUGIN_ID = "quoin";

/**
 * Map of config key → environment variable, applied by `ConfigService` as a
 * layer over the file. Keeping `QUOIN_ORG` here rather than reading it directly
 * means one precedence rule lives in one place.
 */
export const QUOIN_ENV_BINDINGS = { org: "QUOIN_ORG" } as const;

/**
 * The `ixSchema` convention (ix-cli-core FR-014): a host walking its
 * oclif-loaded plugins reads this named export and registers it, so
 * `ix config set quoin org <name>` resolves against the same schema and file
 * that quoin itself reads.
 */
export const ixSchema = {
  id: QUOIN_PLUGIN_ID,
  config: QuoinConfigSchema,
  env: QUOIN_ENV_BINDINGS,
};

/**
 * Register quoin's own schema in-process.
 *
 * Under `ix`, the host's init hook registers every loaded plugin's `ixSchema`.
 * Running standalone there is no host, so `quoin config` registers itself
 * before delegating — otherwise the shared handlers raise `UnknownPluginError`
 * for an id nothing has declared. Registration is idempotent: a duplicate
 * returns a non-throwing failure and keeps the first entry.
 */
export function registerQuoinSchema(): void {
  registerPluginSchema("@agent-ix/quoin", ixSchema);
}
