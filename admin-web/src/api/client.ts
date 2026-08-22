// v1 API 传输层：统一 fetch、Bearer 注入、两套响应/错误适配与下载 blob。

import {
  type Collaborator,
  type CurrentUser,
  type LoginResult,
  type Project,
  type ProjectRole,
  type TokenCreateInput,
  type TokenIssued,
  type TokenSummary,
  type TokenUpdateInput,
  type VersionRecord,
  type Visibility,
} from "./contract";
import { ApiError } from "./errors";

export interface ApiClientConfig {
  baseUrl: string;
  timeoutMs?: number;
}

interface Envelope<T> {
  err: number;
  msg?: string;
  result: T | null;
}

interface RequestOptions {
  bearer?: string | null;
  json?: unknown;
}

export class ApiClient {
  private readonly base: string;
  private readonly timeoutMs: number;

  constructor(config: ApiClientConfig) {
    this.base = config.baseUrl.trim().replace(/\/+$/, "");
    this.timeoutMs = config.timeoutMs ?? 15_000;
  }

  baseUrl(): string {
    return this.base;
  }

  private url(path: string): string {
    return `${this.base}${path}`;
  }

  private async raw(method: string, path: string, options: RequestOptions = {}): Promise<string> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);
    const headers: Record<string, string> = {};
    if (options.json !== undefined) {
      headers["Content-Type"] = "application/json";
    }
    if (options.bearer) {
      headers["Authorization"] = `Bearer ${options.bearer}`;
    }
    let response: Response;
    try {
      response = await fetch(this.url(path), {
        method,
        headers,
        body: options.json !== undefined ? JSON.stringify(options.json) : undefined,
        signal: controller.signal,
      });
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        throw ApiError.transport(`请求超时（${this.timeoutMs}ms）：${method} ${path}`);
      }
      const detail = error instanceof Error ? error.message : String(error);
      throw ApiError.transport(`网络请求失败：${detail}`);
    } finally {
      clearTimeout(timer);
    }
    const bodyText = await response.text();
    if (!response.ok) {
      throw ApiError.fromV1(response.status, bodyText);
    }
    return bodyText;
  }

  private async envelope<T>(method: string, path: string, options: RequestOptions): Promise<T> {
    const bodyText = await this.raw(method, path, options);
    let parsed: Envelope<T>;
    try {
      parsed = JSON.parse(bodyText) as Envelope<T>;
    } catch {
      throw ApiError.transport(`服务响应不是合法 JSON：${path}`);
    }
    if (parsed.err !== 0) {
      throw ApiError.auth(parsed.msg || `服务端返回错误 err=${parsed.err}`);
    }
    if (parsed.result === null || parsed.result === undefined) {
      throw ApiError.transport(`服务响应缺少 result：${path}`);
    }
    return parsed.result;
  }

  private async v1Json<T>(method: string, path: string, options: RequestOptions): Promise<T> {
    const bodyText = await this.raw(method, path, options);
    try {
      return JSON.parse(bodyText) as T;
    } catch {
      throw ApiError.transport(`服务响应不是合法 JSON：${path}`);
    }
  }

  private async v1Empty(method: string, path: string, options: RequestOptions): Promise<void> {
    await this.raw(method, path, options);
  }

  // account 接口（sfo-http 包装 {err,msg,result}，HTTP 200 也可能携带错误）
  async login(userName: string, password: string): Promise<LoginResult> {
    const timestamp = Math.floor(Date.now() / 1000);
    return this.envelope<LoginResult>("POST", "/account/login", {
      json: { user_name: userName, password, timestamp },
    });
  }

  async refreshSession(refresh: string): Promise<LoginResult> {
    return this.envelope<LoginResult>("POST", "/account/refresh_session", { bearer: refresh });
  }

  async getAccountInfo(session: string): Promise<CurrentUser> {
    const data = await this.envelope<CurrentUser & { session_id?: string }>(
      "GET",
      "/account/get_account_info",
      { bearer: session },
    );
    return { id: data.id, name: data.name };
  }

  // /api/v1 接口
  async listProjects(bearer: string): Promise<Project[]> {
    return this.v1Json<Project[]>("GET", "/api/v1/projects", { bearer });
  }

  async createProject(bearer: string, name: string, visibility: Visibility): Promise<Project> {
    return this.v1Json<Project>("POST", "/api/v1/projects", {
      bearer,
      json: { name, visibility },
    });
  }

  async setVisibility(bearer: string, projectId: number, visibility: Visibility): Promise<Project> {
    return this.v1Json<Project>("POST", `/api/v1/projects/${projectId}/visibility`, {
      bearer,
      json: { visibility },
    });
  }

  async deleteProject(bearer: string, projectId: number): Promise<void> {
    await this.v1Empty("DELETE", `/api/v1/projects/${projectId}`, { bearer });
  }

  async listCollaborators(bearer: string, projectId: number): Promise<Collaborator[]> {
    return this.v1Json<Collaborator[]>("GET", `/api/v1/projects/${projectId}/collaborators`, {
      bearer,
    });
  }

  async setCollaborator(
    bearer: string,
    projectId: number,
    userId: number,
    role: ProjectRole,
  ): Promise<Collaborator> {
    return this.v1Json<Collaborator>(
      "PUT",
      `/api/v1/projects/${projectId}/collaborators/${userId}`,
      { bearer, json: { role } },
    );
  }

  async removeCollaborator(bearer: string, projectId: number, userId: number): Promise<void> {
    await this.v1Empty("DELETE", `/api/v1/projects/${projectId}/collaborators/${userId}`, {
      bearer,
    });
  }

  async listTokens(bearer: string): Promise<TokenSummary[]> {
    return this.v1Json<TokenSummary[]>("GET", "/api/v1/tokens", { bearer });
  }

  async createToken(bearer: string, input: TokenCreateInput): Promise<TokenIssued> {
    return this.v1Json<TokenIssued>("POST", "/api/v1/tokens", { bearer, json: input });
  }

  async updateToken(
    bearer: string,
    tokenId: number,
    patch: TokenUpdateInput,
  ): Promise<TokenIssued | TokenSummary> {
    return this.v1Json<TokenIssued | TokenSummary>("POST", `/api/v1/tokens/${tokenId}`, {
      bearer,
      json: patch,
    });
  }

  async rotateToken(bearer: string, tokenId: number): Promise<TokenIssued> {
    return this.v1Json<TokenIssued>("POST", `/api/v1/tokens/${tokenId}/rotate`, { bearer });
  }

  async revokeToken(bearer: string, tokenId: number): Promise<void> {
    await this.v1Empty("DELETE", `/api/v1/tokens/${tokenId}`, { bearer });
  }

  async listVersions(bearer: string | null, projectId: number): Promise<VersionRecord[]> {
    return this.v1Json<VersionRecord[]>("GET", `/api/v1/projects/${projectId}/versions`, {
      bearer,
    });
  }

  async getVersion(bearer: string | null, projectId: number, version: string): Promise<VersionRecord> {
    return this.v1Json<VersionRecord>(
      "GET",
      `/api/v1/projects/${projectId}/versions/${encodeURIComponent(version)}`,
      { bearer },
    );
  }

  async createVersion(bearer: string, projectId: number, version: string): Promise<VersionRecord> {
    return this.v1Json<VersionRecord>("POST", `/api/v1/projects/${projectId}/versions`, {
      bearer,
      json: { version },
    });
  }

  async uploadApp(
    bearer: string,
    projectId: number,
    version: string,
    app: string,
    file: Blob,
    sha256?: string,
  ): Promise<VersionRecord> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);
    const form = new FormData();
    form.append("file", file, file instanceof File ? file.name : "payload.tar.gz");
    if (sha256) {
      form.append("sha256", sha256);
    }
    const headers: Record<string, string> = { Authorization: `Bearer ${bearer}` };
    let response: Response;
    try {
      response = await fetch(
        this.url(
          `/api/v1/projects/${projectId}/versions/${encodeURIComponent(version)}/apps/${encodeURIComponent(app)}`,
        ),
        { method: "PUT", headers, body: form, signal: controller.signal },
      );
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        throw ApiError.transport(`上传超时（${this.timeoutMs}ms）`);
      }
      throw ApiError.transport(`上传请求失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      clearTimeout(timer);
    }
    const bodyText = await response.text();
    if (!response.ok) {
      throw ApiError.fromV1(response.status, bodyText);
    }
    try {
      return JSON.parse(bodyText) as VersionRecord;
    } catch {
      throw ApiError.transport("服务响应不是合法 JSON");
    }
  }

  async deleteApp(
    bearer: string,
    projectId: number,
    version: string,
    app: string,
  ): Promise<void> {
    await this.v1Empty(
      "DELETE",
      `/api/v1/projects/${projectId}/versions/${encodeURIComponent(version)}/apps/${encodeURIComponent(app)}`,
      { bearer },
    );
  }

  async lockVersion(bearer: string, projectId: number, version: string): Promise<VersionRecord> {
    return this.v1Json<VersionRecord>(
      "PUT",
      `/api/v1/projects/${projectId}/versions/${encodeURIComponent(version)}/lock`,
      { bearer },
    );
  }

  async download(
    bearer: string | null,
    projectId: number,
    version: string,
    app: string,
  ): Promise<Blob> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);
    const headers: Record<string, string> = {};
    if (bearer) {
      headers["Authorization"] = `Bearer ${bearer}`;
    }
    let response: Response;
    try {
      response = await fetch(
        this.url(
          `/api/v1/projects/${projectId}/versions/${encodeURIComponent(version)}/download?app=${encodeURIComponent(app)}`,
        ),
        { headers, signal: controller.signal },
      );
    } catch (error) {
      throw ApiError.transport(`下载请求失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      clearTimeout(timer);
    }
    if (!response.ok) {
      const bodyText = await response.text();
      throw ApiError.fromV1(response.status, bodyText);
    }
    return response.blob();
  }
}
