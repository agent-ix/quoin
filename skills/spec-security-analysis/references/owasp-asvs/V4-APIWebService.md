# V4 - API and Web Service

**Area**: API and Web Service  
**Focus**: HTTP service, GraphQL and WebSocket security

## When to Apply

**V4 is PRIMARY for API surfaces.** Apply when your component:
- Exposes REST, GraphQL or WebSocket endpoints
- Sits behind proxies or load balancers

## Requirements

Requirement ids and section titles follow ASVS 5.0.0. Read the requirement text in the official release.

| Req | Section | L1 | L2 | L3 |
|-----|---------|----|----|-----|
| 4.1.1 | Generic Web Service Security | ✓ | ✓ | ✓ |
| 4.1.2 | Generic Web Service Security |  | ✓ | ✓ |
| 4.1.3 | Generic Web Service Security |  | ✓ | ✓ |
| 4.1.4 | Generic Web Service Security |  |  | ✓ |
| 4.1.5 | Generic Web Service Security |  |  | ✓ |
| 4.2.1 | HTTP Message Structure Validation |  | ✓ | ✓ |
| 4.2.2 | HTTP Message Structure Validation |  |  | ✓ |
| 4.2.3 | HTTP Message Structure Validation |  |  | ✓ |
| 4.2.4 | HTTP Message Structure Validation |  |  | ✓ |
| 4.2.5 | HTTP Message Structure Validation |  |  | ✓ |
| 4.3.1 | GraphQL |  | ✓ | ✓ |
| 4.3.2 | GraphQL |  | ✓ | ✓ |
| 4.4.1 | WebSocket | ✓ | ✓ | ✓ |
| 4.4.2 | WebSocket |  | ✓ | ✓ |
| 4.4.3 | WebSocket |  | ✓ | ✓ |
| 4.4.4 | WebSocket |  | ✓ | ✓ |

## Mapping Guidance

```
FR-XXX (API Security) → 4.1.1, 4.1.4
NFR-XXX (GraphQL Limits) → 4.3.1, 4.3.2
NFR-XXX (WebSocket) → 4.4.1, 4.4.2
```
