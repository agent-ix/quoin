# Application Spec Review Checklist

Evaluate each item. Report as FAIL (breaks UI), WARN (degraded display), or INFO (quality).

---

## 1. Frontmatter (spec.md)

- [ ] `type: master-requirements` is set
- [ ] `component_type: application` is set
- [ ] `name` matches repo name
- [ ] `org: <your-org>` is set
- [ ] `depends_on` lists all component repos
- [ ] `standards_alignment` includes at least `iso-iec-ieee-29148`
- [ ] `tags` is a non-empty array (renders in MasterRequirementsViewer)
- [ ] `security_critical` is set (boolean)

**Severity**: FAIL if `type` or `component_type` wrong (UI won't route to app view). WARN for missing tags.

---

## 2. Required Sections (spec.md)

The `parseSpec()` function in `ApplicationDetailPage.tsx` extracts these by heading name. Section names must match exactly (case-insensitive heading search via Quire `section()`).

### 2.1 Purpose
- [ ] Section titled `Purpose` exists
- [ ] Contains 2-3 sentences describing what the app solves

**Severity**: WARN if missing (falls back to `repo.description`)

### 2.2 Scope Boundaries
- [ ] Section titled `Scope Boundaries` exists

**Severity**: WARN (card hidden if missing)

### 2.3 In Scope
- [ ] Subsection titled `In Scope` exists
- [ ] Content is a bullet list (`- Item`)
- [ ] For rich display: use `**Title** — Description` format per bullet

**Severity**: WARN (shows "Not defined in spec")

### 2.4 Out of Scope
- [ ] Subsection titled `Out of Scope` exists
- [ ] Each bullet uses `(Handled by X)` or `(Delegated to X)` parenthetical
- [ ] Do NOT use `(deferred to ...)`, `(future work)`, or other patterns — `parseDelegations()` regex: `/\((?:handled by|delegated to)\s+(.+?)\)/i`

**Severity**: WARN if section missing. INFO if delegation format wrong (items render but without arrow delegation labels).

### 2.5 Operational Boundaries
- [ ] Subsection titled `Operational Boundaries` exists
- [ ] Each bullet uses `**Name**: description` format (bold-colon pattern)

**Severity**: WARN (shows "Not defined" in Design Principles card)

### 2.6 Architecture Rationale
- [ ] Section titled `Architecture Rationale` exists
- [ ] Contains at least ADR-001

**Severity**: WARN (Decisions tab empty)

---

## 3. Mermaid Diagrams

### 3.1 Logical View
- [ ] At least one mermaid code block exists in spec.md
- [ ] The primary dependency graph has `%% @type: logical` comment on line 1 (inside the fence)
- [ ] Falls back to first diagram if no tag — but tag is recommended

**Severity**: WARN (Architecture tab shows no logical view)

### 3.2 Deployment View
- [ ] A second mermaid code block with `%% @type: deployment` exists
- [ ] Shows namespace, pods, ingress, secrets, databases

**Severity**: WARN (Architecture tab deployment toggle hidden)

---

## 4. Requirement Tables (spec.md)

These tables are parsed by `parseTable()` on the section content. Column order matters (positional extraction).

### 4.1 Stakeholder Requirements
- [ ] Section `Stakeholder Requirements` exists with table
- [ ] Table columns: `ID | Title | Artifact`
- [ ] At least 1 StR row

### 4.2 User Stories
- [ ] Section `User Stories` exists with table
- [ ] Table columns: `ID | Title | Role | Artifact`
- [ ] Multiple US covering different roles

### 4.3 Functional Requirements
- [ ] Section `Functional Requirements` exists with table
- [ ] Table columns: `ID | Title | Object Type | Artifact`
- [ ] Required object types present:
  - [ ] `domain` (FR-001: bounded context + ER diagram)
  - [ ] `process` (at least 1 cross-service workflow)
  - [ ] `configuration` (shared config across services)
  - [ ] `integration` (consumer contract)

### 4.4 Non-Functional Requirements
- [ ] Section `Non-Functional Requirements` exists with table
- [ ] Table columns: `ID | Title | Artifact`
- [ ] Required NFRs:
  - [ ] Security Constraints (NFR-001)
  - [ ] System Reliability (NFR-002)
  - [ ] Deployment Topology (NFR-003)

**Severity**: FAIL if tables missing (Requirements tab empty). WARN if required object types absent.

---

## 5. Security Model (spec.md)

### 5.1 Trust Boundaries
- [ ] Section titled `Trust Boundaries` exists (under Security Model)
- [ ] Contains a table with 3 columns: Role | Capabilities | Restrictions
- [ ] Includes at minimum: anonymous, user, admin, operator roles

**Severity**: WARN (Security tab shows "No security trust boundaries defined")

---

## 6. Artifact Frontmatter Tags

The Integration and Security tabs discover artifacts via `tags` in individual artifact YAML frontmatter, not from spec.md.

### 6.1 Integration Artifacts
- [ ] FR with `object: integration` also has `tags: [integration]` in frontmatter
- [ ] Without this tag, Integration tab shows "No integration contracts found"

### 6.2 Security Artifacts
- [ ] NFR-001 (Security) has `tags: [security]` in frontmatter
- [ ] Without this tag, Security tab falls back to parsed NFR table only (no full content rendering)

**Severity**: WARN (tabs show empty/fallback states)

---

## 7. Individual Artifact Quality

For each artifact file (StR, US, FR, NFR):

### 7.1 Frontmatter
- [ ] `id` matches filename pattern (e.g., `FR-001`)
- [ ] `title` is present and descriptive
- [ ] `type` matches the directory (StR, US, FR, NFR)

### 7.2 Heading
- [ ] H1 follows pattern: `# [ID] Title`
- [ ] ID in brackets matches frontmatter `id`

### 7.3 Acceptance Criteria (FR, NFR)
- [ ] AC table exists with columns: `ID | Criteria | Verification Method`
- [ ] AC IDs follow pattern: `{artifact-id}-AC-{n}` (e.g., `FR-001-AC-1`)

### 7.4 Constraints (FR, NFR)
- [ ] Constraint table with columns: `ID | Constraint | Type | Rationale | Validation`
- [ ] Constraint IDs follow pattern: `{artifact-id}-CON-{n}`

### 7.5 Traceability
- [ ] `From:` and `Drives:` links present (or Traceability table)
- [ ] References are bidirectional (if FR-001 drives FR-002, FR-002 traces from FR-001)

### 7.6 User Stories
- [ ] AC use H3 headings with IDs: `### US-001-AC-1: Title`
- [ ] `From:` traces to StR, `Drives:` traces to FR

**Severity**: INFO for most. WARN if AC IDs missing (test matrix can't trace).

---

## 8. Test Matrix (tests.md)

- [ ] File `spec/tests.md` exists
- [ ] Coverage table maps every AC to at least one TC
- [ ] TC entries include Type and Priority, each a single value from the
      enforced vocabularies (see `spec-matrix/SKILL.md` § Test Case Summary —
      `Security` is not one of them; a security test takes the Type that
      describes how it runs)
- [ ] Component ownership note explains which repo runs each TC
- [ ] Constraint boundary tests section maps CON IDs to TCs

**Severity**: WARN if missing (no test traceability)

---

## 9. Cross-Reference Integrity

- [ ] Every artifact linked in spec.md tables exists as a file
- [ ] Every `From:` / `Drives:` reference resolves to a real artifact ID
- [ ] `depends_on` in frontmatter matches Component Registry table
- [ ] FR relationship targets (e.g., `ix://<your-org>/...`) use valid repo names
