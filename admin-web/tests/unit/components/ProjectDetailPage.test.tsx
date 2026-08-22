// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiClient } from "../../../src/api/client";
import type { Project, VersionRecord } from "../../../src/api/contract";
import { sessionStore } from "../../../src/api/session";
import { ProjectDetailPage } from "../../../src/pages/ProjectDetailPage";

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

const originalFetch = globalThis.fetch;

const project: Project = {
  project_id: 7,
  name: "demo-project",
  visibility: "private",
  owner: 1,
};

const firstSha = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const secondSha = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

const versions: VersionRecord[] = [
  {
    project_id: 7,
    version: "1.0.0",
    published_at: "2026-08-20T00:00:00Z",
    locked_at: null,
    apps: [
      {
        app: "server",
        file_id: "file-1",
        sha256: firstSha,
        size: 1024,
        updated_at: "2026-08-20T00:00:00Z",
      },
    ],
  },
  {
    project_id: 7,
    version: "2.0.0",
    published_at: "2026-08-21T00:00:00Z",
    locked_at: null,
    apps: [
      {
        app: "cli",
        file_id: "file-2",
        sha256: secondSha,
        size: 2048,
        updated_at: "2026-08-21T00:00:00Z",
      },
    ],
  },
];

function renderDetail(client: ApiClient): void {
  render(
    <MemoryRouter initialEntries={["/projects/7"]}>
      <Routes>
        <Route path="/projects/:id" element={<ProjectDetailPage client={client} />} />
      </Routes>
    </MemoryRouter>,
  );
}

async function authenticate(): Promise<void> {
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
          result: { id: 1, name: "alice" },
        }),
      );
    }
    return Promise.reject(new Error(`unexpected path ${path}`));
  }) as unknown as typeof fetch;
  await sessionStore.login("alice", "pw");
}

beforeEach(() => {
  sessionStore.logout();
  sessionStorage.clear();
  vi.restoreAllMocks();
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: vi.fn().mockResolvedValue(undefined) },
  });
});

afterEach(() => {
  sessionStore.logout();
  sessionStorage.clear();
  globalThis.fetch = originalFetch;
});

describe("ProjectDetailPage version tables", () => {
  it("keeps columns consistent and shows the complete SHA-256", async () => {
    await authenticate();
    const client = {
      listProjects: vi.fn().mockResolvedValue([project]),
      listVersions: vi.fn().mockResolvedValue(versions),
    } as unknown as ApiClient;

    renderDetail(client);

    expect(await screen.findByText(firstSha)).toBeInTheDocument();
    expect(screen.getByText(secondSha)).toBeInTheDocument();
    expect(screen.queryByText("0123456789abcdef…")).not.toBeInTheDocument();

    const tables = screen.getAllByRole("table");
    expect(tables).toHaveLength(2);
    const columnDefinitions = tables.map((table) =>
      Array.from(table.querySelectorAll("col")).map((column) => column.className),
    );
    expect(columnDefinitions[0]).toEqual(columnDefinitions[1]);
  });

  it("copies the full SHA-256 and shows success feedback", async () => {
    await authenticate();
    const client = {
      listProjects: vi.fn().mockResolvedValue([project]),
      listVersions: vi.fn().mockResolvedValue(versions),
    } as unknown as ApiClient;
    renderDetail(client);

    const copyButtons = await screen.findAllByRole("button", { name: "复制" });
    const copyButton = copyButtons[0];
    expect(copyButton).toBeDefined();
    expect(copyButton?.textContent).toBe("");
    fireEvent.click(copyButton);

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(firstSha);
      expect(screen.getByRole("button", { name: "已复制" })).toBeInTheDocument();
    });
  });

  it("reserves the delete slot so locked and unlocked downloads align", async () => {
    await authenticate();
    const client = {
      listProjects: vi.fn().mockResolvedValue([project]),
      listVersions: vi.fn().mockResolvedValue([
        versions[0],
        { ...versions[1], locked_at: "2026-08-21T01:00:00Z" },
      ]),
    } as unknown as ApiClient;
    renderDetail(client);

    await screen.findByText(firstSha);
    const actionRows = Array.from(document.querySelectorAll(".version-actions"));
    expect(actionRows).toHaveLength(2);
    expect(actionRows.map((row) => row.children.length)).toEqual([2, 2]);
    expect(actionRows[0].querySelector(".danger-icon")).toBeInTheDocument();
    expect(actionRows[1].querySelector(".version-delete-slot")).toBeInTheDocument();
    expect(actionRows[1].querySelector(".danger-icon")).not.toBeInTheDocument();
  });
});
