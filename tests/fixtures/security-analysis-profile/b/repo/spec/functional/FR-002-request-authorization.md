---
id: FR-002
title: "Every request is authorized against the caller's role"
type: FR
---
# FR-002: Every request is authorized against the caller's role

## Description

The example session gateway SHALL evaluate the caller's role against the requested operation before it forwards any request, and SHALL refuse the request when no role grants the operation.
