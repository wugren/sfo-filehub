import { describe, expect, it } from "vitest";
import { sha256Hex } from "../../src/api/sha256";

describe("sha256Hex", () => {
  it("computes the SHA-256 hex digest of a Blob", async () => {
    const hash = await sha256Hex(new Blob(["abc"]));
    expect(hash).toBe("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
  });

  it("matches the server-side compressed-bytes hash contract", async () => {
    const bytes = new TextEncoder().encode("archive-bytes");
    const hash = await sha256Hex(new Blob([bytes]));
    const digest = await crypto.subtle.digest("SHA-256", bytes);
    const expected = Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
    expect(hash).toBe(expected);
    expect(hash).toMatch(/^[0-9a-f]{64}$/);
  });
});
