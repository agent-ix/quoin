import { execFileSync } from "node:child_process";

// Trace: TC-1589, TC-1590, TC-1591, TC-1592, FR-043-AC-32, FR-043-AC-33
test("the native verification selftests enforce candidate relocking and unchanged source gates", () => {
  // This is the same assertion suite the canonical stack invokes, including
  // temporary real Git repositories and the merge-before-promotion controls.
  // Native assertion failure propagates; no console verdict is interpreted.
  execFileSync(process.execPath, ["scripts/verification-stack-selftest.mjs"], {
    timeout: 60_000,
    stdio: ["ignore", "pipe", "pipe"],
  });
}, 65_000);
