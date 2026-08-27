// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClient } from "../../../src/api/client";
import type { TokenSummary } from "../../../src/api/contract";
import { sessionStore } from "../../../src/api/session";
import { TokensPage } from "../../../src/pages/TokensPage";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

const token: TokenSummary = {
  token_id: 7,
  name: "deploy",
  project_scope: "All",
  scopes: ["metadata:read"],
  created_at: "2026-08-20T00:00:00Z",
  updated_at: "2026-08-20T00:00:00Z",
};

function stubFetch(): ReturnType<typeof vi.fn> {
  return vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const path = String(input);
    const method = (init?.method ?? "GET").toUpperCase();
    if (path.endsWith("/account/login")) {
      return jsonResponse(200, {
        err: 0,
        msg: "",
        result: { session: "S", refresh_session: "R" },
      });
    }
    if (path.endsWith("/account/get_account_info")) {
      return jsonResponse(200, { err: 0, msg: "", result: { id: 1, name: "alice" } });
    }
    if (path.endsWith("/api/v1/tokens") && method === "GET") {
      return jsonResponse(200, [token]);
    }
    if (path.includes("/api/v1/projects") && method === "GET") {
      return jsonResponse(200, [], { "x-total-count": "0" });
    }
    if (path.endsWith(`/api/v1/tokens/${token.token_id}`) && method === "POST") {
      const body = JSON.parse(String(init?.body ?? "{}"));
      return jsonResponse(200, {
        ...token,
        name: typeof body.name === "string" ? body.name : token.name,
        scopes: Array.isArray(body.scopes) ? body.scopes : token.scopes,
      });
    }
    if (path.includes(`/api/v1/tokens/${token.token_id}/rotate`) && method === "POST") {
      return jsonResponse(200, {
        token_id: token.token_id,
        jwt: "JWT-ROTATED",
        name: token.name,
        expires_at: null,
      });
    }
    return jsonResponse(404, { error: "not_found", message: `route ${method} ${path}` });
  });
}

async function authenticate(): Promise<void> {
  await sessionStore.login("alice", "pw");
}

const originalFetch = globalThis.fetch;

beforeEach(() => {
  sessionStore.logout();
  sessionStorage.clear();
  globalThis.fetch = originalFetch;
  vi.restoreAllMocks();
});

afterEach(() => {
  cleanup();
  sessionStore.logout();
  sessionStorage.clear();
});

describe("TokensPage", () => {
  it("edit modal does not re-sign: no expiry presets, save sends update without JWT reveal", async () => {
    const fetchMock = stubFetch();
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    await authenticate();
    render(<TokensPage client={new ApiClient({ baseUrl: "http://127.0.0.1:9999" })} />);

    fireEvent.click(await screen.findByRole("button", { name: /编辑/ }));
    expect(screen.getByText(/不重新签发/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "1周" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "1个月" })).not.toBeInTheDocument();

    const nameInput = screen.getByPlaceholderText(/Token/) as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: "renamed" } });
    fireEvent.click(screen.getByRole("button", { name: /保存修改/ }));

    await waitFor(() => {
      const update = fetchMock.mock.calls.find(
        ([input, init]) =>
          String(input).endsWith(`/api/v1/tokens/${token.token_id}`) &&
          ((init as RequestInit | undefined)?.method ?? "POST").toUpperCase() === "POST",
      );
      expect(update).toBeDefined();
      const body = JSON.parse(String((update?.[1] as RequestInit | undefined)?.body ?? "{}"));
      expect(body.name).toBe("renamed");
      expect(body).not.toHaveProperty("expires_at");
    });
    expect(screen.queryByText(/JWT/)).not.toBeInTheDocument();
  });

  it("only the explicit Re-sign button issues a new JWT", async () => {
    const fetchMock = stubFetch();
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    await authenticate();
    render(<TokensPage client={new ApiClient({ baseUrl: "http://127.0.0.1:9999" })} />);

    fireEvent.click(await screen.findByRole("button", { name: /重新签发/ }));
    expect(screen.getByText(/重新签发会生成新 JWT/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /重新签发 Token/ }));

    expect(await screen.findByText("JWT-ROTATED")).toBeInTheDocument();
    const rotate = fetchMock.mock.calls.find(([input]) =>
      String(input).includes(`/api/v1/tokens/${token.token_id}/rotate`),
    );
    expect(rotate).toBeDefined();
  });

  it("loads all project pages and lets Specified scope select page-2 projects", async () => {
    const allProjects = [
      { project_id: 108, name: "project-108", visibility: "public" as const, owner: 1 },
      { project_id: 209, name: "project-209", visibility: "private" as const, owner: 1 },
    ];
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const path = String(input);
      const method = (init?.method ?? "GET").toUpperCase();
      if (path.endsWith("/account/login")) {
        return jsonResponse(200, {
          err: 0,
          msg: "",
          result: { session: "S", refresh_session: "R" },
        });
      }
      if (path.endsWith("/account/get_account_info")) {
        return jsonResponse(200, { err: 0, msg: "", result: { id: 1, name: "alice" } });
      }
      if (path.includes("/api/v1/projects?") && method === "GET") {
        const url = new URL(path, "http://stub");
        const offset = Number(url.searchParams.get("offset") ?? "0");
        const limit = Number(url.searchParams.get("limit") ?? "500");
        return jsonResponse(200, allProjects.slice(offset, offset + limit), {
          "x-total-count": String(allProjects.length),
        });
      }
      if (path.endsWith("/api/v1/tokens") && method === "GET") {
        return jsonResponse(200, [token]);
      }
      if (path.endsWith("/api/v1/tokens") && method === "POST") {
        const body = JSON.parse(String(init?.body ?? "{}"));
        return jsonResponse(200, {
          token_id: 9,
          jwt: "JWT-CREATED",
          name: String(body.name ?? "deploy"),
          expires_at: null,
        });
      }
      if (path.endsWith(`/api/v1/tokens/${token.token_id}`) && method === "POST") {
        const body = JSON.parse(String(init?.body ?? "{}"));
        return jsonResponse(200, {
          ...token,
          name: typeof body.name === "string" ? body.name : token.name,
          scopes: Array.isArray(body.scopes) ? body.scopes : token.scopes,
        });
      }
      return jsonResponse(404, { error: "not_found", message: `route ${method} ${path}` });
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    await authenticate();
    render(<TokensPage client={new ApiClient({ baseUrl: "http://127.0.0.1:9999" })} />);

    fireEvent.click(await screen.findByRole("button", { name: /新建 Token/ }));
    fireEvent.click(screen.getByRole("button", { name: /指定项目/ }));

    expect(screen.getByText("project-108")).toBeInTheDocument();
    expect(screen.getByText("project-209")).toBeInTheDocument();

    const nameInput = screen.getByPlaceholderText(/Token/) as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: "paged-token" } });
    const second = screen.getByRole("checkbox", { name: /project-209/ }) as HTMLInputElement;
    fireEvent.click(second);
    fireEvent.click(screen.getByRole("button", { name: /创建 Token/ }));

    await waitFor(() => {
      const create = fetchMock.mock.calls.find(
        ([input, requestInit]) =>
          String(input).endsWith("/api/v1/tokens") &&
          ((requestInit as RequestInit | undefined)?.method ?? "GET").toUpperCase() === "POST",
      );
      expect(create).toBeDefined();
      const body = JSON.parse(String((create?.[1] as RequestInit | undefined)?.body ?? "{}"));
      expect(body.project_scope).toEqual({ Specified: [209] });
    });
  });
});
