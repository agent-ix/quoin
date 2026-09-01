import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  blake3Hex,
  sealAttestation,
  type ProofAttestation,
} from "../../change-assurance/index.js";
import {
  canonicalOutput,
  jsonFlag,
  messageOf,
  readInputBytes,
  readInputJson,
  refuseSuppliedFields,
} from "./common.js";

export default class ChangeAssuranceSealAttestation extends QuoinCommand {
  protected skipUpdateNudge = true;
  static summary =
    "Seal a proof attestation over an existing retained result file.";
  static description = `Binds ONE already-produced result file to the reviewed record, candidate
revision, proof obligation, command, tool, configuration, environment, and
result the caller states (FR-064), and emits the sealed attestation JSON.

The ONLY fields derived here are \`retained_output.digest\` and
\`retained_output.size_bytes\`, read from the bytes of --output, plus the
\`media_type\` the caller declares. Everything else is the caller's: this
command does not run the proof command, re-read the tool, inspect the
repository, or infer a result from the file it hashes. A body supplying
\`retained_output\` or \`digest\` is refused.

The attestation is emitted, not retained. \`quoin change-assurance intake\`
is the retention step, and it re-checks this binding against the same bytes.`;

  static examples = [
    "quoin change-assurance seal-attestation --input attestation.json --output junit.xml --media-type application/xml",
  ];

  static flags = {
    input: Flags.string({
      description:
        "Proof-attestation body as JSON, without `digest` and without " +
        "`retained_output`. `-` reads stdin.",
      required: true,
    }),
    output: Flags.string({
      description:
        "Path of the retained result file this attestation describes. Its " +
        "bytes are hashed and measured; it is never executed.",
      required: true,
    }),
    "media-type": Flags.string({
      description:
        "Declared media type of the retained result file. Stated by the " +
        "caller rather than sniffed, so a producer's own content type is " +
        "preserved exactly.",
      required: true,
    }),
    json: jsonFlag,
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(ChangeAssuranceSealAttestation);

    let body: unknown;
    try {
      body = readInputJson(flags.input);
    } catch (error) {
      this.error(`cannot read --input ${flags.input}: ${messageOf(error)}`, {
        exit: 2,
      });
    }

    const supplied = refuseSuppliedFields(body, ["digest", "retained_output"]);
    if (supplied) this.error(supplied, { exit: 2 });

    let output: Uint8Array;
    try {
      output = readInputBytes(flags.output);
    } catch (error) {
      this.error(`cannot read --output ${flags.output}: ${messageOf(error)}`, {
        exit: 2,
      });
    }

    let attestation: ProofAttestation;
    try {
      attestation = sealAttestation({
        ...(body as Omit<ProofAttestation, "digest" | "retained_output">),
        retained_output: {
          media_type: flags["media-type"],
          digest: blake3Hex(output),
          size_bytes: output.byteLength,
        },
      });
    } catch (error) {
      this.error(`cannot seal attestation: ${messageOf(error)}`, { exit: 2 });
    }

    this.log(canonicalOutput(attestation));
  }
}
