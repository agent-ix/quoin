---
id: SR-001
title: "Security profile applicability review"
type: SpecReview
analysis: base
scope: "source-profile evaluation; spec/functional/FR-001-refresh-token-rotation.md"
review_set: subset
---

## Summary

The evaluated synthetic ASVS set contains one binding clause and one unresolved clause. The unresolved clause cannot be assessed until the source-profile context declares its `chapter` dimension.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ASVS clause V1.1.3 is unresolved because the evaluation reports the `chapter` context dimension as missing. | V1.1.3; .security-profile-eval/01-asvs.json |
