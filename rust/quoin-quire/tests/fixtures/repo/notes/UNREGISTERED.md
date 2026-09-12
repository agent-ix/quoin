---
id: NOTE-002
type: Memo
title: A document whose type no loaded module registers
---

`Memo` is not an archetype the fixture module declares. Distinct from
`UNTYPED.md`: one document states no premise, the other states one that does
not resolve, and a consumer that could not tell them apart would report a
missing frontmatter field as an unknown archetype.

Both live outside `spec/` so the corpus walk — and therefore the assurance
export — never sees them; they are passed explicitly to the surfaces that take
a document list.
