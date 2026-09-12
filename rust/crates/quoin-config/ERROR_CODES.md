# `quoin-config` error codes

Stable wire strings for `quoin_config::ConfigErrorCode`. **Never renamed, never
reused** — they cross the `quoin-core` subprocess boundary as the non-success
state. `tc_381_004` asserts the catalogue round-trips.

| Code | Variant | Raised when |
|---|---|---|
| `QC001_INVALID_PLUGIN_ID` | `InvalidPluginId` | An id fails `^[a-z][a-z0-9-]*$` or exceeds 64 bytes; also when a non-strict schema is offered for registration. |
| `QC002_INVALID_PACKAGE_NAME` | `InvalidPackageName` | An empty package name. |
| `QC003_UNDERIVABLE_PLUGIN_ID` | `UnderivablePluginId` | A package name reduces to something that is not a legal plugin id. |
| `QC004_DUPLICATE_REGISTRATION` | `DuplicateRegistration` | Reserved for a caller that wants a duplicate to be an error; `PluginSchemaRegistry::register` reports `RegistrationOutcome::Duplicate` instead, matching the oracle. |
| `QC005_UNKNOWN_PLUGIN` | `UnknownPlugin` | A config operation names an id nothing has registered. |
| `QC006_CONFIG_IO` | `ConfigIo` | A config file cannot be read, or exceeds `MAX_CONFIG_FILE_BYTES`. |
| `QC007_CONFIG_PARSE` | `ConfigParse` | A config file is not parsable YAML. |
| `QC008_CONFIG_NOT_A_MAPPING` | `ConfigNotAMapping` | A config file's top-level value is not a mapping. |
| `QC009_CONFIG_SCHEMA` | `ConfigSchema` | A merged config fails schema validation. |
| `QC010_CONFIG_SYMLINK_REFUSED` | `ConfigSymlinkRefused` | A write would follow a symlinked config path. |
| `QC011_CONFIG_WRITE` | `ConfigWrite` | A config file cannot be written or removed. |
| `QC012_CONFIG_SET_PARSE` | `ConfigSetParse` | A complex key's raw value is not JSON. |
| `QC013_UNKNOWN_CONFIG_KEY` | `UnknownConfigKey` | A get/set names a key the schema does not declare. |
| `QC014_INVALID_ORG_NAME` | `InvalidOrgName` | An organization name is empty or whitespace-only; also `ResolvedOrg::require` when nothing resolved. |
| `QC015_CONFIG_LOCK_TIMEOUT` | `ConfigLockTimeout` | The advisory `<path>.lock` guarding a config read-merge-write could not be claimed within the budget; ix-cli-core reports the same condition as `ConfigLockTimeoutError`. |
| `QC016_SCHEMA_NOT_STRICT` | `SchemaNotStrict` | A plugin schema offered for registration is not strict. |
