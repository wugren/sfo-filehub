# filehub（文件集散）模块边界

filehub 是文件集散产品：用户发布 `.tar.gz` 版本产物，管理后台查看/下载，发布客户端命令行发布。产品层分为三个交付面，均属同一生产模块 `filehub`：

1. `filehub-server`（服务后台，crate `server/`）：认证、授权、项目、版本与文件 API，本文件以下边界即指该 crate。
2. `filehub-web`（管理后台，`admin-web/`，归属 `002-filehub-web` 任务）：独立静态站点，只消费 API。
3. `filehub-cli`（发布客户端，`cli/`，归属 `003-filehub-cli` 任务）：命令行发布/下载/查询，只消费 API。

三个交付面共用一份 v1 API 契约（`docs/api/v1-contract.md`），模块间只通过公开接口交互。

## filehub-server crate 子模块

`filehub-server` 是单一 Rust crate（`server/`），全部实现子模块是 crate 内的子 mod：

| 子模块 | 目录 | 职责 |
|--------|------|------|
| account | `server/src/account/` | 配置初始化适配与 `sfo-account` 装配；直接导出 `sfo-account` 的 HTTP 接口（`AccountServer::register_server`：登录/会话信息/refresh 路由）；认证中间件复用 `decode_session`；不自建 SessionService 与 session 生命周期管理；`Authorization` 头，不用 cookie |
| permissions | `server/src/permissions/` | 权限数据存储与校验服务；账号/项目协作角色与统一访问判定 |
| tokens | `server/src/tokens/` | JWT 形态 token 生命周期（创建/列表/修改/轮换/撤销）、权限数据与验签公钥（签名私钥即弃，不落库）、过期仅由 JWT exp 承载（token 本身无过期）、token session 与用户登录 session 凭据类型区分 |
| files | `server/src/storage/` | `.tar.gz` 物理存储、原子写入、SHA-256、下载流与路径防穿越 |
| versions | `server/src/versions/` | 版本显式创建与不可逆锁定、版本内具名 app 的发布/更新/删除、latest 语义、版本-app 关联与原子发布协调 |
| projects | `server/src/projects/` | 项目 CRUD 与 public/private 可见性 |
| http | `server/src/http/` + `server/src/contract/` | `/api/v1` 路由/DTO/错误映射与服务装配 |
| model（共享） | `server/src/model/` | 跨模块标识/角色/权限枚举/Principal/Resource/记录与配置 DTO；无持久化状态 |

依赖方向（无环）：`versions -> files`、`projects -> versions`、`permissions/tokens -> account`、`tokens/projects/versions -> permissions`；各履职子模块与 `http` 共享 `model` 值类型（model 不依赖任何业务子模块）；`http` 为唯一装配层，业务子模块不反向依赖 http。

## filehub-cli crate 子模块

`filehub-cli` 是单一 Rust crate（`cli/`，归属 `003-filehub-cli` 任务），二进制目标 `filehub-cli`；全部实现子模块是该 crate 内的子 mod：

| 子模块 | 目录 | 职责 |
|--------|------|------|
| cli（装配） | `cli/src/cli/` | 命令解析（clap）、参数/环境变量/交互、push/pull 等命令 handler 编排、输出与稳定退出码 |
| apiclient（技术） | `cli/src/apiclient/` | v1 API 契约 DTO 与 HTTP 传输、Bearer 注入、session 401 续期重试（refresh）一次、错误分类 |
| credential_store（技术） | `cli/src/credential_store/` | 本地凭据/配置文件（原子写、最小权限）、多服务器凭据、token > session 复用、login 覆盖与 logout 清除 |
| archive（技术） | `cli/src/archive/` | 安全 `.tar.gz` 打包（排除绝对路径/越界符号链接）、SHA-256、下载文件名净化与校验后原子落盘 |

依赖方向（无环）：`cli -> apiclient / credential_store / archive`、`apiclient -> credential_store`；技术子模块不依赖装配层；CLI 与服务端只经冻结契约 `docs/api/v1-contract.md` 交互。

## filehub-cli 边界与契约

- Owner: filehub-cli crate（当前活动任务 `003-filehub-cli`，设计见其任务包 `design.md` 与 `design/` 子文档）
- Inputs: `filehub login/logout/push/pull/versions/new-version/lock-version/delete-app` 命令、stdin/环境变量凭据输入、本地文件或目录、目标文件路径/输出路径、v1 API 响应
- Outputs: 二进制 `filehub-cli`、本地凭据配置文件（类 Unix `0600`）、push/pull/版本输出与稳定退出码
- Contracts: CLI 命令面、参数与退出码冻结于任务提案与 `design/cli.md`；与 `docs/api/v1-contract.md` 对齐
- Dependencies: `sfo-log`（日志统一）、reqwest-rustls、clap、tar/flate2、sha2、serde/serde_json、toml、dirs（来源与版本经 Cargo.lock 锁定）
- 凭据安全约束：密码/token/session 明文不进入命令行参数与日志；token > session 复用；凭据文件原子写且权限最小化

## filehub-web 模块结构（admin-web）

`filehub-web` 是独立前端交付（`admin-web/`，归属 `002-filehub-web` 任务），Vite + React + TypeScript 单页应用，只消费 v1 API 契约，不实现任何服务端能力：

| 子模块 | 目录 | 职责 |
|--------|------|------|
| api-client | `admin-web/src/api/` | v1 DTO/URL 装配、sfo-account 包装与 `/api/v1` 错误体两套适配、Bearer 注入、下载 blob |
| session | `admin-web/src/api/session.ts` + `components/ProtectedRoute.tsx` + `pages/LoginPage.tsx` | 登录、会话状态、401 refresh 续期一次、本地登出（server 无登出端点） |
| projects | `admin-web/src/pages/ProjectsPage.tsx`、`ProjectDetailPage.tsx` | 可见项目列表/创建/删除/可见性切换、版本显式创建/锁定、按应用上传/更新/删除与下载 |
| tokens | `admin-web/src/pages/TokensPage.tsx` | token 创建/列表/修改(重签)/轮换/撤销与 JWT 一次性明文展示 |
| collaborators | `admin-web/src/pages/MembersPage.tsx` | 按数字 user_id 查看/添加/改级/移除项目协作者 |
| build | `admin-web/` 工程配置 | Vite 构建、`admin-web/dist` 独立静态交付、`VITE_API_BASE_URL` 指向服务后台 |

依赖方向（无环）：页面 -> api-client / session；session -> api-client。页面不做权限本地放行；会话凭据仅内存/sessionStorage，JWT 明文仅签发响应展示一次。

## 边界与契约

## 边界与契约

- Owner: filehub-server（当前活动任务 `001-filehub-core-platform`）
- Level: unit（crate 内单元层为基线；dv/integration 层级随测试阶段落地）
- Inputs: 配置文件（`[users]` 账号与角色、`[server]` sfo-http 监听/CORS（HTTPS 由前置反向代理终结）、`[files]` `data_dir`/归档上限）、`.tar.gz` 上传流（仅 `.tar.gz`）、API 请求
- Outputs: v1 HTTP API、版本与下载、API 契约文档
- Contracts: `/api/v1/*`（认证、项目、版本、下载、token、协作者），公开契约先冻结再实现
- Dependencies: `sfo-account`（账号/会话）、`sfo-http`（HTTP 服务器与全部 HTTP 接口）、`sfo-log`（日志）；来源与版本在设计阶段锁定
- 接口约定（服务端子模块）：可能执行 IO（SQLite、物理文件、验签/密钥读取等）的 trait 接口方法一律声明为 async fn；纯计算保持同步；account 无自建 trait，其初始化方法同样为 async
- Current/Active Task: 无（`002-filehub-web` 已完成并移出任务索引；`001-filehub-core-platform`、`003-filehub-cli` 尚在验收收尾）

## 变更规则

- 本文件只在该模块边界、合约或活动任务变化时更新；任务包文档保持在 `docs/versions/v0.1/modules/filehub/<task-seq>-<task-slug>/`。
