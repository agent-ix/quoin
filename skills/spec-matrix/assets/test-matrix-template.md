# Test Matrix

## Overview
This matrix ensures comprehensive test coverage by transforming requirements into test cases.

## Test Matrix Rules
1. **Coverage Rule**: Every acceptance criterion (AC) must have at least one test case
2. **Option Permutation Rule**: Test all valid combinations of options
3. **Constraint Boundary Rule**: Test at boundaries of each constraint
4. **Error Path Rule**: Test all documented error conditions
5. **State Transition Rule**: Test all valid state transitions (if applicable)
6. **Edge Case Rule**: Identify and test edge cases for each requirement

## Requirements Traceability

### Stakeholder Requirement Coverage
| Stakeholder Req | Trace to US/FR | Test/Validation | Coverage Status |
|-----------------|----------------|-----------------|-----------------|
| StR-XXX | US-XXX, FR-XXX | Review | ✅ Complete |

### User Story Coverage
| User Story | Acceptance Criteria | Test Cases | Coverage Status |
|------------|---------------------|------------|-----------------|
| US-XXX | US-XXX-AC-1 | TC-001, TC-002 | ✅ Complete |
| US-XXX | US-XXX-AC-2 | TC-003 | ✅ Complete |

### Functional Requirement Coverage
| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
|----------------|---------------------|------------|-----------------|
| FR-XXX | FR-XXX-AC-1 | TC-004, TC-005 | ✅ Complete |

### Non-Functional Requirement Coverage
| Non-Functional Req | Verification Method | Evidence/Test Cases | Status |
|--------------------|---------------------|---------------------|--------|
| NFR-XXX | Test / Review | TC-006, Audit Log | ✅ Complete |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
|---------|-------|------|----------|-----------|--------|
| TC-001 | [Test Title] | [Unit/Integration/E2E/Property] | [P0/P1/P2/P3/P4] | [US-XXX-AC-X] | [✅/❌/🚧/⛔] |
| TC-002 | [Test Title] | [Unit/Integration/E2E/Property] | [P0/P1/P2/P3/P4] | [FR-XXX-AC-X] | [✅/❌/🚧/⛔] |

**Note**: Detailed test case definitions should be in separate files in `test-cases/` directory.

**`Type`** shows the four most common values inline. The full enforced set is
`Unit`, `Integration`, `E2E`, `Property`, `Fuzz`, `Benchmark`, `Static`,
`Compile`, `Snapshot`, `Manual` — see the `Test Case Summary` section of
`SKILL.md` for when each applies. Use exactly one value per row; the vocabulary
is validated, so `Unit / pg_test` and other compounds are rejected.

## Option Permutation Matrix
| Test Case | OPT-A | OPT-B | OPT-C | Expected Behavior |
|-----------|-------|-------|-------|-------------------|
| TC-XXX | Value1 | Value1 | Value1 | [behavior] |
| TC-XXX | Value1 | Value2 | Value1 | [behavior] |

## Constraint Boundary Tests
| Constraint | Boundary Type | Test Value | Test Case | Expected |
|------------|---------------|------------|-----------|----------|
| CON-1 | Min | [value] | TC-XXX | [pass/fail] |
| CON-1 | Max | [value] | TC-XXX | [pass/fail] |
| CON-1 | Below Min | [value] | TC-XXX | Error |
| CON-1 | Above Max | [value] | TC-XXX | Error |

## Edge Cases
| ID | Description | Related Req | Test Case | Risk if Untested |
|----|-------------|-------------|-----------|------------------|
| EC-001 | [edge case] | FR-XXX | TC-XXX | [risk] |

## Integration Test Matrix

> Include this section when requirements specify integration testing is required.

### Cross-Project Integrations
| Integration ID | Purpose | Target Project | Type | Test Cases | Status |
|----------------|---------|----------------|------|------------|--------|
| INT-001 | Event consumption | event-monitor | service | TC-INT-001, TC-INT-002 | 🚧 |
| INT-002 | API orchestration | orchestrator-service | service | TC-INT-003, TC-INT-004, TC-INT-005 | ❌ |

### Local Service Integrations
| Integration ID | Purpose | Service | Type | Test Cases | Status |
|----------------|---------|---------|------|------------|--------|
| INT-010 | Data persistence | postgres.ix | database | TC-INT-010a, TC-INT-010b, TC-INT-010c | ✅ |
| INT-011 | Cache operations | redis.ix | service | TC-INT-011, TC-INT-012 | 🚧 |
| INT-012 | Message publishing | event-bus | event | TC-INT-013, TC-INT-014, TC-INT-015 | ❌ |

### Browser/UI Integrations
| Integration ID | Purpose | Target | Type | Test Cases | Status |
|----------------|---------|--------|------|------------|--------|
| INT-020 | Component rendering | Storybook | browser | TC-INT-020a thru TC-INT-020f | ✅ |
| INT-021 | User interaction flows | Storybook | browser | TC-INT-021, TC-INT-022, TC-INT-023 | 🚧 |
| INT-022 | Visual regression | Storybook | browser | TC-INT-024, TC-INT-025 | ❌ |

### Integration Test Details

For each integration, define extensive test coverage:

| Test Case | Integration | Scenario | Input | Expected | Priority |
|-----------|-------------|----------|-------|----------|----------|
| TC-INT-001 | INT-001 | Happy path event processing | Valid event payload | Event processed, state updated | P0 |
| TC-INT-002 | INT-001 | Malformed event rejection | Invalid JSON schema | Event DLQ'd, error logged | P1 |
| TC-INT-010a | INT-010 | CRUD operations | Standard model data | Data persisted correctly | P0 |
| TC-INT-010b | INT-010 | Concurrent writes | Simultaneous updates | Proper lock handling | P1 |
| TC-INT-010c | INT-010 | Connection recovery | DB restart mid-operation | Graceful reconnection | P1 |
| TC-INT-020a | INT-020 | Default state render | No props | Component renders correctly | P0 |
| TC-INT-020b | INT-020 | Loading state | isLoading=true | Spinner displayed | P0 |
| TC-INT-020c | INT-020 | Error state | error prop set | Error message visible | P0 |
| TC-INT-020d | INT-020 | Interactive actions | User clicks button | Handler invoked | P1 |
| TC-INT-020e | INT-020 | Accessibility | Keyboard navigation | All controls reachable | P1 |
| TC-INT-020f | INT-020 | Mobile responsive | Viewport 375px | Layout adapts correctly | P2 |

## Coverage Gaps
| Gap ID | Description | Risk Level | Mitigation |
|--------|-------------|------------|------------|
| GAP-001 | [description] | [High/Med/Low] | [plan to address] |

## Test Execution Summary
| Category | Total | Passed | Failed | Blocked | Coverage |
|----------|-------|--------|--------|---------|----------|
| Unit | X | X | X | X | XX% |
| Integration | X | X | X | X | XX% |
| E2E | X | X | X | X | XX% |
