import { once } from "node:events";
import {
  createServer,
  type IncomingMessage,
  type Server,
  type ServerResponse,
} from "node:http";
import type { AddressInfo } from "node:net";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { ApiClient } from "../../src/api/client";

const ACCOUNTS: Record<string, { id: number; name: string; password: string }> = {
  alice: { id: 1, name: "alice", password: "pw" },
  bob: { id: 2, name: "bob", password: "pw" },
};

interface StubToken {
  token_id: number;
  name: string;
  project_scope: unknown;
  scopes: string[];
  created_at: string;
  updated_at: string;
  revoked: boolean;
}

const state = {
  sessions: new Map<string, number>(),
  projects: [
    { project_id: 1, name: "demo", visibility: "private" as const, owner: 1 },
  ],
  collaborators: new Map<number, Map<number, string>>(),
  tokens: new Map<number, StubToken>(),
  versions: [
    {
      project_id: 1,
      version: "1.0.0",
      published_at: "2026-08-20T00:00:00Z",
      locked_at: null,
      apps: [
        {
          app: "default",
          file_id: "f1",
          sha256: "abc123",
          size: 7,
          updated_at: "2026-08-20T00:00:00Z",
        },
      ],
    },
  ],
  nextTokenId: 1,
  nextProjectId: 2,
  seenAuthHeaders: [] as string[],
};

function sendJson(res: ServerResponse, status: number, body: unknown, headers: Record<string, string> = {}): void {
  res.writeHead(status, { "content-type": "application/json", ...headers });
  res.end(JSON.stringify(body));
}

function sendEmpty(res: ServerResponse, status: number): void {
  res.writeHead(status);
  res.end();
}

function envelope(err: number, result: unknown, msg = ""): unknown {
  return { err, msg: err ? msg : "", result };
}

async function readJson(req: IncomingMessage): Promise<Record<string, unknown> | null> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(chunk as Buffer);
  }
  if (chunks.length === 0) {
    return null;
  }
  try {
    return JSON.parse(Buffer.concat(chunks).toString("utf-8")) as Record<string, unknown>;
  } catch {
    return null;
  }
}

function bearer(req: IncomingMessage): string | null {
  const value = req.headers.authorization;
  if (!value || !value.toLowerCase().startsWith("bearer ")) {
    return null;
  }
  const token = value.slice("Bearer ".length).trim();
  state.seenAuthHeaders.push(value);
  return token || null;
}

async function handle(req: IncomingMessage, res: ServerResponse): Promise<void> {
  const url = new URL(req.url ?? "/", "http://stub");
  const parts = url.pathname.split("/").filter(Boolean);
  const method = req.method ?? "GET";

  if (parts[0] === "account") {
    if (method === "POST" && parts[1] === "login") {
      const body = (await readJson(req)) as { user_name?: string; password?: string };
      const account = body?.user_name ? ACCOUNTS[body.user_name] : undefined;
      if (!account || account.password !== body?.password) {
        sendJson(res, 200, envelope(3, null, "用户名或密码错误"));
        return;
      }
      const session = `S-${Date.now()}-${Math.random().toString(16).slice(2)}`;
      const refresh = `R-${Date.now()}-${Math.random().toString(16).slice(2)}`;
      state.sessions.set(session, account.id);
      state.sessions.set(refresh, account.id);
      sendJson(res, 200, envelope(0, { session, refresh_session: refresh }));
      return;
    }
    if (method === "GET" && parts[1] === "get_account_info") {
      const id = state.sessions.get(bearer(req) ?? "");
      if (!id) {
        sendJson(res, 200, envelope(7, null, "invalid session"));
        return;
      }
      const account = Object.values(ACCOUNTS).find((item) => item.id === id);
      sendJson(res, 200, envelope(0, { id, name: account?.name, session_id: "x" }));
      return;
    }
    if (method === "POST" && parts[1] === "refresh_session") {
      const id = state.sessions.get(bearer(req) ?? "");
      if (!id) {
        sendJson(res, 200, envelope(9, null, "refresh flag invalid"));
        return;
      }
      sendJson(res, 200, envelope(0, { session: "S-NEW", refresh_session: "R-NEW" }));
      return;
    }
    sendJson(res, 404, { error: "not_found", message: "unknown account route" });
    return;
  }

  if (parts[0] === "api" && parts[1] === "v1") {
    if (method === "GET" && parts[2] === "projects" && parts.length === 3) {
      const limitRaw = url.searchParams.get("limit");
      const offsetRaw = url.searchParams.get("offset");
      const offset = offsetRaw !== null ? Number(offsetRaw) : 0;
      const end = limitRaw !== null ? offset + Number(limitRaw) : state.projects.length;
      sendJson(res, 200, state.projects.slice(offset, end), {
        "x-total-count": String(state.projects.length),
      });
      return;
    }
    if (method === "POST" && parts[2] === "projects" && parts.length === 3) {
      const body = (await readJson(req)) ?? {};
      const project = {
        project_id: state.nextProjectId++,
        name: String(body.name ?? "unnamed"),
        visibility: body.visibility === "public" ? "public" : "private",
        owner: 1,
      };
      state.projects.push(project);
      sendJson(res, 201, project);
      return;
    }
    const projectMatch = parts[2] === "projects" && /^\d+$/.test(parts[3] ?? "");
    if (projectMatch && parts.length === 4 && method === "GET") {
      const id = Number(parts[3]);
      const project = state.projects.find((item) => item.project_id === id);
      if (!project) {
        sendJson(res, 404, { error: "not_found", message: "project not found" });
        return;
      }
      sendJson(res, 200, project);
      return;
    }
    if (projectMatch && parts.length === 4 && method === "DELETE") {
      const id = Number(parts[3]);
      state.projects = state.projects.filter((item) => item.project_id !== id);
      sendEmpty(res, 204);
      return;
    }
    if (projectMatch && parts[4] === "visibility" && method === "POST") {
      const id = Number(parts[3]);
      const project = state.projects.find((item) => item.project_id === id);
      if (!project) {
        sendJson(res, 404, { error: "not_found", message: "project not found" });
        return;
      }
      const body = (await readJson(req)) ?? {};
      project.visibility = body.visibility === "public" ? "public" : "private";
      sendJson(res, 200, project);
      return;
    }
    if (projectMatch && parts[4] === "collaborators" && parts.length === 5 && method === "GET") {
      const list = Array.from(state.collaborators.get(Number(parts[3]))?.entries() ?? []).map(
        ([user_id, role]) => ({ user_id, role }),
      );
      sendJson(res, 200, list);
      return;
    }
    if (projectMatch && parts[4] === "collaborators" && parts.length === 6 && method === "PUT") {
      const id = Number(parts[3]);
      const body = (await readJson(req)) ?? {};
      const userId = Number(parts[5]);
      if (userId === 1) {
        sendJson(res, 403, { error: "forbidden", message: "project owner cannot be granted" });
        return;
      }
      const role = String(body.role ?? "read");
      state.collaborators.set(id, (state.collaborators.get(id) ?? new Map()).set(userId, role));
      sendJson(res, 200, { user_id: userId, role });
      return;
    }
    if (projectMatch && parts[4] === "collaborators" && parts.length === 6 && method === "DELETE") {
      const id = Number(parts[3]);
      const userId = Number(parts[5]);
      if (userId === 1) {
        sendJson(res, 403, { error: "forbidden", message: "project owner cannot be removed" });
        return;
      }
      state.collaborators.get(id)?.delete(userId);
      sendEmpty(res, 204);
      return;
    }
    if (projectMatch && parts[4] === "versions" && parts.length === 5 && method === "GET") {
      const id = Number(parts[3]);
      if (!state.projects.some((item) => item.project_id === id)) {
        sendJson(res, 404, { error: "not_found", message: "project not found" });
        return;
      }
      sendJson(res, 200, state.versions.filter((item) => item.project_id === id));
      return;
    }
    if (projectMatch && parts[4] === "versions" && parts.length === 7 && parts[6] === "download" && method === "GET") {
      const projectId = Number(parts[3]);
      const version = parts[5];
      const query = new URL(req.url ?? "/", "http://stub").searchParams;
      const app = query.get("app") ?? "";
      if (!state.projects.some((item) => item.project_id === projectId)) {
        sendJson(res, 404, { error: "not_found", message: "project not found" });
        return;
      }
      if (!bearer(req)) {
        sendJson(res, 401, { error: "unauthorized", message: "login required for private project" });
        return;
      }
      const body = new TextEncoder().encode(`archive-${projectId}-${version}-${app}`);
      res.writeHead(200, {
        "content-type": "application/gzip",
        "content-disposition": `attachment; filename="${projectId}-${version}-${app}.tar.gz"`,
      });
      res.end(body);
      return;
    }
    if (parts[2] === "tokens" && parts.length === 3) {
      if (method === "POST") {
        const body = (await readJson(req)) ?? {};
        const token: StubToken = {
          token_id: state.nextTokenId++,
          name: String(body.name ?? "token"),
          project_scope: body.project_scope ?? "All",
          scopes: Array.isArray(body.scopes) ? (body.scopes as string[]) : [],
          created_at: "2026-08-20T00:00:00Z",
          updated_at: "2026-08-20T00:00:00Z",
          revoked: false,
        };
        state.tokens.set(token.token_id, token);
        sendJson(res, 201, {
          token_id: token.token_id,
          jwt: `jwt-${token.token_id}`,
          name: token.name,
          expires_at: body.expires_at ?? null,
        });
        return;
      }
      if (method === "GET") {
        const list = Array.from(state.tokens.values())
          .filter((item) => !item.revoked)
          .map(({ token_id, name, project_scope, scopes, created_at, updated_at }) => ({
            token_id,
            name,
            project_scope,
            scopes,
            created_at,
            updated_at,
          }));
        sendJson(res, 200, list);
        return;
      }
    }
    if (parts[2] === "tokens" && /^\d+$/.test(parts[3] ?? "")) {
      const id = Number(parts[3]);
      const token = state.tokens.get(id);
      if (!token) {
        sendJson(res, 404, { error: "not_found", message: "token not found" });
        return;
      }
      if (method === "DELETE") {
        token.revoked = true;
        sendEmpty(res, 204);
        return;
      }
      if (method === "POST" && parts[4] === "rotate") {
        sendJson(res, 200, { token_id: id, jwt: `jwt-rotated-${id}`, name: token.name, expires_at: null });
        return;
      }
      if (method === "POST") {
        const body = (await readJson(req)) ?? {};
        if (Array.isArray(body.scopes)) {
          token.scopes = body.scopes as string[];
        }
        if (body.project_scope !== undefined) {
          token.project_scope = body.project_scope;
        }
        if (typeof body.name === "string") {
          token.name = body.name;
        }
        sendJson(res, 200, {
          token_id: id,
          name: token.name,
          project_scope: token.project_scope,
          scopes: token.scopes,
          created_at: token.created_at,
          updated_at: token.updated_at,
        });
        return;
      }
    }
    sendJson(res, 404, { error: "not_found", message: `route not found: ${method} ${url.pathname}` });
    return;
  }

  sendJson(res, 404, { error: "not_found", message: "unknown route" });
}

let server: Server;
let base: string;
let client: ApiClient;
let session: string;

beforeAll(async () => {
  server = createServer((req, res) => {
    void handle(req, res).catch((error: unknown) => {
      res.writeHead(500, { "content-type": "application/json" });
      res.end(JSON.stringify({ error: "server_error", message: String(error) }));
    });
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const port = (server.address() as AddressInfo).port;
  base = `http://127.0.0.1:${port}`;
  client = new ApiClient({ baseUrl: base });
  session = (await client.login("alice", "pw")).session;
});

afterAll(async () => {
  await new Promise((resolve) => server.close(() => resolve(undefined)));
});

describe("v1 contract integration (契约桩)", () => {
  it("unwraps login envelope and rejects HTTP-200 envelope failures", async () => {
    expect(session.startsWith("S-")).toBe(true);
    await expect(client.login("alice", "wrong")).rejects.toMatchObject({
      kind: "auth",
      message: "用户名或密码错误",
    });
  });

  it("supports project list, POST visibility update, and delete", async () => {
    const projects = await client.listProjects(session);
    expect(projects[0]?.name).toBe("demo");
    const updated = await client.setVisibility(session, 1, "public");
    expect(updated.visibility).toBe("public");
    const created = await client.createProject(session, "tmp", "private");
    expect(created.project_id).toBeGreaterThan(1);
    await client.deleteProject(session, created.project_id);
    const after = await client.listProjects(session);
    expect(after.some((item) => item.project_id === created.project_id)).toBe(false);
  });

  it("pages project list with limit/offset and the X-Total-Count header", async () => {
    for (let i = 0; i < 4; i++) {
      await client.createProject(session, `tmp-${i}`, "private");
    }
    const first = await client.listProjectsPage(session, { limit: 2, offset: 0 });
    expect(first.items.map((item) => item.name)).toEqual(["demo", "tmp-0"]);
    expect(first.total).toBe(5);
    const second = await client.listProjectsPage(session, { limit: 2, offset: 2 });
    expect(second.items.map((item) => item.name)).toEqual(["tmp-1", "tmp-2"]);
    expect(second.total).toBe(5);
  });

  it("gets a single project by id and collects all pages into the full list", async () => {
    const created = await client.createProject(session, "page-target", "private");
    const direct = await client.getProject(session, created.project_id);
    expect(direct.name).toBe("page-target");

    const all = await client.listAllProjects(session, 2);
    expect(all.length).toBeGreaterThanOrEqual(5);
    expect(all.some((item) => item.project_id === created.project_id)).toBe(true);

    await expect(client.getProject(session, 9999)).rejects.toMatchObject({
      kind: "not_found",
      status: 404,
    });
  });

  it("uploads project_scope as server JSON and keeps token list free of expiry", async () => {
    const issued = await client.createToken(session, {
      name: "deploy",
      project_scope: { Specified: [1] },
      scopes: ["metadata:read", "artifacts:read"],
    });
    expect(issued.jwt.startsWith("jwt-")).toBe(true);
    const createdTokenId = issued.token_id;
    const list = await client.listTokens(session);
    expect(list.some((item) => item.token_id === createdTokenId)).toBe(true);
    const item = list.find((entry) => entry.token_id === createdTokenId);
    expect(item?.project_scope).toEqual({ Specified: [1] });
    expect(Object.keys(item ?? {})).not.toContain("expires_at");
  });

  it("attribute updates return TokenSummary; only rotate (re-sign) returns TokenIssued", async () => {
    const summary = await client.updateToken(session, 1, { name: "renamed" });
    expect("jwt" in summary).toBe(false);
    expect(summary.name).toBe("renamed");
    const updated = await client.updateToken(session, 1, { scopes: ["artifacts:read"] });
    expect("jwt" in updated).toBe(false);
    expect(updated.scopes).toEqual(["artifacts:read"]);
    const rotated = await client.rotateToken(session, 1);
    expect(rotated.jwt).toContain("rotated");
    await client.revokeToken(session, 1);
    const list = await client.listTokens(session);
    expect(list.some((item) => item.token_id === 1)).toBe(false);
  });

  it("manages collaborators by user_id with upsert semantics and owner 403", async () => {
    await client.setCollaborator(session, 1, 2, "read");
    await client.setCollaborator(session, 1, 2, "write");
    let list = await client.listCollaborators(session, 1);
    expect(list).toEqual([{ user_id: 2, role: "write" }]);
    await client.removeCollaborator(session, 1, 2);
    list = await client.listCollaborators(session, 1);
    expect(list).toEqual([]);
    await expect(client.setCollaborator(session, 1, 1, "admin")).rejects.toMatchObject({
      kind: "forbidden",
      status: 403,
    });
  });

  it("downloads with Bearer authorization and rejects anonymous private access", async () => {
    const anonClient = new ApiClient({ baseUrl: base });
    await expect(anonClient.download(null, 1, "1.0.0", "default")).rejects.toMatchObject({
      kind: "auth",
      status: 401,
    });
    const blob = await client.download(session, 1, "1.0.0", "default");
    const bytes = new Uint8Array(await blob.arrayBuffer());
    expect(new TextDecoder().decode(bytes)).toBe("archive-1-1.0.0-default");
    expect(state.seenAuthHeaders.some((header) => header.startsWith("Bearer S-"))).toBe(true);
  });

  it("maps unknown routes to not_found errors", async () => {
    await expect(client.listVersions(session, 404)).rejects.toMatchObject({
      kind: "not_found",
      status: 404,
    });
  });
});
