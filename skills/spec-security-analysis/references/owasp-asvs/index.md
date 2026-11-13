# OWASP ASVS 5.0.0

**Version**: 5.0.0  
**Scope**: Application Security Verification  
**Region**: Global (industry standard)

## When to Apply

Apply OWASP ASVS when your component:
- Is a web application or API
- Has user-facing interfaces
- Processes untrusted input
- Requires security testing criteria

## Area Selection Guide

**Use this to decide which area to investigate:**

| If your component... | Read Area |
|---------------------|-----------|
| Implements authorization/RBAC/ABAC | [V8-Authorization](V8-Authorization.md) |
| Validates user input or enforces business rules | [V2-ValidationBusinessLogic](V2-ValidationBusinessLogic.md) |
| Builds queries, commands or output from untrusted data | [V1-EncodingSanitization](V1-EncodingSanitization.md) |
| Handles errors or produces logs | [V16-LoggingErrorHandling](V16-LoggingErrorHandling.md) |
| Exposes REST/GraphQL/WebSocket APIs | [V4-APIWebService](V4-APIWebService.md) |
| Has configuration, secrets or dependencies | [V13-Configuration](V13-Configuration.md), [V15-SecureCodingArchitecture](V15-SecureCodingArchitecture.md) |
| Authenticates users | [V6-Authentication](V6-Authentication.md) |
| Manages sessions | [V7-SessionManagement](V7-SessionManagement.md) |
| Issues or accepts JWTs or similar tokens | [V9-Tokens](V9-Tokens.md) |
| Uses OAuth 2 or OpenID Connect | [V10-OAuthOIDC](V10-OAuthOIDC.md) |
| Uses encryption | [V11-Cryptography](V11-Cryptography.md) |
| Protects sensitive data | [V14-DataProtection](V14-DataProtection.md) |
| Uses network communication | [V12-SecureCommunication](V12-SecureCommunication.md) |
| Has file upload/download | [V5-FileHandling](V5-FileHandling.md) |
| Serves browsers (cookies, headers, origins) | [V3-WebFrontend](V3-WebFrontend.md) |
| Uses WebRTC | [V17-WebRTC](V17-WebRTC.md) |

## Verification Levels

| Level | Name | Apply To |
|-------|------|----------|
| L1 | Opportunistic | All applications (minimum) |
| L2 | Standard | Sensitive data, business apps |
| L3 | Advanced | Critical systems, high-value |

## Verification Areas

| Area | Primary For | Skip If |
|------|-------------|---------|
| [V1-EncodingSanitization](V1-EncodingSanitization.md) | **Injection defence** | No untrusted data |
| [V2-ValidationBusinessLogic](V2-ValidationBusinessLogic.md) | **All input handling** | No external input |
| [V3-WebFrontend](V3-WebFrontend.md) | Browser-facing apps | No browser clients |
| [V4-APIWebService](V4-APIWebService.md) | **All APIs** | No API surface |
| [V5-FileHandling](V5-FileHandling.md) | File handling | No file I/O |
| [V6-Authentication](V6-Authentication.md) | Auth services | Auth handled elsewhere |
| [V7-SessionManagement](V7-SessionManagement.md) | Stateful apps | Stateless/library |
| [V8-Authorization](V8-Authorization.md) | **Authorization** | No access decisions |
| [V9-Tokens](V9-Tokens.md) | Token issuers and verifiers | No tokens |
| [V10-OAuthOIDC](V10-OAuthOIDC.md) | OAuth/OIDC participants | No OAuth/OIDC |
| [V11-Cryptography](V11-Cryptography.md) | Encryption | No crypto usage |
| [V12-SecureCommunication](V12-SecureCommunication.md) | Network apps | Local only |
| [V13-Configuration](V13-Configuration.md) | **All software** | Never skip |
| [V14-DataProtection](V14-DataProtection.md) | Sensitive data | No sensitive data |
| [V15-SecureCodingArchitecture](V15-SecureCodingArchitecture.md) | **All software** | Never skip |
| [V16-LoggingErrorHandling](V16-LoggingErrorHandling.md) | **All production** | Never skip |
| [V17-WebRTC](V17-WebRTC.md) | WebRTC apps | No WebRTC |

## Attribution

Requirement ids and section titles are from OWASP ASVS 5.0.0, which is licensed CC BY-SA 4.0. Requirement text is not reproduced here; read it in the official release.
