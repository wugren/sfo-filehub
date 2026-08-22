import { describe, expect, it, vi } from "vitest";
import { ApiError } from "../../src/api/errors";
import { SessionStore, withAuthRetry } from "../../src/api/session";

function storeLike(bearer: string | null, refreshOutcome: boolean) {
  const store = {
    bearer: vi.fn(() => bearer),
    refreshOnce: vi.fn(async () => {
      if (!refreshOutcome) {
        store.logout();
      }
      return refreshOutcome;
    }),
    logout: vi.fn(),
  } as unknown as SessionStore;
  return store;
}

describe("withAuthRetry", () => {
  it("retries exactly once with the refreshed bearer after an auth error", async () => {
    const store = storeLike("S1", true);
    const run = vi
      .fn()
      .mockRejectedValueOnce(ApiError.auth("session expired"))
      .mockResolvedValueOnce("ok");
    const result = await withAuthRetry(store, async (bearer) => {
      expect(bearer).toBe("S1");
      return run();
    });
    expect(result).toBe("ok");
    expect(store.refreshOnce).toHaveBeenCalledTimes(1);
    expect(run).toHaveBeenCalledTimes(2);
  });

  it("propagates the original auth error when refresh fails", async () => {
    const store = storeLike("S1", false);
    const run = vi.fn().mockRejectedValue(new ApiError("auth", "session expired", 401));
    await expect(
      withAuthRetry(store, () => run()),
    ).rejects.toMatchObject({ kind: "auth", message: "session expired" });
    expect(store.logout).toHaveBeenCalled();
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("fails fast for anonymous sessions without invoking the request", async () => {
    const store = storeLike(null, false);
    const run = vi.fn().mockResolvedValue("unused");
    await expect(withAuthRetry(store, (bearer) => run(bearer))).rejects.toMatchObject({
      kind: "auth",
    });
    expect(run).not.toHaveBeenCalled();
  });
});
