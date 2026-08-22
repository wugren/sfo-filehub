# filehub v1 API 契约（002-web / 003-cli 唯一契约源）

本文档由 `design/http.md` 冻结路由表落地。认证凭据一律经 `Authorization: Bearer <session|refresh|token-jwt>` 头传输，不使用 cookie。错误统一为 JSON `{"error": "<code>", "message": "<text>"}`：

- 401 `unauthorized`：无凭据/凭据无效/Anonymous 访问 private
- 403 `forbidden`：已认证但越权
- 404 `not_found`：不存在或对该身份不可见
- 409 `conflict`：版本/项目名已存在
- 422 `invalid_input`：参数非法、非 `.tar.gz`、超限、非法过期参数
- 5xx：落库/存储/服务器错误

实现说明（2026-08-19）：`sfo-http` 0.7 Actix 后端未注册 PATCH 路由，本实现将两个「更新语义」端点以 **POST** 提供，语义与冻结表的 PATCH 完全一致（web/CLI 按 POST 调用；后端 PATCH 支持修复后自动对齐，无需改客户端参数）。

## 账号与会话（sfo-account 直接导出）

| method | path | 认证 | 备注 |
|--------|------|------|------|
| POST | `/account/login` | 匿名 | body `{user_name,password,timestamp}`；响应 sfo-http 包装 `{err:0,result:{session,refresh_session}}` |
| POST | `/account/get_account_info_of_session` | session | body `{session}` |
| GET | `/account/get_account_info` | session | 获取当前账号 |
| POST | `/account/refresh_session` | refresh session | 返回新 session/refresh_session |

## /api/v1 路由

| method | path | 认证 | action | 成功响应 |
|--------|------|------|--------|----------|
| GET | `/api/v1/projects` | 匿名/session/token | 列表按可见性过滤 | 200 `Project[]` |
| POST | `/api/v1/projects` | session/token | `projects:create` | 201 `Project` |
| GET | `/api/v1/projects/{id}` | 匿名(public)/session/token | `metadata:read` | 200 `Project` / 404(已认证) / 401(匿名 private) |
| POST | `/api/v1/projects/{id}/visibility` | session/token | `administration` | 200 `Project`（POST 承载 PATCH 语义） |
| DELETE | `/api/v1/projects/{id}` | session/token | `projects:delete` | 204 |
| GET | `/api/v1/projects/{id}/collaborators` | session/token | `administration` | 200 `Collaborator[]` |
| PUT | `/api/v1/projects/{id}/collaborators/{user_id}` | session/token | `administration` | 200 `Collaborator` |
| DELETE | `/api/v1/projects/{id}/collaborators/{user_id}` | session/token | `administration` | 204 |
| POST | `/api/v1/tokens` | session | 创建 | 201 `TokenIssued`（jwt 仅此一次） |
| GET | `/api/v1/tokens` | session | 列表 | 200 `TokenSummary[]`（无过期字段） |
| POST | `/api/v1/tokens/{id}` | session | 属性修改（POST 承载 PATCH 语义） | 200 `TokenIssued`（重签）或 `TokenSummary`（仅 name） |
| POST | `/api/v1/tokens/{id}/rotate` | session | 轮换 | 200 `TokenIssued` |
| DELETE | `/api/v1/tokens/{id}` | session | 撤销 | 204 |
| POST | `/api/v1/projects/{id}/versions` | session/token | `artifacts:write`（JSON body `{"version": "1.0.0"}`；显式创建版本，不接收文件） | 201 `VersionRecord`（`apps:[]`）/ 409 版本已存在 |
| PUT | `/api/v1/projects/{id}/versions/{version}/apps/{app}` | session/token | `artifacts:write`（multipart：`file`(.tar.gz)，可选 `sha256`；app 首次发布创建、重复发布即更新） | 201（新建）/ 200（更新）`VersionRecord`；404 版本不存在；409 版本已锁定 |
| DELETE | `/api/v1/projects/{id}/versions/{version}/apps/{app}` | session/token | `artifacts:write` | 204；404 版本或 app 不存在；409 版本已锁定 |
| PUT | `/api/v1/projects/{id}/versions/{version}/lock` | session/token | `administration`（不可逆锁定，重复锁定幂等） | 200 `VersionRecord`（`locked_at` 非空）/ 404 |
| GET | `/api/v1/projects/{id}/versions` | 匿名(public)/session/token | `metadata:read` | 200 `VersionRecord[]` |
| GET | `/api/v1/projects/{id}/versions/{version}` | 匿名(public)/session/token | `metadata:read` | 200 `VersionRecord`（含全部 `apps`）/ 404；`latest` 关键字取最近创建的版本 |
| GET | `/api/v1/projects/{id}/versions/{version}/download?app={app}` | 匿名(public)/session/token | `artifacts:read` | 200 `.tar.gz` 流（`Content-Disposition: attachment; filename="{id}-{version}-{app}.tar.gz"`）；`latest` 同上；缺省 `app` 时单应用版本兼容下载、多应用版本 422、空版本 404 |
| GET | `/api/v1/projects/{id}/versions/latest` 、`/download` | 同上 | 同上 | 路由由 `{version}=latest` 提供同一语义 |

## 数据形状（简）

- `Project`：`{project_id,name,visibility:"public|private",owner}`
- `Collaborator`：`{user_id,role:"read|write|admin"}`
- `AppRecord`：`{app,file_id,sha256,size,updated_at}`
- `VersionRecord`：`{project_id,version,published_at,locked_at:null|ISO,apps:AppRecord[]}`
- `TokenIssued`：`{token_id,jwt,name,expires_at|null}` —— `expires_at` 只出现在创建/属性修改/轮换响应
- `TokenSummary`：`{token_id,name,project_scope,scopes,created_at,updated_at}` —— 无过期字段

## 消费对齐

- 002-web：projects/collaborators/tokens/versions 读接口、token 创建响应；token 列表不展示过期时间，与 tokens.md 契约定位一致。
- 003-cli：登录、显式创建版本（`new-version`）、按 app 发布/更新（`publish --app`）、按 app 下载（`.tar.gz` 流、按所选 app 的 SHA-256 校验）、锁定（`lock-version`）、删除 app（`delete-app`）与版本列表。

## v1 破缺说明（2026-08-21）

- `POST /versions` 从「multipart 发布（隐式建版本）」改为「JSON 显式创建版本」；发布/更新 app 改走 `PUT /versions/{version}/apps/{app}`。CLI 与 admin-web 同批迁移，不保留旧调用兼容层。
