# Test Matrix - Example

## Overview
This is an example test matrix demonstrating comprehensive test coverage for the Catalog Project Ingestion feature.

## Test Matrix Rules
1. **Coverage Rule**: Every acceptance criterion (AC) must have at least one test case
2. **Option Permutation Rule**: Test all valid combinations of options
3. **Constraint Boundary Rule**: Test at boundaries of each constraint
4. **Error Path Rule**: Test all documented error conditions
5. **State Transition Rule**: Test all valid state transitions (if applicable)
6. **Edge Case Rule**: Identify and test edge cases for each requirement

## Requirements Traceability

### User Story Coverage
| User Story | Acceptance Criteria | Test Cases | Coverage Status |
|------------|---------------------|------------|-----------------|
| US-001 | US-001-AC-1 | TC-001, TC-002, TC-003 | ✅ Complete |
| US-001 | US-001-AC-2 | TC-004, TC-005 | ✅ Complete |
| US-001 | US-001-AC-3 | TC-006, TC-007, TC-008 | ✅ Complete |

### Functional Requirement Coverage
| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
|----------------|---------------------|------------|-----------------|
| FR-001 | FR-001-AC-1 | TC-001 | ✅ Complete |
| FR-001 | FR-001-AC-2 | TC-002 | ✅ Complete |
| FR-001 | FR-001-AC-3 | TC-006, TC-022 | ✅ Complete |
| FR-001 | FR-001-AC-4 | TC-007 | ✅ Complete |
| FR-001 | FR-001-AC-5 | TC-008 | ✅ Complete |
| FR-001 | FR-001-AC-6 | TC-009 | ✅ Complete |
| FR-001 | FR-001-AC-7 | TC-010 | ✅ Complete |
| FR-001 | FR-001-AC-8 | TC-011 | ✅ Complete |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
|---------|-------|------|----------|-----------|--------|
| TC-001 | Clone Public GitHub Repository Successfully | Integration | P0 | US-001-AC-1, FR-001-AC-1 | ✅ Pass |
| TC-002 | Clone Private GitHub Repository with Auth Token | Integration | P0 | US-001-AC-1, FR-001-AC-2 | ✅ Pass |
| TC-003 | Authentication Failure on Private Repository | Integration | P1 | US-001-AC-1 | ✅ Pass |
| TC-004 | Ingest from Local Filesystem Path | Integration | P0 | US-001-AC-2 | ✅ Pass |
| TC-005 | Local Path Does Not Exist | Integration | P1 | US-001-AC-2 | ✅ Pass |
| TC-006 | Identify Deployment Resources | Unit | P0 | US-001-AC-3, FR-001-AC-3 | ✅ Pass |
| TC-007 | Identify Service Resources | Unit | P0 | US-001-AC-3, FR-001-AC-4 | ✅ Pass |
| TC-008 | Extract Environment Variables from Deployment | Unit | P0 | US-001-AC-3, FR-001-AC-5 | ✅ Pass |
| TC-009 | Handle Repository with No Kubernetes Configs | Integration | P1 | FR-001-AC-6 | ✅ Pass |
| TC-010 | Reject Repository Exceeding Size Limit | Integration | P1 | FR-001-AC-7 | ✅ Pass |
| TC-011 | Handle Clone Timeout | Integration | P2 | FR-001-AC-8 | ✅ Pass |
| TC-012 | Full Clone Option | Integration | P2 | FR-001-OPT-B | 🚧 In Progress |
| TC-013 | Repository at Size Limit | Integration | P2 | FR-001-CON-1 | 🚧 In Progress |
| TC-014 | Clone at Timeout Boundary | Integration | P2 | FR-001-CON-2 | 🚧 In Progress |
| TC-015 | Monorepo with Multiple K8s Projects | Integration | P2 | EC-001 | 🚧 Partial |
| TC-016 | Mixed K8s and Non-K8s YAML Files | Integration | P2 | EC-002 | 🚧 Partial |
| TC-017 | Malformed YAML Handling | Integration | P1 | EC-003 | ❌ Missing |
| TC-018 | Symlinks to K8s Configs | Integration | P2 | EC-004 | ❌ Missing |
| TC-019 | Repository Changes During Clone | Integration | P2 | EC-005 | ❌ Missing |
| TC-022 | Resource identification is order-independent and idempotent over any generated manifest set | Property | P1 | FR-001-AC-3 | ✅ Pass |

**Note**: Detailed test case definitions are in the `test-cases/` directory.

**Note on `Type`**: TC-022 is `Property`, not `Unit`, because FR-001-AC-3 is
quantified — it holds for *any* manifest set, not for one worked example. TC-006
witnesses the criterion with a specific input; TC-022 discriminates it across a
generated domain. Both are legitimate and they trace to the same AC.

## Option Permutation Matrix
| Test Case | OPT-A (Shallow) | OPT-B (Full) | Expected Behavior |
|-----------|-----------------|--------------|-------------------|
| TC-001 | Y | N | Fast clone, only latest commit |
| TC-012 | N | Y | Slower clone, full history available |

## Constraint Boundary Tests
| Constraint | Boundary Type | Test Value | Test Case | Expected |
|------------|---------------|------------|-----------|----------|
| FR-001-CON-1 | Max Size | 1GB | TC-013 | Pass (at limit) |
| FR-001-CON-1 | Above Max | 1.1GB | TC-010 | Error CATALOG-004 |
| FR-001-CON-2 | At Timeout | 5 minutes | TC-014 | Pass (just under) |
| FR-001-CON-2 | Exceed Timeout | 5.5 minutes | TC-011 | Error CATALOG-005 |

## Edge Cases
| ID | Description | Related Req | Test Case | Risk if Untested |
|----|-------------|-------------|-----------|------------------|
| EC-001 | Monorepo with multiple K8s projects | US-001 | TC-015 | May miss components or create duplicate entries |
| EC-002 | Repository with mixed K8s and non-K8s YAML files | FR-001 | TC-016 | May incorrectly parse non-K8s YAML as components |
| EC-003 | Malformed YAML that crashes parser | FR-001 | TC-017 | Service crash or incomplete ingestion |
| EC-004 | Repository with symlinks to K8s configs | FR-001 | TC-018 | May not follow symlinks, miss components |
| EC-005 | Repository changes during clone/scan | FR-001 | TC-019 | Inconsistent state or race conditions |

## Coverage Gaps
| Gap ID | Description | Risk Level | Mitigation |
|--------|-------------|------------|------------|
| GAP-001 | No tests for GitLab/Bitbucket | Low | Not in scope for v1, document for future |
| GAP-002 | No performance tests for concurrent ingestions | Medium | Add TC-020 to test 10 concurrent ingestions |
| GAP-003 | No tests for webhook-triggered re-ingestion | High | Add TC-021 for GitHub webhook integration |

## Test Execution Summary
| Category | Total | Passed | Failed | Blocked | Coverage |
|----------|-------|--------|--------|---------|----------|
| Unit | 3 | 3 | 0 | 0 | 100% |
| Integration | 8 | 8 | 0 | 0 | 100% |
| E2E | 0 | 0 | 0 | 0 | N/A |
| Property | 1 | 1 | 0 | 0 | 100% |
| Edge Cases | 5 | 3 | 0 | 2 | 60% |
