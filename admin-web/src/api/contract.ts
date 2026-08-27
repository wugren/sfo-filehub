// filehub v1 契约 DTO 与序列化辅助（与 docs/api/v1-contract.md 对齐）。

export type Visibility = "public" | "private";
export type ProjectRole = "read" | "write" | "admin";

export type Scope =
  | "metadata:read"
  | "artifacts:read"
  | "artifacts:write"
  | "administration"
  | "projects:create"
  | "projects:delete";

export const ALL_SCOPES: readonly Scope[] = [
  "metadata:read",
  "artifacts:read",
  "artifacts:write",
  "administration",
  "projects:create",
  "projects:delete",
];

// 服务端 serde 外部标签枚举：ProjectScope { All, Specified(Vec<ProjectId>) }。
export type ProjectScopeDto = "All" | { Specified: number[] };

export interface LoginRequest {
  user_name: string;
  password: string;
  timestamp: number;
}

export interface LoginResult {
  session: string;
  refresh_session: string;
}

export interface CurrentUser {
  id: number;
  name: string;
}

export interface Project {
  project_id: number;
  name: string;
  visibility: Visibility;
  owner: number;
}

export interface ProjectPage {
  items: Project[];
  total: number;
}

export interface Collaborator {
  user_id: number;
  role: ProjectRole;
}

export interface AppRecord {
  app: string;
  file_id: string;
  sha256: string;
  size: number;
  updated_at: string;
}

export interface TokenSummary {
  token_id: number;
  name: string;
  project_scope: ProjectScopeDto;
  scopes: Scope[];
  created_at: string;
  updated_at: string;
}

export interface TokenIssued {
  token_id: number;
  jwt: string;
  name: string;
  expires_at: string | null;
}

export interface TokenCreateInput {
  name: string;
  project_scope?: ProjectScopeDto;
  scopes?: Scope[];
  expires_at?: string | null;
}

export interface TokenUpdateInput {
  name?: string;
  project_scope?: ProjectScopeDto;
  scopes?: Scope[];
}

export interface VersionRecord {
  project_id: number;
  version: string;
  published_at: string;
  locked_at: string | null;
  apps: AppRecord[];
}

export function encodeProjectScope(scope: "all" | number[]): ProjectScopeDto {
  return scope === "all" ? "All" : { Specified: [...scope] };
}

export function describeProjectScope(dto: ProjectScopeDto): string {
  if (dto === "All") {
    return "全部项目";
  }
  if (dto.Specified.length === 0) {
    return "无";
  }
  return `指定项目：${dto.Specified.join(", ")}`;
}

export function formatBytes(size: number): string {
  if (!Number.isFinite(size) || size < 0) {
    return `${size}`;
  }
  if (size < 1024) {
    return `${size} B`;
  }
  const units = ["KB", "MB", "GB", "TB"];
  let value = size;
  let unit = "B";
  for (const candidate of units) {
    value = value / 1024;
    unit = candidate;
    if (value < 1024) {
      break;
    }
  }
  return `${value.toFixed(value >= 100 ? 0 : 1)} ${unit}`;
}

export function formatTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return date.toLocaleString("zh-CN", { hour12: false });
}
