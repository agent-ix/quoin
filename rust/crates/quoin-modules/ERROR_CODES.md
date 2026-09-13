# `quoin-modules` error codes

Stable wire strings for `quoin_modules::ModulesErrorCode`. **Never renamed,
never reused.** `tc_381_202` asserts the catalogue round-trips.

| Code | Variant | Raised when |
|---|---|---|
| `QM001_INVALID_SOURCE` | `InvalidSourceField`, `UnknownSourceType` | A source descriptor field is empty, begins with `-`, or names an unknown type. |
| `QM002_UNSUPPORTED_SOURCE` | `UnsupportedSource` | An `npm` or `url` source; structurally valid, not resolvable. |
| `QM003_UNPARSABLE_SOURCE_ARG` | `UnparsableSourceArg` | A CLI install argument carries a prefix with no payload. |
| `QM004_INVALID_MANIFEST` | `InvalidManifest` | A default-set manifest has the wrong `schemaVersion`, a malformed entry, or too many entries. |
| `QM005_INVALID_MODULE_NAME` | `InvalidModuleName` | A module name is empty, `.`/`..`, or carries a path separator. |
| `QM006_PATH_SOURCE_NOT_FOUND` | `PathSourceNotFound` | A `path:` source does not exist. |
| `QM007_MANIFEST_NOT_FOUND` | `ManifestNotFound` | Neither manifest layout has a `manifest.yaml`. |
| `QM008_MANIFEST_HAS_NO_NAME` | `ManifestHasNoName` | `name` is absent, not a string, empty, or not a legal module name. |
| `QM009_MANIFEST_UNREADABLE` | `ManifestUnreadable` | A `manifest.yaml` cannot be read or parsed, or exceeds its bound. |
| `QM010_REGISTRY_UNREADABLE` | `RegistryUnreadable` | The install registry cannot be read, exceeds its bound, or is malformed JSON. |
| `QM011_REGISTRY_UNWRITABLE` | `RegistryUnwritable` | The install registry cannot be written. |
| `QM012_GIT_TRANSPORT` | `GitTransport` | A remote cannot be reached, or the protocol fails. |
| `QM013_GIT_REVISION_NOT_FOUND` | `GitRevisionNotFound` | A tag, branch or sha is absent from the fetched repository — including an orphaned pin after a history rebuild (quoin#308). |
| `QM014_GIT_OBJECT_STORE` | `GitObjectStore` | An object-store read fails, or a requested subdirectory is absent from the resolved tree. |
| `QM015_GIT_TIMEOUT` | `GitTimeout` | A fetch exceeds `GitLimits::fetch_budget`; the interrupted transport's own message is carried in `detail` rather than discarded. |
| `QM016_RESOURCE_BOUND_EXCEEDED` | `ResourceBoundExceeded` | An extraction exceeds the byte, file-count or tree-depth ceiling. |
| `QM017_UNSAFE_TREE_PATH` | `UnsafeTreePath` | A tree entry or entry subpath escapes its destination directory. |
| `QM018_MATERIALIZE_FAILED` | `MaterializeFailed` | Copying a module into its target directory fails. |
| `QM019_MODULE_NOT_INSTALLED` | `ModuleNotInstalled` | A named module is not in the registry. |
| `QM020_SEMANTIC_CONTRACT_VIOLATION` | `SemanticContractViolation` | The semantic gate rejects a module; carries the `RollbackOutcome`. |
| `QM021_FETCH_DEADLINE_UNAVAILABLE` | `FetchDeadlineUnavailable` | The watchdog thread enforcing `GitLimits::fetch_budget` cannot be spawned, so the blocking fetch would have no wall-clock bound at all. |
