// 会话状态：内存为真相，sessionStorage 仅用于刷新后恢复；不写 localStorage。

import { ApiClient } from "./client";
import type { CurrentUser } from "./contract";
import { ApiError } from "./errors";

const SESSION_KEY = "fh_web_session";
const REFRESH_KEY = "fh_web_refresh";
const USER_KEY = "fh_web_user";

export type SessionState = "anonymous" | "authenticated";

function readStorage(key: string): string | null {
  try {
    return sessionStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string | null): void {
  try {
    if (value === null) {
      sessionStorage.removeItem(key);
    } else {
      sessionStorage.setItem(key, value);
    }
  } catch {
    // 隐私/存储不可用时仅保留内存态。
  }
}

export class SessionStore {
  private readonly client: ApiClient;
  private session: string | null = null;
  private refresh: string | null = null;
  private user: CurrentUser | null = null;
  private version = 0;
  private readonly listeners = new Set<() => void>();

  constructor(client: ApiClient) {
    this.client = client;
    this.restore();
  }

  get state(): SessionState {
    return this.session ? "authenticated" : "anonymous";
  }

  get currentUser(): CurrentUser | null {
    return this.user;
  }

  bearer(): string | null {
    return this.session;
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  getSnapshot = (): number => this.version;

  private notify(): void {
    this.version += 1;
    for (const listener of this.listeners) {
      listener();
    }
  }

  private persist(): void {
    if (this.session && this.refresh && this.user) {
      writeStorage(SESSION_KEY, this.session);
      writeStorage(REFRESH_KEY, this.refresh);
      writeStorage(USER_KEY, JSON.stringify(this.user));
    } else {
      writeStorage(SESSION_KEY, null);
      writeStorage(REFRESH_KEY, null);
      writeStorage(USER_KEY, null);
    }
  }

  private restore(): void {
    const session = readStorage(SESSION_KEY);
    const refresh = readStorage(REFRESH_KEY);
    const rawUser = readStorage(USER_KEY);
    if (!session || !refresh || !rawUser) {
      return;
    }
    try {
      this.user = JSON.parse(rawUser) as CurrentUser;
    } catch {
      return;
    }
    this.session = session;
    this.refresh = refresh;
  }

  async login(userName: string, password: string): Promise<CurrentUser> {
    const login = await this.client.login(userName, password);
    const user = await this.client.getAccountInfo(login.session);
    this.session = login.session;
    this.refresh = login.refresh_session;
    this.user = user;
    this.persist();
    this.notify();
    return user;
  }

  async refreshOnce(): Promise<boolean> {
    if (!this.refresh) {
      return false;
    }
    try {
      const renewed = await this.client.refreshSession(this.refresh);
      this.session = renewed.session;
      this.refresh = renewed.refresh_session;
      this.persist();
      this.notify();
      return true;
    } catch {
      this.logout();
      return false;
    }
  }

  logout(): void {
    this.session = null;
    this.refresh = null;
    this.user = null;
    this.persist();
    this.notify();
  }
}

export async function withAuthRetry<T>(
  store: SessionStore,
  run: (bearer: string) => Promise<T>,
): Promise<T> {
  const bearer = store.bearer();
  if (!bearer) {
    throw ApiError.auth("未登录");
  }
  try {
    return await run(bearer);
  } catch (error) {
    if (error instanceof ApiError && error.kind === "auth") {
      const refreshed = await store.refreshOnce();
      if (!refreshed) {
        throw error;
      }
      const newBearer = store.bearer();
      if (!newBearer) {
        throw error;
      }
      return run(newBearer);
    }
    throw error;
  }
}

const baseUrl =
  (import.meta.env.VITE_API_BASE_URL as string | undefined) || "http://127.0.0.1:8080";

export const apiClient = new ApiClient({ baseUrl });
export const sessionStore = new SessionStore(apiClient);
