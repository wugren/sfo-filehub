// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClient } from "../../../src/api/client";
import type { Project } from "../../../src/api/contract";
import { LanguageProvider } from "../../../src/i18n";
import { sessionStore } from "../../../src/api/session";
import { ProjectsPage } from "../../../src/pages/ProjectsPage";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function makeProjects(count: number, startId: number): Project[] {
  return Array.from({ length: count }, (_, index) => ({
    project_id: startId + index,
    name: `project-${startId + index}`,
    visibility: "private" as const,
    owner: 1,
  }));
}

function stubSessionFetch(): ReturnType<typeof vi.fn> {
  return vi.fn(async (input: RequestInfo | URL) => {
    const path = String(input);
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
    return jsonResponse(404, { error: "not_found", message: `route ${path}` });
  });
}

async function authenticate(): Promise<void> {
  await sessionStore.login("alice", "pw");
}

function renderPage(mock: Record<string, unknown>): void {
  render(
    <LanguageProvider>
      <MemoryRouter>
        <ProjectsPage client={mock as unknown as ApiClient} />
      </MemoryRouter>
    </LanguageProvider>,
  );
}

const originalFetch = globalThis.fetch;

beforeEach(() => {
  sessionStore.logout();
  sessionStorage.clear();
  localStorage.setItem("fh_web_lang", "zh");
  vi.restoreAllMocks();
  globalThis.fetch = originalFetch;
});

afterEach(() => {
  sessionStore.logout();
  sessionStorage.clear();
  localStorage.removeItem("fh_web_lang");
});

describe("ProjectsPage pagination", () => {
  it("loads the first page, shows the total, and disables Previous", async () => {
    globalThis.fetch = stubSessionFetch() as unknown as typeof fetch;
    await authenticate();
    const listProjectsPage = vi.fn().mockResolvedValue({
      items: makeProjects(10, 1),
      total: 23,
    });
    renderPage({ listProjectsPage });

    expect(await screen.findByText("project-1")).toBeInTheDocument();
    expect(screen.getByText("23")).toBeInTheDocument();
    expect(screen.getByText(/第 1 \/ 3 页 · 共 23 个项目/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "上一页" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "下一页" })).toBeEnabled();
    expect(listProjectsPage).toHaveBeenCalledWith("S", { limit: 10, offset: 0 });
  });

  it("navigates forward and backward and disables buttons at the edges", async () => {
    globalThis.fetch = stubSessionFetch() as unknown as typeof fetch;
    await authenticate();
    const listProjectsPage = vi.fn(async (_bearer: string, options: { limit?: number; offset?: number }) => {
      const offset = options.offset ?? 0;
      return { items: makeProjects(10, offset + 1), total: 23 };
    });
    renderPage({ listProjectsPage });

    await screen.findByText("project-1");
    fireEvent.click(screen.getByRole("button", { name: "下一页" }));
    await waitFor(() => {
      expect(listProjectsPage).toHaveBeenLastCalledWith("S", { limit: 10, offset: 10 });
    });
    expect(await screen.findByText("project-11")).toBeInTheDocument();
    expect(screen.getByText(/第 2 \/ 3 页/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "上一页" }));
    await waitFor(() => {
      expect(listProjectsPage).toHaveBeenLastCalledWith("S", { limit: 10, offset: 0 });
    });
    expect(await screen.findByText("project-1")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "上一页" })).toBeDisabled();
  });

  it("disables both buttons when the whole list fits on one page", async () => {
    globalThis.fetch = stubSessionFetch() as unknown as typeof fetch;
    await authenticate();
    const listProjectsPage = vi.fn().mockResolvedValue({
      items: makeProjects(10, 1),
      total: 10,
    });
    renderPage({ listProjectsPage });

    await screen.findByText("project-1");
    expect(screen.getByText(/第 1 \/ 1 页 · 共 10 个项目/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "上一页" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "下一页" })).toBeDisabled();
  });

  it("jumps to the last page after creating a project", async () => {
    globalThis.fetch = stubSessionFetch() as unknown as typeof fetch;
    await authenticate();
    const listProjectsPage = vi.fn().mockResolvedValue({
      items: makeProjects(10, 1),
      total: 23,
    });
    const createProject = vi.fn().mockResolvedValue({
      project_id: 24,
      name: "new-project",
      visibility: "private",
      owner: 1,
    });
    renderPage({ listProjectsPage, createProject });

    await screen.findByText("project-1");
    fireEvent.click(screen.getByRole("button", { name: /新建项目/ }));
    const nameInput = screen.getByPlaceholderText("my-project") as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: "new-project" } });
    fireEvent.click(screen.getByRole("button", { name: /创建/ }));

    await waitFor(() => expect(createProject).toHaveBeenCalledWith("S", "new-project", "public"));
    await waitFor(() => {
      expect(listProjectsPage).toHaveBeenLastCalledWith("S", { limit: 10, offset: 20 });
    });
  });

  it("steps back one page when deleting the last row of the current page", async () => {
    globalThis.fetch = stubSessionFetch() as unknown as typeof fetch;
    await authenticate();
    const listProjectsPage = vi
      .fn()
      .mockResolvedValueOnce({ items: makeProjects(10, 1), total: 11 })
      .mockResolvedValueOnce({ items: makeProjects(1, 11), total: 11 })
      .mockResolvedValueOnce({ items: makeProjects(10, 1), total: 10 });
    const deleteProject = vi.fn().mockResolvedValue(undefined);
    renderPage({ listProjectsPage, deleteProject });

    await screen.findByText("project-1");
    fireEvent.click(screen.getByRole("button", { name: /下一页/ }));
    await screen.findByText("project-11");

    const rowDelete = screen.getAllByRole("button", { name: /删除项目/ });
    fireEvent.click(rowDelete[0]!);
    const modalDelete = screen.getAllByRole("button", { name: /删除项目/ });
    fireEvent.click(modalDelete[1]!);

    await waitFor(() => expect(deleteProject).toHaveBeenCalledWith("S", 11));
    await waitFor(() => {
      expect(listProjectsPage).toHaveBeenLastCalledWith("S", { limit: 10, offset: 0 });
    });
    expect(await screen.findByText("project-1")).toBeInTheDocument();
    expect(screen.getByText(/第 1 \/ 1 页 · 共 10 个项目/)).toBeInTheDocument();
  });
});
