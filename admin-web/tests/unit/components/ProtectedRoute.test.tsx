// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { sessionStore } from "../../../src/api/session";
import { ProtectedRoute } from "../../../src/components/ProtectedRoute";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

const originalFetch = globalThis.fetch;

beforeEach(() => {
  sessionStore.logout();
  sessionStorage.clear();
  globalThis.fetch = originalFetch;
  vi.restoreAllMocks();
});

afterEach(() => {
  sessionStore.logout();
  sessionStorage.clear();
});

describe("ProtectedRoute", () => {
  it("redirects anonymous users to /login with the original target", () => {
    render(
      <MemoryRouter initialEntries={["/projects"]}>
        <Routes>
          <Route
            path="/projects"
            element={
              <ProtectedRoute>
                <div>protected content</div>
              </ProtectedRoute>
            }
          />
          <Route path="/login" element={<div>login page</div>} />
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.getByText("login page")).toBeInTheDocument();
    expect(screen.queryByText("protected content")).not.toBeInTheDocument();
  });

  it("renders children for authenticated sessions", async () => {
    globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
      const path = String(input);
      if (path.endsWith("/account/login")) {
        return Promise.resolve(
          jsonResponse(200, {
            err: 0,
            msg: "",
            result: { session: "S", refresh_session: "R" },
          }),
        );
      }
      if (path.endsWith("/account/get_account_info")) {
        return Promise.resolve(
          jsonResponse(200, {
            err: 0,
            msg: "",
            result: { id: 1, name: "alice", session_id: "x" },
          }),
        );
      }
      return Promise.reject(new Error(`unexpected path ${path}`));
    }) as unknown as typeof fetch;
    await sessionStore.login("alice", "pw");

    render(
      <MemoryRouter initialEntries={["/projects"]}>
        <Routes>
          <Route
            path="/projects"
            element={
              <ProtectedRoute>
                <div>protected content</div>
              </ProtectedRoute>
            }
          />
          <Route path="/login" element={<div>login page</div>} />
        </Routes>
      </MemoryRouter>,
    );
    expect(screen.getByText("protected content")).toBeInTheDocument();
  });
});
