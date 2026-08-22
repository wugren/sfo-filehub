import { describe, expect, it } from "vitest";
import { ApiError } from "../../src/api/errors";

describe("ApiError", () => {
  it("maps each v1 HTTP status to the expected error kind", () => {
    expect(ApiError.fromV1(401, '{"error":"unauthorized","message":"login required"}').kind).toBe("auth");
    expect(ApiError.fromV1(403, '{"error":"forbidden","message":"denied"}').kind).toBe("forbidden");
    expect(ApiError.fromV1(404, '{"error":"not_found","message":"missing"}').kind).toBe("not_found");
    expect(ApiError.fromV1(409, '{"error":"conflict","message":"exists"}').kind).toBe("conflict");
    expect(ApiError.fromV1(422, '{"error":"invalid_input","message":"bad"}').kind).toBe("invalid_input");
    expect(ApiError.fromV1(500, '{"error":"server_error","message":"boom"}').kind).toBe("transport");
  });

  it("uses server message text and falls back to a readable default", () => {
    const auth = ApiError.fromV1(401, '{"error":"unauthorized","message":"session expired"}');
    expect(auth.message).toContain("session expired");
    const bare = ApiError.fromV1(500, "proxy error");
    expect(bare.message).toContain("proxy error");
  });

  it("keeps account-envelope failures as auth-kind errors", () => {
    expect(ApiError.auth("bad credentials").kind).toBe("auth");
    expect(ApiError.transport("dns failed").kind).toBe("transport");
  });
});
