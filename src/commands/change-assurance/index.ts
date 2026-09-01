import { QuoinCommand } from "../../base.js";

export default class ChangeAssurance extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary =
    "Form, retain, and verify change-assurance evidence from explicit inputs.";
  static description = `Producer-facing surface over the FR-063 record, FR-064 proof attestation,
retained-output intake, and FR-065 verification receipt contracts (FR-068).

These commands transcribe and verify what a producer supplies. None of them
runs the command an attestation describes, spawns a process, invokes Git, or
performs a network request. Digests establish content integrity and recorded
actor labels are attribution only; nothing here establishes authorization or
non-repudiation, and no output is a certification.

Subcommands:
  quoin change-assurance seal-record
  quoin change-assurance seal-attestation
  quoin change-assurance intake
  quoin change-assurance receipt
  quoin change-assurance verify-receipt
  quoin change-assurance schema
  quoin change-assurance recover

Exit status: 0 for a valid receipt, 1 for an invalid or incomplete one, and 2
for a usage, parse, or integrity error.`;

  async run(): Promise<void> {
    await this.parse(ChangeAssurance);
    this.log(ChangeAssurance.description);
  }
}
