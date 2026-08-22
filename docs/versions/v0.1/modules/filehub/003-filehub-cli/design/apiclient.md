---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-20
approved_content_sha256: 871d9258476244dca2e68daff46bd7fe50d2d3c23b7889ef4f78da1705b2581a
---

## Approval Record

- approver: user
- approval_date: 2026-08-20
- user_statement: 自动完成003任务吧


# apiclient 子模块设计（v1 API 传输）

## Responsibility

- 实现 filehub v1 契约的客户端侧：DTO、HTTP 传输、Bearer 注入、401 续期重试（session 场景）、错误分类与上传/下载流。
- 无持久状态；不解析 CLI 参数；不决定业务语义（发布/下载/查询编排在 cli 子模块）。

## Interfaces

```rust
// cli/src/apiclient/mod.rs（fh-cli-login / fh-cli-publish / fh-cli-download / fh-cli-versions）
pub struct Config { pub base_url: String, pub timeout: Duration, pub connect_timeout: Duration }
pub struct FilehubClient { cfg: Config, http: reqwest::Client }
impl FilehubClient {
    pub fn new(cfg: Config) -> Result<Self, ClientError>;        // 身份 host[:port] -> HTTPS 优先候选 + loopback HTTP 降级
    pub async fn login_password(&self, user: &str, password: &str) -> Result<LoginResp, ClientError>;
    pub async fn refresh_session(&self, refresh: &str) -> Result<LoginResp, ClientError>;
    pub async fn list_projects(&self, bearer: &str) -> Result<Vec<ProjectDto>, ClientError>;
    pub async fn resolve_project(&self, bearer: &str, name: &str) -> Result<ProjectDto, ClientError>;
    pub async fn publish(&self, bearer: &str, project_id: &str, version: &str, archive: &Path, sha256: &str) -> Result<VersionDto, ClientError>;
    pub async fn get_version(&self, bearer: &str, project_id: &str, version: Option<&str>) -> Result<VersionDto, ClientError>;
    pub async fn download(&self, bearer: &str, project_id: &str, version: Option<&str>, tmp: &Path) -> Result<(), ClientError>;
    pub async fn list_versions(&self, bearer: &str, project_id: &str) -> Result<Vec<VersionDto>, ClientError>;
}
pub struct AuthClient { transport: FilehubClient, store: Arc<RwLock<CredentialStore>> }
impl AuthClient {
    pub async fn prepare(&self, server: Option<&str>) -> Result<Prepared, ClientError>;
        // current_credential -> Bearer；401 + refresh_session -> refresh_session() -> store.update_session -> 重试一次
}
```

```rust
// cli/src/apiclient/contract.rs：契约 DTO（与 docs/api/v1-contract.md 数据形状对齐）
pub struct LoginResp { pub session: String, pub refresh_session: String }
pub struct ProjectDto { pub project_id: String, pub name: String, pub visibility: String, pub owner: String }
pub struct VersionDto { pub project_id: String, pub version: String, pub file_id: String, pub sha256: String, pub size: u64, pub published_at: String }
pub struct ApiError { pub code: String, pub message: String }  // {"error","message"} 统一错误体
```

## Contract Behavior

- `/account/login`：body `{user_name,password,timestamp}`（timestamp 为客户端当前 unix 秒）；响应为 sfo-http 包装 `{err:0,result:{session,refresh_session}}`，`err != 0` 或解析失败按认证/传输错误分类。
- `/account/refresh_session`：`Authorization: Bearer <refresh_session>`；成功返回新 `{session,refresh_session}`。
- `GET /api/v1/projects`：可用作 token 有效性与项目名解析；token 无项目可见性时仍为有效认证（列表可空）。
- 发布：`POST /api/v1/projects/{id}/versions`，multipart 字段 `version`、`file`（`.tar.gz` 字节）、`sha256`；201 返回 `VersionDto`；409 为终态冲突，不重试不重传。
- 下载：`GET /api/v1/projects/{id}/versions/{version|latest}/download` 流式写入临时文件；先经 `get_version` 取得 sha256 再下载。
- 错误映射：401 -> Auth、403 -> Forbidden、404 -> NotFound、409 -> Conflict、422 -> InvalidInput、5xx/超时/连接/TLS -> Transport；`{"error":code,"message"}` 解析失败时按传输类处理并把原始状态码附入错误信息（不含 body 明文以外内容）。
- 超时：connect_timeout 与服务 timeout 由 `Config` 定义；4xx 不重试；传输/5xx 由调用方按命令语义决定（login 直接失败，publish/download 单次失败返回网络退出码）。

## State and Ownership

- 无持久状态；运行时唯一共享对象是 `reqwest::Client`（连接池）；`AuthClient` 持有的 `CredentialStore` 引用归属 credential_store 模块。

## Design Notes

- TLS 用 rustls 默认根证书；server 参数按身份 `host[:port]` 归一化（无协议/显式协议均落到同一身份）。请求端点按 Docker 语义构造：HTTPS 优先；`localhost`/`127.0.0.0/8`/`::1` 等 loopback 身份在连接失败时降级尝试 HTTP；非 loopback 只走 HTTPS，不开放任意明文 HTTP。
- 下载流程中的续期重试只作用于同一请求一次；流式响应已开始后不再重试（避免读到部分字节再重发）。
- `Prepared` 返回凭据类型与 Bearer，命令层只透传；任何 DTO/日志输出不得包含凭据字段（session/refresh/token 全部脱敏）。
