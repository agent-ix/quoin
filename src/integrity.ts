import { blake3 } from "@noble/hashes/blake3.js";
import { bytesToHex } from "@noble/hashes/utils.js";

const encoder = new TextEncoder();

export class IntegrityError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "IntegrityError";
  }
}

export function blake3Hex(bytes: Uint8Array): string {
  return bytesToHex(blake3(bytes));
}

export function parseStrictJson(input: string | Uint8Array): unknown {
  let text: string;
  if (typeof input === "string") {
    text = input;
  } else {
    try {
      text = new TextDecoder("utf-8", { fatal: true }).decode(input);
    } catch {
      throw new IntegrityError("input is not valid UTF-8");
    }
  }
  if (text.charCodeAt(0) === 0xfeff) {
    throw new IntegrityError("a byte-order mark is not permitted");
  }
  return new StrictJsonParser(text).parse();
}

class StrictJsonParser {
  private offset = 0;

  constructor(private readonly text: string) {}

  parse(): unknown {
    const value = this.value();
    this.space();
    if (this.offset !== this.text.length) this.fail("trailing JSON content");
    return value;
  }

  private value(): unknown {
    this.space();
    const ch = this.text[this.offset];
    if (ch === "{") return this.object();
    if (ch === "[") return this.array();
    if (ch === '"') return this.string();
    if (ch === "t") return this.literal("true", true);
    if (ch === "f") return this.literal("false", false);
    if (ch === "n") return this.literal("null", null);
    return this.number();
  }

  private object(): Record<string, unknown> {
    this.offset++;
    this.space();
    // JSON object names are data, including "__proto__". A normal `{}` would
    // route assignment through Object.prototype.__proto__ and turn attacker
    // input into inherited fields that closed-schema validation cannot see.
    const value = Object.create(null) as Record<string, unknown>;
    const keys = new Set<string>();
    if (this.take("}")) return value;
    while (true) {
      this.space();
      if (this.text[this.offset] !== '"')
        this.fail("object key must be a string");
      const key = this.string();
      if (keys.has(key))
        this.fail(`duplicate object name ${JSON.stringify(key)}`);
      keys.add(key);
      this.space();
      if (!this.take(":")) this.fail("expected ':' after object key");
      value[key] = this.value();
      this.space();
      if (this.take("}")) return value;
      if (!this.take(",")) this.fail("expected ',' or '}' in object");
    }
  }

  private array(): unknown[] {
    this.offset++;
    this.space();
    const value: unknown[] = [];
    if (this.take("]")) return value;
    while (true) {
      value.push(this.value());
      this.space();
      if (this.take("]")) return value;
      if (!this.take(",")) this.fail("expected ',' or ']' in array");
    }
  }

  private string(): string {
    const start = this.offset;
    this.offset++;
    let escaped = false;
    while (this.offset < this.text.length) {
      const code = this.text.charCodeAt(this.offset);
      if (!escaped && code === 0x22) {
        this.offset++;
        let value: string;
        try {
          value = JSON.parse(this.text.slice(start, this.offset)) as string;
        } catch {
          this.fail("invalid JSON string");
        }
        assertValidUnicode(value);
        return value;
      }
      if (!escaped && code < 0x20) this.fail("unescaped control character");
      if (!escaped && code === 0x5c) {
        escaped = true;
      } else {
        escaped = false;
      }
      this.offset++;
    }
    this.fail("unterminated JSON string");
  }

  private number(): number {
    const remaining = this.text.slice(this.offset);
    const match = /^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/.exec(
      remaining,
    );
    if (!match) this.fail("expected a JSON value");
    this.offset += match[0].length;
    const value = Number(match[0]);
    if (!Number.isFinite(value)) this.fail("number is not finite I-JSON");
    return value;
  }

  private literal<T>(token: string, value: T): T {
    if (this.text.slice(this.offset, this.offset + token.length) !== token) {
      this.fail(`expected ${token}`);
    }
    this.offset += token.length;
    return value;
  }

  private take(ch: string): boolean {
    if (this.text[this.offset] !== ch) return false;
    this.offset++;
    return true;
  }

  private space(): void {
    while ([" ", "\t", "\r", "\n"].includes(this.text[this.offset] ?? "")) {
      this.offset++;
    }
  }

  private fail(message: string): never {
    throw new IntegrityError(`${message} at byte ${this.offset}`);
  }
}

export function canonicalizeJcs(value: unknown): string {
  if (value === null) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") {
    if (!Number.isFinite(value))
      throw new IntegrityError("non-finite I-JSON number");
    return JSON.stringify(value);
  }
  if (typeof value === "string") {
    assertValidUnicode(value);
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) return `[${value.map(canonicalizeJcs).join(",")}]`;
  if (typeof value !== "object" || value === undefined) {
    throw new IntegrityError(`unsupported JSON value: ${typeof value}`);
  }
  const record = value as Record<string, unknown>;
  return `{${Object.keys(record)
    .sort()
    .map((key) => {
      if (record[key] === undefined)
        throw new IntegrityError(`undefined member ${key}`);
      assertValidUnicode(key);
      return `${JSON.stringify(key)}:${canonicalizeJcs(record[key])}`;
    })
    .join(",")}}`;
}

export function canonicalBytes(value: unknown): Uint8Array {
  return encoder.encode(canonicalizeJcs(value));
}

export function digestValue(value: Record<string, unknown>): string {
  const unsigned = { ...value };
  delete unsigned.digest;
  return blake3Hex(canonicalBytes(unsigned));
}

export function assertDigest(value: Record<string, unknown>): void {
  const digest = value.digest;
  if (typeof digest !== "string" || !/^[a-f0-9]{64}$/.test(digest)) {
    throw new IntegrityError(
      "digest must be exactly 64 lowercase hexadecimal characters",
    );
  }
  if (digestValue(value) !== digest)
    throw new IntegrityError("digest mismatch");
}

export function assertValidUnicode(value: string): void {
  for (let index = 0; index < value.length; index++) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) {
        throw new IntegrityError("invalid Unicode lone high surrogate");
      }
      index++;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      throw new IntegrityError("invalid Unicode lone low surrogate");
    }
  }
}
