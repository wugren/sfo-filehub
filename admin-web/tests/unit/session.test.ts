// @vitest-environment jsdom

import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClient } from "../../src/api/client";
import { SessionStore } from "../../src/api/session";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

const originalFetch = globalThis.fetch;

beforeEach(() => {
  sessionStorage.clear();
  globalThis.fetch = originalFetch;
  vi.restoreAllMocks();
});

function client(): ApiClient {
  return new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
}

describe("SessionStore", () => {
  it("stores session credentials in sessionStorage and restores them", async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce(jsonResponse(200, { err: 0, msg: "", result: { session: "S1", refresh_session: "R1" } }))
      .mockResolvedValueOnce(jsonResponse(200, { err: 0, msg: "", result: { id: 1, name: "alice", session_id: "x" } })) as unknown as typeof fetch;
    const store = new SessionStore(client());
    const user = await store.login("alice", "pw");
    expect(user).toEqual({ id: 1, name: "alice" });
    expect(store.state).toBe("authenticated");
    expect(sessionStorage.getItem("fh_web_session")).toBe("S1");

    const restored = new SessionStore(client());
    expect(restored.state).toBe("authenticated");
    expect(restored.bearer()).toBe("S1");
  });

  it("logout clears memory and sessionStorage", async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce(jsonResponse(200, { err: 0, msg: "", result: { session: "S", refresh_session: "R" } }))
      .mockResolvedValueOnce(jsonResponse(200, { err: 0, msg: "", result: { id: 2, name: "bob", session_id: "y" } })) as unknown as typeof fetch;
    const store = new SessionStore(client());
    await store.login("bob", "pw");
    store.logout();
    expect(store.state).toBe("anonymous");
    expect(sessionStorage.length).toBe(0);
  });

  it("refreshOnce replaces credentials and a failed refresh logs out", async () => {
    const store = await loggedInStore("S", "R", 3);
    globalThis.fetch = vi.fn().mockResolvedValue(
      jsonResponse(200, { err: 0, msg: "", result: { session: "S2", refresh_session: "R2" } }),
    ) as unknown as typeof fetch;
    expect(await store.refreshOnce()).toBe(true);
    expect(store.bearer()).toBe("S2");

    const failing = await loggedInStore("S3", "R3", 4);
    globalThis.fetch = vi.fn().mockResolvedValue(
      jsonResponse(200, { err: 9, msg: "refresh flag invalid", result: null }),
    ) as unknown as typeof fetch;
    expect(await failing.refreshOnce()).toBe(false);
    expect(failing.state).toBe("anonymous");
    expect(sessionStorage.length).toBe(0);
  });
});

async function loggedInStore(session: string, refresh: string, id: number): Promise<SessionStore> {
  const fetchMock = vi.fn((input: RequestInfo | URL) => {
    const path = String(input);
    if (path.endsWith("/account/login")) {
      return Promise.resolve(
        jsonResponse(200, { err: 0, msg: "", result: { session, refresh_session: refresh } }),
      );
    }
    if (path.endsWith("/account/get_account_info")) {
      return Promise.resolve(
        jsonResponse(200, { err: 0, msg: "", result: { id, name: `u${id}`, session_id: "x" } }),
      );
    }
    return Promise.reject(new Error(`unexpected path ${path}`));
  });
  globalThis.fetch = fetchMock as unknown as typeof fetch;
  const store = new SessionStore(client());
  await store.login(`u${id}`, "pw");
  return store;
}
