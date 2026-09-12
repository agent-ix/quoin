---
id: NOTE-001
title: A document whose frontmatter declares no type
---

Deliberately carries no `type:`. `quire properties` exits 1 over a document
like this while still emitting a complete payload for every document that did
resolve (agent-ix/quoin#103); here it becomes one `Unresolved` record and
costs the other documents nothing.
