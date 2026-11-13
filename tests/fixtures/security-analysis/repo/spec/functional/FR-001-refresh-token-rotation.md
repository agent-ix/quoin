---
id: FR-001
title: "Refresh tokens rotate on every exchange"
type: FR
---
# FR-001: Refresh tokens rotate on every exchange

## Description

The example session gateway SHALL issue a new refresh token on every exchange and invalidate the presented one. A reused token revokes the whole token family.
