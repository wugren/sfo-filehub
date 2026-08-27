# filehub v1 API 契约（002-web / 003-cli 唯一契约源）

本文档由 `design/http.md` 冻结路由表落地。认证凭据一律经 `Authorization: Bearer <session|refresh|token-jwt>` 头传输，不使用 cookie。错误统一为 JSON `{"error": "<code>", "message": "<text>"}`：

- 401 `unauthorized`：无凭据/凭据无效/Anonymous 访问 private
- 403 `forbidden`：已认证但越权
- 404 `not_found`：不存在或对该身份不可见
- 409 `conflict`：版本/项目名已存在、token 轮换并发冲突
- 422 `invalid_input`：参数非法、缺少/非法/错误的 `sha256`、超限、非法过期参数
- 5xx：落库/存储/服务器错误

上传实现说明（2026-08-24）：`PUT .../apps/{app}` 在读取请求体之前完成
`artifacts:write` 判定（匿名 401、越权 403）；请求体经 sfo-http 0.8 流式读取，
multipart 增量解析，`file` part 边收边写临时文件。`Content-Length` 预检、
`max_archive_bytes` 实时计数超限、缺少/非法 `sha256` 或 `sha256` 不匹配均
返回 422；`sha256` 为上传必填字段，服务端对收到的压缩包字节流式计算
sha256 并核对，不再做 gzip/tar 解压校验；内存占用与归档大小无关。

实现说明（2026-08-19）：`sfo-http` 0.7 Actix 后端未注册 PATCH 路由，本实现将两个「更新语义」端点以 **POST** 提供，语义与冻结表的 PATCH 完全一致（web/CLI 按 POST 调用；后端 PATCH 支持修复后自动对齐，无需改客户端参数）。

## 账号与会话（sfo-account 直接导出）

| method | path | 认证 | 备注 |
|--------|------|------|------|
| POST | `/account/login` | 匿名 | body `{user_name,password,timestamp}`；响应 sfo-http 包装 `{err:0,result:{session,refresh_session}}` |
| POST | `/account/get_account_info_of_session` | session | body `{session}` |
| GET | `/account/get_account_info` | session | 获取当前账号 |
| POST | `/account/refresh_session` | refresh session | 返回新 session/refresh_session |

### 账号与会话：登录失败与限流语义（2026-08-27 起跟随 crates.io sfo-account 0.2.1）

- `/account/login` 失败为 sfo-http 200 信封，按上游 0.2.1 语义区分错误：
  - 未知账号：`err=9`（`InvalidAccount`），消息 `account <user_name> not
    found`（含用户名）；
  - 密码错误：`err=10`（`InvalidPassword`），消息 `Invalid username or
    password`；
  - 两种失败不再执行等成本伪校验，错误码/消息/响应时间可区分账号存在性
    （054 任务用户确认采用上游语义）；成功仍为 `err=0`；
- 登录尝试按来源 IP 限流：应用层默认 30 次/60 秒/IP（`X-Real-IP` →
  `peer_addr` 归一化，配置字段 `login_rate_limit_per_minute` /
  `login_rate_limit_window_secs`，前者 0 表示关闭），超限返回
  `err=11`（`TooManyRequests`）与消息 `Too many login attempts; please try
  again later`；
- docker nginx 对 `location = /account/login` 另设 `limit_req`（5r/s、
  burst 20），超限直接返回 HTTP 429；上传/下载与其它 `/account/`、`/api/v1/`
  路由不受限流影响。

## /api/v1 路由

| method | path | 认证 | action | 成功响应 |
|--------|------|------|--------|----------|
| GET | `/api/v1/projects` | 匿名/session/token | 列表按可见性过滤；可选 `?limit=`（默认 100，上限 500）与 `?offset=`（默认 0），非法参数 422 | 200 `Project[]` + `X-Total-Count` 头（可见项目总数） |
| POST | `/api/v1/projects` | session/token | `projects:create`（任意已登录 session 账号；token 需携带该 scope）；项目名 trim 后非空、不得包含 `/`、不得含首尾空白，非法 422（与 CLI `<server>/<project>` 精确寻址语义一致） | 201 `Project`（创建者即 owner） |
| GET | `/api/v1/projects/{id}` | 匿名(public)/session/token | `metadata:read` | 200 `Project` / 404(已认证) / 401(匿名 private) |
| POST | `/api/v1/projects/{id}/visibility` | session/token | `administration` | 200 `Project`（POST 承载 PATCH 语义） |
| DELETE | `/api/v1/projects/{id}` | session/token | `projects:delete`（项目级：仅项目 owner；token 还需 `administration` scope 且所属用户为项目 owner） | 204 |
| GET | `/api/v1/projects/{id}/collaborators` | session/token | `administration` | 200 `Collaborator[]` |
| PUT | `/api/v1/projects/{id}/collaborators/{user_id}` | session/token | `administration` | 200 `Collaborator` |
| DELETE | `/api/v1/projects/{id}/collaborators/{user_id}` | session/token | `administration` | 204 |
| POST | `/api/v1/tokens` | session | 创建 | 201 `TokenIssued`（jwt 仅此一次） |
| GET | `/api/v1/tokens` | session | 列表 | 200 `TokenSummary[]`（无过期字段） |
| POST | `/api/v1/tokens/{id}` | session | 属性修改（POST 承载 PATCH 语义；name/project_scope/scopes，不重签） | 200 `TokenSummary` |
| POST | `/api/v1/tokens/{id}/rotate` | session | 重新签发（轮换）：换验签公钥并签发新 JWT，旧 JWT 立即失效 | 200 `TokenIssued` / 404（不存在或已撤销）/ 409（并发轮换冲突，请重试） |
| DELETE | `/api/v1/tokens/{id}` | session | 撤销 | 204 |
| POST | `/api/v1/projects/{id}/versions` | session/token | `artifacts:write`（JSON body `{"version": "1.0.0"}`；显式创建版本，不接收文件；版本名 trim 后不得为 `latest`、`.`、`..`，不得包含 `/`、`?`、`#`、`"`、`\` 或控制字符，不限制其它格式） | 201 `VersionRecord`（`apps:[]`）/ 409 版本已存在 / 422 非法版本名 |
| PUT | `/api/v1/projects/{id}/versions/{version}/apps/{app}` | session/token | `artifacts:write`（multipart：`file`(.tar.gz)，必填 `sha256`；app 首次发布创建、重复发布即更新） | 201（新建）/ 200（更新）`VersionRecord`；404 版本不存在；409 版本已锁定 |
| DELETE | `/api/v1/projects/{id}/versions/{version}/apps/{app}` | session/token | `artifacts:write` | 204；404 版本或 app 不存在；409 版本已锁定 |
| PUT | `/api/v1/projects/{id}/versions/{version}/lock` | session/token | `administration`（不可逆锁定，重复锁定幂等） | 200 `VersionRecord`（`locked_at` 非空）/ 404 |
| GET | `/api/v1/projects/{id}/versions` | 匿名(public)/session/token | `metadata:read` | 200 `VersionRecord[]` |
| GET | `/api/v1/projects/{id}/versions/{version}` | 匿名(public)/session/token | `metadata:read` | 200 `VersionRecord`（含全部 `apps`）/ 404；`latest` 关键字取最近创建的版本 |
| GET | `/api/v1/projects/{id}/versions/{version}/download?app={app}` | 匿名(public)/session/token | `artifacts:read` | 200 `.tar.gz` 流（`Content-Disposition: attachment; filename="{id}-{version}-{app}.tar.gz"`，filename 按 RFC 9110 quoted-string 转义 `"`/`\` 并移除控制字符）；`latest` 同上；缺省 `app` 时单应用版本兼容下载、多应用版本 422、空版本 404 |
| GET | `/api/v1/projects/{id}/versions/latest` 、`/download` | 同上 | 同上 | 路由由 `{version}=latest` 提供同一语义 |

## 项目列表分页与单项目直查（2026-08-25）

- 列表接口服务端按同一可见性过滤（public / owner / `project_grants` 协作者 /
  token 项目范围）直接 SQL 分页，不再先取全表再逐项目权限判定；响应体保持
  `Project[]`，可见项目总数经 `X-Total-Count` 响应头返回，客户端可用
  `?limit`/`?offset` 翻页。
- `GET /api/v1/projects/{id}` 直接按目标项目查询，401/404 语义不变：
  匿名访问 private 或不存在返回 401，已认证访问不可见/不存在返回 404。

## 数据形状（简）

- `Project`：`{project_id,name,visibility:"public|private",owner}`
- `Collaborator`：`{user_id,role:"read|write|admin"}`
- `AppRecord`：`{app,file_id,sha256,size,updated_at}`
- `VersionRecord`：`{project_id,version,published_at,locked_at:null|ISO,apps:AppRecord[]}`
- `TokenIssued`：`{token_id,jwt,name,expires_at|null}` —— `expires_at` 只出现在创建/轮换（重新签发）响应
- `TokenSummary`：`{token_id,name,project_scope,scopes,created_at,updated_at}` —— 无过期字段
- `project_scope` 语义：`"All"` 表示 token 所属用户有权操作的全部项目；
  `{"Specified": []}`（空项目集合）与 `"All"` 等价；`{"Specified": [id,...]}`
  仅限集合内项目
- token JWT claims：`data` 载荷仅含 `token_id`/`user_id`（另含服务端签发的
  `sub`/`jti`/`iat`/`exp`）；`scopes` 与 `project_scope` 不放入 JWT，
  授权判定以服务端数据库（`token_scopes`、`tokens.project_scope`）为权威
- token 属性修改语义：`POST /api/v1/tokens/{id}` 只接受
  `name/project_scope/scopes`（全部可选，缺省不修改），只落库且**不重签**；
  已签发 JWT 继续有效（权限变更按数据库立即生效），`exp` 不被触碰。过期
  时间只在创建或显式「重新签发」（`POST /api/v1/tokens/{id}/rotate`）时
  设置/重置；重签默认签发无 `exp` 的新 JWT（不过期）并一次性展示。

## 消费对齐

- 002-web：projects/collaborators/tokens/versions 读接口、token 创建响应；
  项目列表按 `?limit/offset` 分页并读取 `X-Total-Count`；token 列表不展示过期
  时间，与 tokens.md 契约定位一致；项目详情页直接调
  `GET /api/v1/projects/{id}`（不再依赖列表首屏），Token 页以
  `?limit/offset` 循环拉取全部可见项目用于 Specified 范围选择与名称展示。
- 003-cli：登录、显式创建版本（`new-version`）、按 app 发布/更新（`publish --app`）、按 app 下载（`.tar.gz` 流、按所选 app 的 SHA-256 校验）、锁定（`lock-version`）、删除 app（`delete-app`）与版本列表。
  项目列表读取走分页循环（`X-Total-Count` 驱动），按名解析基于全量可见项目，
  第 100 条之后的项目同样可解析；无 `X-Total-Count` 的旧服务端回退单页。

## v1 破缺说明（2026-08-21）

- `POST /versions` 从「multipart 发布（隐式建版本）」改为「JSON 显式创建版本」；发布/更新 app 改走 `PUT /versions/{version}/apps/{app}`。CLI 与 admin-web 同批迁移，不保留旧调用兼容层。
