---
id: FR-018
title: "Publish the v0.1 schema and conformance corpus"
type: FR
relationships:
  - target: ix://agent-ix/quire-contract-ir/StR-003
    type: traces_to
---
# FR-018: Publish the v0.1 schema and conformance corpus

## Description

The repository shall publish a Draft 7 JSON schema, representative valid
packages, targeted invalid packages, expected diagnostics, canonical encodings,
digests, dependency sets, and a reusable conformance runner.

## Inputs

Versioned schema files, fixture manifest, fixture bytes, expected outcomes, and
runner configuration.

## Outputs

Machine-readable per-fixture results with stable diagnostics and a nonzero exit
status for any mismatch.

## Behavior

The published package schema is JSON Schema Draft 7 with identity
`https://agent-ix.github.io/quire-contract-ir/schemas/contract-package-reference-v1.schema.json`.
It describes the complete `ContractPackage<ReferenceBody>` wire representation
for supported schema versions 1.0 and 1.1, closes every object with
`additionalProperties: false`, uses fixed-width numeric bounds, and carries no
implementation-language names. The schema file's lowercase SHA-256 is a corpus
identity input. Schema success never substitutes for semantic validation.

The conformance manifest and fixture payloads are independently validated by
Draft 7 schema identity
`https://agent-ix.github.io/quire-contract-ir/schemas/contract-conformance-manifest-v1.schema.json`.
The conformance schema exposes the named subschemas `#/definitions/manifest`,
`packageInput`, `expressionInput`, `migrationInput`, `coverageInput`, and one
corresponding `*Expectation` subschema for each operation. The runner selects
the input and expectation subschemas from the declared operation before any
semantic conversion; every object forbids unknown fields.
The published schema root is exercised against the actual manifest and negative
manifest mutations, independently of the runner's typed decoding.

The manifest declares corpus identity, exact package-schema path and digest,
exact conformance-schema path and digest, exact inventory path and digest,
canonical profile, tool protocol, and an authored fixture array. Fixture IDs
are unique validated identifiers. Paths are UTF-8, relative to the manifest
directory, contain no empty, `.` or `..` segment, and after symlink resolution
remain below that directory. The manifest itself, package schema, conformance
schema, inventory, and each referenced input, expectation, or canonical-byte
file is at most 16777216 bytes, checked before parsing or digesting; the
manifest contains at most 10000 fixtures. Duplicate IDs, unsafe paths, unknown
fields, malformed inputs or expectations, digest mismatch, unsupported
profiles, or resource-limit breach fail before any fixture executes.
Each fixture carries the lowercase SHA-256 of its input and expectation, and
each referenced canonical-byte record carries its raw-byte SHA-256. The runner
verifies those manifest-bound digests before semantic execution; adjacent
sidecars remain convenient external pin material. The manifest plus every
logical referenced-file read has a 67108864-byte aggregate preload budget, so
repeated paths cannot multiply accepted allocation without bound.

The four closed fixture operations are:

| Operation | Declarative input | Comparable result |
|---|---|---|
| `package` | package JSON, optionally wrapped with authored clause-resolution references and a canonical byte limit, or raw package JSON text for decoder-boundary probes | validity, ordered diagnostics, package/requirement/clause canonical bundle and package dependency union |
| `expression` | declarations, expression, expected type, execution point, clause-root flag | validity, ordered diagnostics, separate declaration/expression canonical outputs and expression dependencies |
| `migration` | reference-body package and explicit target version | validity, ordered diagnostics, migrated package digest, immutable receipt |
| `coverage` | reference-body package and artifact traces | ordered diagnostics and sorted requirement/artifact rows |

Expression fixture syntax covers every FR-013/FR-014 declaration, value type,
literal, reference, access, call, numeric, comparison, Boolean, option,
collection, record, and quantifier variant. It is an unvalidated wire model;
conversion invokes the same public constructors and checker as the Rust API.
Fixture IDs never select constructors, expected results, or special behavior.

Every fixture names one input path, one expectation path, its operation, a
non-empty sorted unique `covers` array, and a non-empty sorted unique `trace_ids` array. The trace
IDs are acceptance-criterion targets derived from the fixture's observed coverage
tokens and operation using the owned `schemas/conformance-trace-map-v1.json`
registry. Every inventory token has an owner; each applicable operation/token
pair has exactly one entry, which may name several relevant criteria. In
particular, reference-body dependencies do not claim typed-expression criteria,
and artifact orphan diagnostics do not claim semantic-reference resolution.
Unmapped operation/token pairs fail. The fixture must declare exactly the sorted unique union of those
targets; arbitrary, missing, or operation-wide substituted targets fail before
execution. Results copy the validated targets unchanged. A trace identifies a
relevant observation, not proof that every conjunct of that criterion passed;
Quire owns criterion identity and static relationships, and the wider test suite
still owns criteria not exercised by this corpus. Coverage tokens are the closed forms
`construct:<registered-tag>`, `diagnostic:<STD-001-code>`,
`obligation:<DefinednessObligationKind>`, `boundary:<registered-boundary>`, and
`operation:<operation>`. The repository
publishes the exact required inventory beside the manifest. The Rust library
exports the sorted fixed-width registries `PUBLIC_CONSTRUCT_TAGS` and
`CONFORMANCE_BOUNDARIES`; operation tokens come from the four-operation enum,
diagnostic tokens from `DiagnosticCode::ALL`, and obligation tokens from all
four `DefinednessObligationKind` values. Construct tags are qualified by
wire namespace, for example `expression.boolean_literal`, `type.boolean`, and
`clause_kind.precondition`, so equal snake-case variant names cannot collide.
The checked-in inventory must equal those four registry projections exactly,
and the union of manifest tokens must equal the inventory; an absent required
token or unknown token is a corpus failure. Every STD-001 diagnostic has a
failing fixture, every public wire construct has a successful fixture, and the
four obligation values have distinct `potentially_undefined` fixtures. The
inventory is derived exactly from `PUBLIC_CONSTRUCT_TAGS`,
`CONFORMANCE_BOUNDARIES`, the four-operation enum, `DiagnosticCode::ALL`, and
the four-obligation enum.

Coverage claims are observations, not trusted fixture declarations.
`operation:` is observed only by selecting that declared operation;
`diagnostic:` and `obligation:` are observed only in the actual structured
diagnostic result; and `construct:` requires semantic success plus the named
wire/result construct. Except for the deliberately shape-invalid wire-depth
probes specified below, a valid minimum, maximum, normalization, revision,
schema, canonical-order, or depth boundary requires semantic success and its
exact structural predicate. An invalid boundary requires both its exact
structural predicate and the owning actual diagnostic. Artifact boundaries are
observed only by the coverage operation's artifact result/diagnostic domain;
a package reference diagnostic cannot claim an artifact-trace boundary. Any
fixture token not observed under these family rules invalidates the manifest.

The closed boundary registry is `source_span.minimum`, `source_span.reversed`,
`revision.current`,
`revision.stale`, `schema.1_0`, `schema.1_1`, `schema.zero_major`,
`schema.unknown_major`, `schema.unregistered_minor`, `integer.minimum`,
`integer.maximum`, `integer.out_of_range`, `rational.normalized`,
`rational.zero_denominator`, `rational.maximum_denominator`, `text.maximum`,
`text.over_maximum`, `collection.declared_maximum`,
`collection.declared_out_of_range`, `collection.minimum`,
`collection.maximum`, `collection.over_maximum`, `expression.depth.maximum`,
`expression.depth.over_maximum`, `expression.nodes.maximum`,
`expression.nodes.over_maximum`, `type.depth.maximum`,
`type.depth.over_maximum`, `semantic.nodes.maximum`,
`semantic.nodes.over_maximum`, `semantic_collection.maximum`,
`semantic_collection.over_maximum`, `canonical.escape_controls`,
`canonical.semantic_set_order`, `canonical.sequence_order`,
`canonical.resource_failure`, `artifact.cross_package`, `artifact.missing`,
`artifact.stale`, `artifact.duplicate`, and `artifact.digest_mismatch`.
The decoder registry also includes `wire.depth.maximum` and
`wire.depth.over_maximum`; raw-text package probes exercise exactly 576 and 577
levels without requiring the manifest decoder to materialize the nested value. The at-limit probe
must reach ordinary package-shape decoding at `document`; the over-limit probe must be rejected by
the pre-decode nesting guard at `document.nesting`. This distinction keeps the wire-depth cliff
observable even though both deliberately shape-invalid probe documents share the
`invalid_wire_format` code.

Expectations contain only fields meaningful for their operation. Valid results
carry no diagnostics; invalid results carry the exact authored-order diagnostic
code, semantic path, optional span, related identities, and optional obligation
kind. Successful canonical results carry exact UTF-8 bytes as a fixture path and
lowercase digest. A package result carries the package output plus separately
sorted requirement and clause outputs; an expression result carries the
declaration environment and typed expression outputs separately. Dependency
identities and coverage rows use their normative structural order. The runner
compares values structurally and bytes exactly; it never parses diagnostic
messages.

The corpus root contains external SHA-256 sidecars for the schema, manifest,
inventory, inputs, expectations, and canonical-byte files. Its README shows
downstream users how to pin the repository commit plus those content digests in
their own evidence; no checked-in file attempts to contain the commit that
contains itself. A downstream pin is documentation, not evidence that
downstream execution occurred. Downstream tools can execute the process runner
without linking the Rust library.
The checked-in generator is owned by this requirement, derives its runner from
the configured Cargo target directory, and has a scratch-directory byte-for-byte
regeneration gate. Frozen outputs establish regression stability and
determinism for this implementation, not independent semantic correctness.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-018-AC-1 | The manifest-schema, exact inventory equality, non-empty trace targets, and positive/negative fixtures prove every registered public construct, STD-001 diagnostic, operation, and boundary token is covered; duplicate, unknown, unsafe-path, oversize, profile/digest-drift, and one-past-limit manifests fail before execution. | Test (TC-018) |
| FR-018-AC-2 | Mutation fixtures independently alter schema validity, diagnostic code/path/order/span/obligation, canonical byte, digest, dependency, migration receipt, and coverage row/reason; each produces the exact mismatch result without message parsing. | Test (TC-018) |
| FR-018-AC-3 | Per-family observation rules reject every unobserved claim, cross-domain artifact claim, stale fixture/canonical digest, aggregate-byte overflow, and disabled corpus lane; raw package probes pin unknown-member rejection and exact/one-past wire depth; the portable generator reproduces the complete checked-in corpus byte-for-byte in scratch space. | Test (TC-018) |

## Dependencies

FR-011 through FR-017 define the normative corpus behavior; FR-019 owns the
stable public registry exports consumed here.
