---
id: IU-001
title: Quoin intended use
type: IntendedUse
tool: quoin
relationships: []
---
# Quoin intended use

## Purpose

For a software repository team, Quoin creates, validates, and presents the
engineering records used to assess that team's requirements and verification
evidence. The tool and repository scope is defined in the master specification
at [spec/spec.md](../spec.md).

## Records produced

| Record kind | Command that writes it |
|---|---|
| Evidence-store suite-run record | `quoin evidence record` |
| Evidence-producer trust decision | `quoin evidence trust` |
| Experiment and operational-evidence records | `quoin evidence record-experiment` and `quoin evidence record-operational` |
| Clause-discharge facts | `quoin evidence record-discharge` |
| Change-assurance record and retained attestation inputs | `quoin change-assurance write-record` and `quoin change-assurance intake` |
| Software dossier documents | `quoin dossier generate` |
| QA measurement collections | `quoin measurement record`, `quoin measurement intervention`, and `quoin measurement operational-release` |
| SOUP inventory records | `quoin soup promote` |

Other commands render views and reports, including `quoin assurance`,
`quoin matrix`, `quoin discharge --rmm`, and `quoin report`; these outputs are
not persistent records unless the caller stores them separately.

## Who relies on them

Software repository maintainers, quality and assurance staff, and reviewers
who inspect a software team's requirements, verification evidence, and related
engineering records rely on these outputs. An adopter remains responsible for
deciding whether the records and configured inputs are suitable for its own
use.

## Limits

Quoin is not a compliance oracle. It does not establish that a repository or
process complies with a law, regulation, or standard, and it does not make a
product or clinical safety decision. A command's successful result is limited
to the inputs, configuration, and checks that command actually used. People
remain responsible for reviewing records and accepting decisions.

## Operating context

Quoin runs as a native command-line program in a repository or other configured
working directory. Inputs can include Markdown requirements, module manifests,
configuration, source and test files, explicit command inputs, and records in
the selected evidence store. Outputs may be written to that store or to
requested dossier and report paths. The operating environment supplies the
filesystem and any configured external executables. Depending on the command,
Quoin invokes Git to inspect repository state, Quire to validate or export
specification artifacts, Engineering Assurance to verify signatures and
approval records, and explicitly configured producer or analysis programs.
