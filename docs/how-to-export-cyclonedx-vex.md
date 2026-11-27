---
type: how-to
description: Export reviewed SOUP VEX statements beside a CycloneDX SBOM.
---
# Export CycloneDX VEX statements

Run these steps from the repository that contains the SOUP records. Use a
Quoin build whose `quoin soup vex export --help` lists `cyclonedx` as a
supported format. The published Quoin 0.37.0 binary supports OpenVEX only.
Run `quoin module ensure-defaults` to install the modules used to read SOUP
records.

1. Generate a CycloneDX SBOM with Syft, if you do not already have one.
2. Export the reviewed SOUP statements beside that SBOM:

```sh
quoin soup vex export \
  --format cyclonedx \
  --repo . \
  --records spec/soup \
  --sbom build/syft.cdx.json \
  --out build/syft.vex.cdx.json
```

The command reads each record's `vex_statements` and writes one CycloneDX 1.7
vulnerability entry per statement. It uses the record's canonical PURL and
version to identify the affected component. The SBOM path only supplies the
default output location; the command does not read the SBOM contents.

## Expected output

The command reports the output path and the number of exported statements:

```text
Wrote CycloneDX 1.7 VEX to build/syft.vex.cdx.json (2 vulnerabilities)
```

To let Quoin choose the output path, omit `--out`. For an SBOM named
`probe.cdx.json`, Quoin writes `probe.vex.cdx.json` in the same directory.
CycloneDX output requires `--sbom` when `--out` is omitted.
