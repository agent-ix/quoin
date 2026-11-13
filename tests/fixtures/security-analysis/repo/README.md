# Fixture: example session gateway

Synthetic repository for the `spec-security-analysis` skill eval. It holds three
requirements and one component, and no real third-party content. `../expected/`
is the output the skill is expected to produce for it: the clause sets installed
are exactly `nist/sp800-53@5.2.0` and `owasp/asvs@5.0.0`.

The component is a session gateway. Gaps the analysis must surface:

- storage of session data at rest applies and no requirement covers it;
- physical access applies and is outside the gateway: the deployment platform (`COMP-001`) is responsible;
- system backup applies and is outside the gateway, and nobody is known to be responsible;
- SOC 2 and ISO/IEC 27001 are not installed, so nothing is mapped for them.
