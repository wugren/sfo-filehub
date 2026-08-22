// 统一错误分类：登录/account 包装错误与 /api/v1 错误体映射到同类 ApiError。

export type ApiErrorKind =
  | "auth"
  | "forbidden"
  | "not_found"
  | "conflict"
  | "invalid_input"
  | "transport";

export class ApiError extends Error {
  readonly kind: ApiErrorKind;
  readonly status: number | null;

  constructor(kind: ApiErrorKind, message: string, status: number | null = null) {
    super(message);
    this.name = "ApiError";
    this.kind = kind;
    this.status = status;
  }

  static auth(message: string): ApiError {
    return new ApiError("auth", message, 401);
  }

  static fromV1(status: number, bodyText: string): ApiError {
    let message = bodyText.trim().slice(0, 240);
    try {
      const parsed = JSON.parse(bodyText) as { error?: string; message?: string };
      if (parsed.message) {
        message = parsed.message;
      }
    } catch {
      // 保持原始文本用于提示。
    }
    let kind: ApiErrorKind;
    switch (status) {
      case 401:
        kind = "auth";
        break;
      case 403:
        kind = "forbidden";
        break;
      case 404:
        kind = "not_found";
        break;
      case 409:
        kind = "conflict";
        break;
      case 422:
        kind = "invalid_input";
        break;
      default:
        kind = "transport";
        message = message || `服务端错误（HTTP ${status}）`;
    }
    return new ApiError(kind, message || `HTTP ${status}`, status);
  }

  static transport(message: string): ApiError {
    return new ApiError("transport", message);
  }
}
