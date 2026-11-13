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
     An exclusion is a boundary statement; approval is recorded elsewhere. -->
# [SCOPEX-001] RemoteAccessOutsideComponent

## Set

`nist/sp800-53@5.2.0`

## Clause

`AC-17`

## Responsible

`COMP-002`

## Rationale

`Remote access is terminated at the network layer before a request reaches this component.`
