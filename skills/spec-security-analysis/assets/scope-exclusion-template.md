---
id: SCOPEX-001
title: "RemoteAccessOutsideComponent"
type: scope_exclusion
object: scope_exclusion
---
<!-- Scope exclusion template for spec-security-analysis. One exclusion per
     clause outside the component's boundary. Take the skeleton with
     `quoin write <repo> --types scope_exclusion`; delete this comment.
     - Set / Clause: as in a control mapping; one inline-code value each.
     - Responsible: the bare id of the bundle artifact for the component,
       service or document that is responsible (for example COMP-002). If no
       responsible party is known, write NO exclusion; list the clause in the
       reply instead.
     - Rationale: why the clause is outside this component, in your own words.
     - Decision: delegated, accepted_risk or temporary_exception.
     - Decision owner: the bare id of the artifact that owns the disposition.
     - Approval ref: an opaque reference to the approval record; this field
       does not itself prove authorization.
     - Expires at: required only for temporary_exception, as an RFC 3339
       instant with an explicit time zone. Remove it for other decisions.
     Replace every placeholder with the authored decision. An exclusion is a
     boundary statement; it is not approval evidence. -->
# [SCOPEX-001] RemoteAccessOutsideComponent

## Set

`nist/sp800-53@5.2.0`

## Clause

`AC-17`

## Responsible

`COMP-002`

## Rationale

`Remote access is terminated at the network layer before a request reaches this component.`

## Decision

`delegated`

## Decision owner

`SEC-OWNER-001`

## Approval ref

`APPROVAL-001`
