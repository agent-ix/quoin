---
type: index
title: "Functional Requirements"
description: "Index of functional requirements (FR) for quoin."
---

# Functional Requirements

## Contents

- [FR-001: Parse the command line into command, flags, and positionals](./FR-001-parse-command-line.md)
- [FR-002: Report the installed package version](./FR-002-print-package-version.md)
- [FR-003: Print usage and command-scoped help](./FR-003-print-usage-and-help.md)
- [FR-004: Resolve the config root and configure the runtime context](./FR-004-resolve-config-root.md)
- [FR-005: Reject unknown commands and subcommands](./FR-005-reject-unknown-commands.md)
- [FR-006: Locate a module root from a candidate path](./FR-006-locate-module-roots.md)
- [FR-007: Assemble module roots in a defined order](./FR-007-assemble-module-roots.md)
- [FR-008: Deduplicate module roots and names first-wins](./FR-008-deduplicate-modules.md)
- [FR-009: Read each module manifest into catalog entries](./FR-009-read-module-manifest.md)
- [FR-010: Look up catalog types case-insensitively](./FR-010-case-insensitive-type-lookup.md)
- [FR-011: List the catalog and show a single type](./FR-011-list-and-show-catalog.md)
- [FR-012: Detect duplicate type definitions and validate the catalog](./FR-012-detect-duplicate-types.md)
- [FR-013: Resolve the requested types for an authoring pack](./FR-013-resolve-requested-types.md)
- [FR-014: Emit an authoring contract per resolved type](./FR-014-emit-authoring-contract.md)
- [FR-015: Emit a scoped Quire validation command](./FR-015-emit-quire-validate-command.md)
- [FR-016: Default module set manifest schema](./FR-016-default-modules-schema.md)
- [FR-017: Reconcile the default module set into the shared store](./FR-017-reconcile-default-modules.md)
- [FR-018: Map plugin source arguments to typed sources](./FR-018-map-plugin-sources.md)
- [FR-019: Install, list, and remove plugins through the registry](./FR-019-manage-plugin-registry.md)
- [FR-020: Expose workflow launchers and resolve their skills](./FR-020-resolve-workflow-skills.md)
- [FR-021: Launch workflow runs through ix-flow](./FR-021-launch-ix-flow-runs.md)
- [FR-022: Upgrade quoin to the latest published release](./FR-022-self-update.md)
- [FR-023: quoin runtime configuration surface](./FR-023-runtime-configuration.md)
- [FR-024: Expose plugin and catalog operations as a stable library API](./FR-024-plugin-catalog-library-api.md)
- [FR-025: Resolve the authoring organization](./FR-025-resolve-authoring-organization.md)
- [FR-026: Dispatch commands through the oclif runner](./FR-026-dispatch-through-oclif-runner.md)
- [FR-027: Store the authoring organization](./FR-027-store-the-authoring-organization.md)
- [FR-028: Generate property tests from classified acceptance criteria](./FR-028-generate-property-tests-from-criteria.md)
* [FR-029: Consume the published quire JSON contract](./FR-029-consume-the-quire-json-contract.md)
* [FR-030: The evidence store as the artifact of record](./FR-030-evidence-store.md)
* [FR-031: Catalog-driven verification-method advisor](./FR-031-catalog-driven-advisor.md)
* [FR-032: Suspect-link, freshness and vacuous-evidence auditor](./FR-032-evidence-auditor.md)
* [FR-033: Evidence format adapters](./FR-033-evidence-format-adapters.md)
