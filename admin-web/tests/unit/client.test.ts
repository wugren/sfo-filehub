import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClient } from "../../src/api/client";
import { ApiError } from "../../src/api/errors";

function jsonResponse(status: number, body: unknown, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json", ...headers },
  });
}

function envelope(err: number, result: unknown, msg = ""): unknown {
  return { err, msg, result };
}

const originalFetch = globalThis.fetch;

beforeEach(() => {
  vi.restoreAllMocks();
});

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe("ApiClient account envelope handling", () => {
  it("unwraps the sfo-http envelope on successful login", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      jsonResponse(200, envelope(0, { session: "S", refresh_session: "R" })),
    ) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    const result = await client.login("alice", "pw");
    expect(result.session).toBe("S");
    const body = JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body as string);
    expect(body).toMatchObject({ user_name: "alice", password: "pw" });
    expect(typeof body.timestamp).toBe("number");
  });

  it("turns an HTTP-200 envelope with err!=0 into an auth error", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      jsonResponse(200, envelope(7, null, "bad credentials")),
    ) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    await expect(client.login("alice", "wrong")).rejects.toMatchObject({
      kind: "auth",
      message: "bad credentials",
    });
  });

  it("maps /api/v1 errors through the {error,message} error body", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      jsonResponse(403, { error: "forbidden", message: "administration required" }),
    ) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    await expect(client.listProjects("S")).rejects.toMatchObject({
      kind: "forbidden",
      status: 403,
    });
  });
});

describe("ApiClient token contract details", () => {
  it("serializes project_scope for a token create request", async () => {
    let captured: unknown;
    globalThis.fetch = vi.fn(async (_url: RequestInfo | URL, init?: RequestInit) => {
      captured = JSON.parse(String(init?.body));
      return jsonResponse(201, { token_id: 1, jwt: "J", name: "deploy", expires_at: null });
    }) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    await client.createToken("B", {
      name: "deploy",
      project_scope: { Specified: [3, 9] },
      scopes: ["metadata:read", "artifacts:read"],
      expires_at: null,
    });
    expect(captured).toEqual({
      name: "deploy",
      project_scope: { Specified: [3, 9] },
      scopes: ["metadata:read", "artifacts:read"],
      expires_at: null,
    });
  });

  it("distinguishes name-only update (TokenSummary) from a resign (TokenIssued)", async () => {
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    globalThis.fetch = vi.fn().mockResolvedValueOnce(
      jsonResponse(200, {
        token_id: 5,
        name: "renamed",
        project_scope: "All",
        scopes: ["artifacts:read"],
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-01-01T00:00:00Z",
      }),
    ).mockResolvedValueOnce(
      jsonResponse(200, { token_id: 5, jwt: "NEW", name: "renamed", expires_at: null }),
    ) as unknown as typeof fetch;

    const nameOnly = await client.updateToken("B", 5, { name: "renamed" });
    expect("jwt" in nameOnly).toBe(false);
    const resigned = await client.updateToken("B", 5, { scopes: ["artifacts:read"] });
    expect("jwt" in resigned).toBe(true);
  });

  it("sends Authorization on download and returns a Blob", async () => {
    const bytes = new TextEncoder().encode("tarball-bytes");
    const mock = vi.fn(async (_url: RequestInfo | URL, init?: RequestInit) => {
      expect(init?.headers).toMatchObject({ Authorization: "Bearer B" });
      return new Response(new Blob([bytes]), {
        status: 200,
        headers: {
          "content-type": "application/gzip",
          "content-disposition": 'attachment; filename="4-1.0.0.tar.gz"',
        },
      });
    });
    globalThis.fetch = mock as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    const blob = await client.download("B", 4, "1.0.0", "default");
    expect(await blob.arrayBuffer()).toEqual(bytes.buffer);
    const url = String(mock.mock.calls[0][0]);
    expect(url).toContain("/api/v1/projects/4/versions/1.0.0/download");
    expect(url).toContain("app=default");
  });

  it("classifies a failed download with a v1 error body", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      jsonResponse(422, { error: "invalid_input", message: "version not found" }),
    ) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    await expect(client.download("B", 4, "bad", "default")).rejects.toMatchObject({
      kind: "invalid_input",
    });
  });

  it("converts network errors into transport errors", async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new TypeError("failed to fetch")) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    await expect(client.listProjects("B")).rejects.toMatchObject({ kind: "transport" });
  });
});

describe("ApiClient version/app lifecycle methods", () => {
  it("creates a version with a JSON body", async () => {
    let captured: unknown;
    globalThis.fetch = vi.fn(async (_url: RequestInfo | URL, init?: RequestInit) => {
      captured = JSON.parse(String(init?.body));
      return jsonResponse(201, {
        project_id: 4,
        version: "1.0.0",
        published_at: "2026-08-21T00:00:00Z",
        locked_at: null,
        apps: [],
      });
    }) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    const record = await client.createVersion("B", 4, "1.0.0");
    expect(record.apps).toEqual([]);
    expect(captured).toEqual({ version: "1.0.0" });
    const url = String((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]);
    expect(url).toContain("/api/v1/projects/4/versions");
  });

  it("uploads an app as multipart PUT with bearer", async () => {
    const mock = vi.fn(async (_url: RequestInfo | URL, init?: RequestInit) => {
      expect(init?.method).toBe("PUT");
      expect(init?.headers).toMatchObject({ Authorization: "Bearer B" });
      expect(init?.body).toBeInstanceOf(FormData);
      return jsonResponse(201, {
        project_id: 4,
        version: "1.0.0",
        published_at: "2026-08-21T00:00:00Z",
        locked_at: null,
        apps: [
          {
            app: "server",
            file_id: "f1",
            sha256: "s",
            size: 1,
            updated_at: "2026-08-21T00:00:00Z",
          },
        ],
      });
    });
    globalThis.fetch = mock as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    const record = await client.uploadApp("B", 4, "1.0.0", "server", new Blob(["x"]));
    expect(record.apps[0].app).toBe("server");
    const url = String(mock.mock.calls[0][0]);
    expect(url).toContain("/api/v1/projects/4/versions/1.0.0/apps/server");
  });

  it("maps 409 lock conflicts to conflict errors for upload", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      jsonResponse(409, { error: "conflict", message: "version is locked" }),
    ) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    await expect(
      client.uploadApp("B", 4, "1.0.0", "web", new Blob(["x"])),
    ).rejects.toMatchObject({ kind: "conflict", status: 409 });
  });

  it("locks a version and deletes an app through PUT/DELETE", async () => {
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce(jsonResponse(200, {
        project_id: 4,
        version: "1.0.0",
        published_at: "2026-08-21T00:00:00Z",
        locked_at: "2026-08-21T01:00:00Z",
        apps: [],
      }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    const locked = await client.lockVersion("B", 4, "1.0.0");
    expect(locked.locked_at).not.toBeNull();
    await client.deleteApp("B", 4, "1.0.0", "web");
    const urls = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls.map((call) => String(call[0]));
    expect(urls[0]).toContain("/versions/1.0.0/lock");
    expect(urls[1]).toContain("/versions/1.0.0/apps/web");
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[1][1]).toMatchObject({
      method: "DELETE",
    });
  });
});

describe("ApiClient empty responses", () => {
  it("accepts 204 for deletes and revokes", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(new Response(null, { status: 204 })) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://127.0.0.1:9999" });
    await expect(client.deleteProject("B", 1)).resolves.toBeUndefined();
    await expect(client.revokeToken("B", 9)).resolves.toBeUndefined();
  });
});

it("exposes ApiError type for consumers", () => {
  expect(ApiError).toBeDefined();
});
