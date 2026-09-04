import "@typespec/json-schema";
import "@agent-ix/semantic-core";

using TypeSpec.JsonSchema;
using AgentIx.Semantic.Core;

/**
 * Declaration schemas for {{ cookiecutter.repo_name }}.
 *
 * One model per type this module exports, each validating that type's Quire
 * declaration record. Every grammar item — a field, a relation, a clause, an
 * operation — is a reference to `@agent-ix/semantic-core`
 * {{ cookiecutter.semantic_core_version }} and is never redeclared here: this
 * namespace contributes shapes, not grammar.
 *
 * The `$id` base embeds the manifest `version` on purpose. One schema URL then
 * names exactly one immutable byte sequence, so a consumer that pinned a version
 * can never silently read later bytes. A bump edits this base and
 * `manifest.yaml`'s `version` in one commit and re-runs `make schemas`;
 * `scripts/generate-schemas.mjs` fails when the two disagree.
 *
 * REPLACE THE MODELS BELOW. They are a working example of the shape, not a
 * vocabulary: `Element` and `Note` demonstrate a closed record with a required
 * identity field and an open one with typed properties. Your module's types go
 * here, and `manifest.yaml`, the skeletons and the fixtures follow them.
 */
@jsonSchema("https://schemas.agent-ix.org/{{ cookiecutter.org }}/{{ cookiecutter.repo_name }}/{{ cookiecutter.version }}/")
namespace {{ cookiecutter.org.split('-') | map('capitalize') | join('') }}.{{ cookiecutter.package_name.split('_') | map('capitalize') | join('') }};

// ---------------------------------------------------------------------------
// Support models
// ---------------------------------------------------------------------------

/**
 * Open marker: a `FieldDecl` flagged as identity. Used only as a `@contains`
 * predicate, so it is deliberately open and matches any identity-flagged field.
 */
model IdentityField {
  identity: true;
}

{%- if cookiecutter.module_kind in ("object", "mixed") %}

// ---------------------------------------------------------------------------
// Object types
// ---------------------------------------------------------------------------

/**
 * A declared element: a closed record carrying at least one identity field.
 *
 * `...Record<never>` seals it. A key the schema does not name is a defect the
 * author wants to hear about, not a value that survives to a consumer that
 * cannot read it.
 */
model Element {
  @minItems(1)
  @contains(IdentityField)
  fields: FieldDecl[];

  relations?: RelationDecl[];
  clauses?: ClauseRef[];
  operations?: OperationDecl[];
  ...Record<never>;
}
{%- endif %}
{%- if cookiecutter.module_kind in ("artifact", "mixed") %}

// ---------------------------------------------------------------------------
// Artifact types
// ---------------------------------------------------------------------------

/**
 * A declared note: a document record whose typed properties are extracted from
 * its Markdown body. It carries no identity field requirement — a document's
 * identity is its frontmatter `id`, not a declared field.
 */
model Note {
  fields: FieldDecl[];
  clauses?: ClauseRef[];
  ...Record<never>;
}
{%- endif %}
