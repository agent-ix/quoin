---
id: CLASS-1
title: Health data
type: data_classification
object: data_classification
regulated_categories: [health]
---
# Health data

## Handling

Health data is encrypted in transit and protected from unauthorized access.

## Properties

| Field | Type | Multiplicity | Constraints |
|---|---|---|---|
| classification_id | UUID | 1..1 | identity |
| level | String | 1..1 | minLength: 1 |
