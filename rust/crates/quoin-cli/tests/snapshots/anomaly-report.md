# Anomaly report

## Anomalies

| ID | Title | Path | Status | Resolution | Severity | Safety impact | Flag | Assessment | Affected | Release rationale | Fixing change | Rationale change |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| AN-001 | Fixture identified with severity | spec/anomalies/AN-001.md | identified | not stated | minor | unassessed | unassessed | none | none | none | absent | absent |
| AN-002 | Fixture identified | spec/anomalies/AN-002.md | identified | not stated | not stated | unassessed | unassessed | none | none | none | absent | absent |
| AN-003 | Fixture rationale resolved | spec/anomalies/AN-003.md | resolved | fixed | major | impacting | none | Requirement FR-001 Affects: The anomaly shows FR-001 is not enforced.<br>Requirement FR-001-AC-1 Affects: The criterion is not met.<br>Hazard HAZ-001 Introduces: A corrupt artifact is accepted. | Requirement FR-001 (Affects)<br>Requirement FR-001-AC-1 (Affects)<br>Hazard HAZ-001 (Introduces) | Morgan Reyes: The checksum path is not on a safety function and the operator procedure catches the case until the next release. | absent (pending) | absent (pending) |
| AN-004 | Fixture rationale resolved | spec/anomalies/AN-004.md | resolved | fixed | major | impacting | none | Requirement FR-001 Affects: The anomaly shows FR-001 is not enforced.<br>Requirement FR-001-AC-1 Affects: The criterion is not met.<br>Hazard HAZ-001 Introduces: A corrupt artifact is accepted. | Requirement FR-001 (Affects)<br>Requirement FR-001-AC-1 (Affects)<br>Hazard HAZ-001 (Introduces) | Morgan Reyes: The checksum path is not on a safety function and the operator procedure catches the case until the next release. | <commit-1> Fixture Author 2026-01-01T00:05:00+00:00 unsigned (N) | <commit-1> Fixture Author 2026-01-01T00:05:00+00:00 unsigned (N) |
| AN-006 | Fixture closed wont-fix | spec/anomalies/AN-006.md | closed | wont-fix | major | impacting | none | Requirement FR-001 Affects: The anomaly shows FR-001 is not enforced.<br>Requirement FR-001-AC-1 Affects: The criterion is not met.<br>Hazard HAZ-001 Introduces: A corrupt artifact is accepted. | Requirement FR-001 (Affects)<br>Requirement FR-001-AC-1 (Affects)<br>Hazard HAZ-001 (Introduces) | Morgan Reyes: The checksum path is not on a safety function and the operator procedure catches the case until the next release. | absent | <commit-1> Fixture Author 2026-01-01T00:05:00+00:00 unsigned (N) |
| AN-007 | Fixture closed wont-fix | spec/anomalies/AN-007.md | closed | wont-fix | major | impacting | none | Requirement FR-001 Affects: The anomaly shows FR-001 is not enforced.<br>Requirement FR-001-AC-1 Affects: The criterion is not met.<br>Hazard HAZ-001 Introduces: A corrupt artifact is accepted. | Requirement FR-001 (Affects)<br>Requirement FR-001-AC-1 (Affects)<br>Hazard HAZ-001 (Introduces) | none | absent | absent |
| AN-009 | Fixture two hazards | spec/anomalies/AN-009.md | assessed | not stated | major | impacting | none | Hazard HAZ-001 Introduces: A corrupt artifact is accepted.<br>Hazard HAZ-002 Affects: A second corrupt-artifact path. | Hazard HAZ-001 (Introduces)<br>Hazard HAZ-002 (Affects) | none | absent | absent |
| AN-010 | Fixture none | spec/anomalies/AN-010.md | assessed | not stated | major | none | none | None not stated None: Cosmetic message only; no safety function involved. | none | none | absent | absent |
| AN-011 | Fixture f3 | spec/anomalies/AN-011.md | closed | fixed | major | unassessed | unassessed | Requirement FR-001 Affects: The anomaly shows FR-001 is not enforced.<br>Requirement FR-001-AC-1 Affects: The criterion is not met.<br>Hazard HAZ-001 Introduces: A corrupt artifact is accepted. | Requirement FR-001 (Affects)<br>Requirement FR-001-AC-1 (Affects)<br>Hazard HAZ-001 (Introduces) | none | <commit-1> Fixture Author 2026-01-01T00:05:00+00:00 unsigned (N) | absent |
| AN-012 | Fixture g | spec/anomalies/AN-012.md | assessed | not stated | major | impacting | none | Requirement FR-999 Affects: Names a requirement that does not exist.<br>Hazard HAZ-001 Introduces: A corrupt artifact is accepted. | Requirement FR-999 (Affects)<br>Hazard HAZ-001 (Introduces) | none | absent | absent |
| AN-013 | Fixture rationale assessed | spec/anomalies/AN-013.md | assessed | not stated | major | impacting | none | Requirement FR-001 Affects: The anomaly shows FR-001 is not enforced.<br>Requirement FR-001-AC-1 Affects: The criterion is not met.<br>Hazard HAZ-001 Introduces: A corrupt artifact is accepted. | Requirement FR-001 (Affects)<br>Requirement FR-001-AC-1 (Affects)<br>Hazard HAZ-001 (Introduces) | none | absent | absent |

## By hazard

### HAZ-001

- AN-003 (Introduces)
- AN-004 (Introduces)
- AN-006 (Introduces)
- AN-007 (Introduces)
- AN-009 (Introduces)
- AN-011 (Introduces)
- AN-012 (Introduces)
- AN-013 (Introduces)

### HAZ-002

- AN-009 (Affects)

## By requirement

### FR-001

- AN-003 (Affects)
- AN-004 (Affects)
- AN-006 (Affects)
- AN-007 (Affects)
- AN-011 (Affects)
- AN-013 (Affects)

### FR-001-AC-1

- AN-003 (Affects)
- AN-004 (Affects)
- AN-006 (Affects)
- AN-007 (Affects)
- AN-011 (Affects)
- AN-013 (Affects)

### FR-999

- AN-012 (Affects)

## Ungrouped

- AN-001: unassessed
- AN-002: unassessed
- AN-010: no-affected-id

## Unreadable

- spec/anomalies/AN-014.md: missing

