import { randomUUID } from "node:crypto";
import {
  existsSync,
  linkSync,
  mkdirSync,
  readFileSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { dirname } from "node:path";

/** Publish complete bytes atomically without ever replacing retained evidence. */
export function writeFileAtomicNoReplace(
  path: string,
  bytes: string,
  collision: (path: string) => Error,
): string {
  if (existsSync(path)) {
    if (readFileSync(path, "utf8") === bytes) return path;
    throw collision(path);
  }
  mkdirSync(dirname(path), { recursive: true });
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  try {
    writeFileSync(temporary, bytes, { encoding: "utf8", flag: "wx" });
    publishFileNoReplace(temporary, path, bytes, collision);
  } finally {
    if (existsSync(temporary)) unlinkSync(temporary);
  }
  return path;
}

/** Atomic commit primitive, exported so the destination-appeared race is testable. */
export function publishFileNoReplace(
  temporary: string,
  path: string,
  bytes: string,
  collision: (path: string) => Error,
): void {
  try {
    linkSync(temporary, path);
  } catch (error) {
    if (!isNodeError(error) || error.code !== "EEXIST") throw error;
    if (readFileSync(path, "utf8") !== bytes) throw collision(path);
  }
}

function isNodeError(value: unknown): value is NodeJS.ErrnoException {
  return value instanceof Error && "code" in value;
}
