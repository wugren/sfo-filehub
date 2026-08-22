// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { sessionStore } from "../../../src/api/session";
import { LoginPage } from "../../../src/pages/LoginPage";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function renderLogin(): void {
  render(
    <MemoryRouter initialEntries={["/login?next=/tokens"]}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/projects" element={<div>projects page</div>} />
        <Route path="/tokens" element={<div>tokens page</div>} />
      </Routes>
    </MemoryRouter>,
  );
}

async function fillAndSubmit(user: string, password: string): Promise<void> {
  fireEvent.change(screen.getByLabelText("用户名"), { target: { value: user } });
  fireEvent.change(screen.getByLabelText("密码"), { target: { value: password } });
  fireEvent.click(screen.getByRole("button", { name: "登录" }));
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

describe("LoginPage", () => {
  it("shows the server message when the account envelope reports failure", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      jsonResponse(200, { err: 3, msg: "用户名或密码错误", result: null }),
    ) as unknown as typeof fetch;
    renderLogin();
    await fillAndSubmit("alice", "wrong");
    expect(await screen.findByRole("alert")).toHaveTextContent("用户名或密码错误");
    expect(screen.queryByText("projects page")).not.toBeInTheDocument();
  });

  it("navigates to the original target after a successful login", async () => {
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
    renderLogin();
    await fillAndSubmit("alice", "pw");
    await waitFor(() => {
      expect(screen.getByText("tokens page")).toBeInTheDocument();
    });
  });
});
