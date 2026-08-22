---
status: approved
task_manifest: task.yaml
---
## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: high-risk
  - 确认记录：用户 2026-08-19 明确确认，保持 high-risk 全流程（详见下方确认记录）。
- 触发边界/理由：服务后台承载账号密码认证、session 会话、带时效与项目级权限的 token 授权、项目 public/private 访问边界、版本与 `.tar.gz` 二进制产物存储/下载、持久化数据 schema，以及对外公开 HTTP API 合约；命中安全边界、持久化 schema、公开协议/API、产物发布与部署面等实质性风险类别，按 high-risk 全流程（提案 -> 设计 -> 实现 -> 测试 -> 验收）执行。
- 拆分说明：本任务由原综合提案 `001-filehub-core-platform` 拆解而来，作为三模块实现中的“服务后台”任务；包编号与路径保持不变（未确认任务不能从 unfinished index 移除，故复用本包并重定义范围）。管理后台页面与发布客户端分别由兄弟任务 `002-filehub-web`、`003-filehub-cli` 承担。
- 子模块拆分说明：用户进一步确认服务后台内部实现拆分为七个子模块——账号模块、权限管理模块、token管理模块、文件管理模块、项目版本信息及其文件模块、项目管理模块、http接口模块；七个提案项（P-01 ~ P-07）与七个子模块一一对应（按依赖顺序排列）。
- 确认记录：2026-08-19 当前用户回复“确认，进入设计阶段吧”，按当前提案确认，最终层级 high-risk，进入设计阶段。
- 修订记录：2026-08-19 当前用户要求「Token 的权限和登录 session 需要能够修改」，属对已确认需求的变更；按任务入口规则收回本提案为 draft。用户随后补充澄清：token 本质是用户分配的一个 JWT，可用于调用服务端各 API，与登录 session 一样；token 在服务端存储时具有签名公私钥、名称、权限范围等属性，且需要能区分 token session 与用户登录 session。修订需求（见 Scope、P-01/P-03 与待确认问题）后需重新获得用户确认；最终层级维持 high-risk（安全边界/持久化 schema/公开 API 契约实质性风险不变）。
- 重新确认记录：2026-08-19 当前用户回复「1.A方案 2.私钥可以放弃，每次重新生成jwt时都生成新的密钥对」：登录 session「修改」按方案 A 确认（账号模块 session 生命周期管理）；token 签名私钥签发后即弃、不落库，每次重签 JWT 时生成新密钥对，服务端仅保存验签公钥。修订后提案按此确认，重新设为 approved，继续设计阶段。
- 修订记录（第二次）：2026-08-19 当前用户澄清「Token 中的过期时间只有生成的 JWT 才有效，token 本身没有过期时间」：token 记录不再存储过期时间，过期仅作为签发 JWT 的 `exp` 声明存在（创建/update/rotate 时写入，最长 1 年或不过期由服务端在签发时校验）；撤销、轮换、重签与旧 JWT 失效语义不变。属已确认需求变更，收回为 draft，待重新确认。
- 第三次确认记录：2026-08-19 当前用户回复「确认」，按第三次修订定稿——过期只存在于生成 JWT 的 `exp` 声明，token 本身无过期时间；重新设为 approved，继续设计阶段。
- 修订记录（第四次）：2026-08-19 当前用户要求「docs/versions/v0.1/modules/filehub/001-filehub-core-platform/design/account.md 中不需要 SessionService，直接导出 sfo-account 中的 http 接口就好」，属对已确认「方案 A：账号模块提供 session 生命周期管理（session 列表、逐会话撤销/登出）」的需求修订；按任务入口规则收回本提案为 draft。P-01 收缩为直接挂载/导出 `sfo-account` 现役 HTTP 接口（`AccountServer::register_server`），不再自定义 `SessionService` 与 session 列表/逐会话撤销/登出路由；最终层级维持 high-risk（安全边界/持久化 schema/公开 API 契约实质性风险不变），修订后需重新获得确认。
- 第四次确认记录：2026-08-19 当前用户回复「确认」：按第四次修订定稿——P-01 直接导出 `sfo-account` 的 HTTP 接口（`AccountServer::register_server`），去掉 `SessionService` 与自建 session 生命周期；认证中间件直接复用 `AccountManager::decode_session`（不保留独立 `JwtSessionVerifier` 设计）；重新设为 approved，继续设计阶段。
- 修订记录（第五次）：2026-08-19 当前用户要求「服务端各个子模块中的trait接口方法可能有io操作的地方都应该是async」，属对已确认设计接口语义的修订：服务端各子模块 trait 接口方法中凡可能执行 IO（SQLite、物理文件、验签/密钥读取等）的一律声明为 `async fn`，纯计算接口保持同步；account 模块不自建 trait，其 `AccountModule::init`（配置与 SQLite 初始化）同步改为 async。按任务入口规则收回本提案为 draft；最终层级维持 high-risk（范围边界与安全/持久化/schema/公开 API 风险评估不变），修订后需重新获得确认。
- 第五次确认记录：2026-08-19 当前用户回复「确认」：按第五次修订定稿——服务端各子模块可能执行 IO 的 trait 接口方法统一声明为 async，account 的 `AccountModule::init` 同步改为 async；重新设为 approved，继续设计阶段。
- 修订记录（第六次）：2026-08-19 当前用户要求「server 的 http 服务器使用 sfo-http 来实现」，属对已确认需求的变更——HTTP 服务器实现由 Axum/Tokio 自建装配改为以 `sfo-http` 为唯一 HTTP 服务器实现（监听、TLS/HTTPS、路由注册、中间件链路均基于 `sfo-http`），原"账号/session 与 token 管理 HTTP 接口拟统一使用 sfo-http"的待确认建议随之定稿；按任务入口规则收回本提案为 draft，修订后需重新获得确认；最终层级维持 high-risk（安全边界/持久化 schema/公开 API 契约/部署面实质性风险不变）。
- 第六次确认记录：2026-08-19 当前用户回复「确认」，按第六次修订定稿——server 的 HTTP 服务器统一使用 `sfo-http` 实现；重新设为 approved，继续设计阶段。
- 实现核验补充（2026-08-19）：`sfo-http` 0.7.0 的 `HttpServerConfig` 提供监听地址/端口与 CORS 配置，服务端实现为 Actix/Tide 两个后端，但服务端自身未提供 TLS 能力；因此 HTTPS 由部署面前置反向代理终结，HTTP 服务器本体仍为 `sfo-http`（与"HTTP 服务器使用 sfo-http"不冲突，不改变本次确认范围）。

## Background and Goal

filehub 是文件集散产品。本任务交付其服务后台（`filehub-server`）：提供用户认证、session 与 token 授权、项目与版本管理、二进制产物发布/下载，以及公开 API 契约，作为页面（`filehub-web`）和发布客户端（`filehub-cli`）的共同后端。

目标产出：

- 配置驱动的账号初始化（用户名/密码，首版无注册）、密码登录与 session 验证/续期（直接导出 `sfo-account` 提供的 HTTP 接口）；
- 带过期时间与项目级权限的 token 授权；
- 项目创建/列表/删除与 public/private 可见性控制；public 项目匿名只读，private 项目强制授权；
- 项目下版本与 `.tar.gz` 产物管理（统一 `.tar.gz` 格式）、版本列表、按版本下载与校验；
- 提供文档化的 v1 API 供 web 与 CLI 使用（不包含前端静态托管）。

## Scope

### In scope

1. 用户认证（服务端）
   - 账号能力复用 `sfo-account` 库（`Account` trait、`DefaultAccountManager` 等）：账号模型、登录与会话/refresh JWT 由该库提供；配置驱动的账号初始化（用户名/密码或密码哈希）按 sfo-account 接口对齐；
   - 账号子模块直接导出 `sfo-account` 的 HTTP 接口（`AccountServer::register_server` 挂载）：`POST /account/login`（用户名/密码登录，返回 session + refresh_session）、`GET /account/get_account_info`（`Authorization: Bearer <session>` 获取当前账号）、`POST /account/get_account_info_of_session`（session 获取当前账号）、`POST /account/refresh_session`（refresh_session 续期）；凭据经 HTTP `Authorization` 头携带，不使用 cookie；不自定义 `SessionService`、session 列表/逐会话撤销/登出路由；
   - 登录 session 定义（本轮澄清）：用户登录后获得的服务端签发 JWT 会话凭据，用于调用服务端 API，凭据类型上与 token 明确区分（见 P-03「区分 token session 与用户登录 session」）；
   - 登录 session 需要能够修改（第四次修订）：按当前用户要求，账号模块不提供自建 session 生命周期管理；session 语义以 `sfo-account` 现役 HTTP 接口为准（登录、账号信息获取、refresh 续期，过期/解码失效），不再新增 session 列表、逐会话撤销、登出接口；
   - 角色模型不属于账号模块：账号角色与项目协作角色由权限管理模块（P-02）定义与维护，授权以项目为载体（首版由配置初始化，无自助注册、无后台建号/重置密码）；
   - 首版不包含自助注册与管理员用户管理。
2. token 授权
   - token 本质（本轮澄清）：token 是用户分配/生成的服务端签发 JWT 凭据，经 HTTP `Authorization: Bearer <token-jwt>` 调用服务端各 API，与登录 session 同为可验证凭据；
   - token 创建/列出/撤销/轮换：创建与轮换时一次性返回 JWT 凭据；服务端存储 token 属性与凭据校验所需密钥材料，不保存可重放的历史明文副本；
   - token 服务端存储属性（本轮确认）：所属用户、名称（name）、权限范围（scope）、验签公钥；签发时临时生成密钥对，私钥签名 JWT 后立即丢弃、不落库，重签/轮换时生成新密钥对并替换验签公钥；
   - 过期时间承载（本次修订）：过期只存在于生成的 JWT 的 `exp` 声明，token 本身没有过期时间；创建/update/rotate 时按请求把过期写入 JWT exp（最长 1 年或不过期由服务端签发时校验，不信任客户端），token 记录不保存过期字段；
   - token 属性修改（本轮修订）：token 创建后允许修改名称、权限数据（项目访问范围、项目级 permissions、账号级权限）等属性；权限修改即时生效，且仍受「token 权限不超过其所属用户权限」二次限制；属性变更涉及重签 JWTs 时，新凭据一次性返回、旧凭据立即使失效；
   - 区分 token session 与用户登录 session（本轮澄清）：认证/授权中间件解析凭据时必须能区分凭据类型——token 凭据解析为 `TokenPrincipal`（token 会话），登录 session 凭据解析为 `UserPrincipal`（用户登录会话）；两类凭据在 claims、服务端存储与授权判定路径上互不混用；
   - 过期策略按用户决定定稿：支持“不过期”或自定义过期且最长 1 年，服务端签发时校验并写入 JWT `exp`，不信任客户端；
   - 权限模型参考 GitHub token（用户已确认）：
     - 访问范围：全部项目，或指定项目列表；
     - 项目级权限：`metadata: read`（基础必需）、`artifacts: read|write`（版本列表/下载与发布/写入版本）、`administration`（项目设置与 public/private 切换）；
     - 账号级权限：`projects: create`、`projects: delete`（参考 GitHub 的创建/删除仓库语义）；
     - 权限与操作的完整映射矩阵在设计阶段定稿。
3. 项目与可见性
   - 项目创建、列表、删除，public/private 切换；
   - public 项目匿名只读（版本列表与下载）；private 项目必须经有效 session 或 token 且具备 read 权限；
   - 所有写操作（上传版本、修改/删除项目、切换可见性）统一走授权中间件。
4. 项目与用户协作授权（GitHub 账户/仓库协作者语义）
   - 项目级协作角色参考 GitHub 仓库访问角色（首版简化为三档）：`read`（查看/下载）、`write`（发布版本/写入产物）、`admin`（项目设置、public/private 切换、协作者管理）；
   - 项目 owner/admin 可把其他配置用户添加/移除为协作者并设置角色；
   - 协作者管理 HTTP 接口属于权限管理模块，使用 `sfo-http` 库实现；
   - 账号级能力参考 GitHub 账户设置：创建项目默认仅 `owner` 账号拥有；删除项目由账号级权限控制；
   - 权限不变量：用户可访问集合 = 账号角色 × 被授权的项目协作角色；token 权限不超过其所属用户本身权限（token 在用户权限之上做二次限制）。
5. 版本与二进制产物
   - 项目下版本元数据（版本号、发布时间、文件大小、SHA-256 等）；
   - 发布形式统一为 `.tar.gz`（单个文件与目录均封装为 `.tar.gz`），不支持其它归档格式；
   - 版本不可覆盖：同一项目下版本号一经发布即视为不可变，重复发布同一版本号被拒绝（HTTP 409），已有版本内容与下载地址不受影响；
   - 下载接口支持省略版本号：省略时下载该项目的最新版本（用户已确认：最新版 = 最近发布的版本，按发布时间倒序取最近一次发布）；
   - 原子发布与下载完整性校验，避免读取半成品版本；下载路径防穿越。
6. 公开 API 与契约
   - 定义并实现 v1 HTTP API（认证、项目、用户授权、版本、下载、token 管理），输出 API 契约文档，供 web 与 CLI 共同消费。
7. 服务端接口并发语义（第五次修订，横切 P-01 ~ P-07）
   - 服务端各子模块 trait 接口方法中可能执行 IO（SQLite、物理文件、验签/密钥读取等）的位置一律声明为 `async fn`，纯内存计算保持同步；
   - account 模块无自建服务 trait，`AccountModule::init`（配置读取与 SQLite 初始化）同样声明为 async，保持 IO 接口语义一致；
   - http 模块公共入口（`register_api`/`extract_bearer`）保持 async；接口签名冻结进入 design.md 与各子模块设计文档。

### Out of scope / non-goals

- 管理后台页面本身的开发与部署/静态托管（归属 `002-filehub-web`，服务后台只提供 API，不托管前端资源）；
- 发布 CLI 客户端（归属 `003-filehub-cli`）；
- 用户自助注册、后台管理员创建用户/重置密码；
- `.tar.gz` 以外的归档或发布格式；
- GitHub Organization/团队层级与邀请流程（只做 GitHub 个人账户/仓库协作者语义，不做组织层级）、源码托管、CI 构建、计费配额、断点续传/分片上传、CDN/对象存储回源；
- 完整合规审计产品（保留基础操作日志，不承诺完整留痕）。

### 相邻边界

- 二进制内容不做杀毒/合规扫描；
- 服务端不提供任何前端静态资源托管或页面路由，web 单独部署并通过配置的 API base URL 访问服务；
- public 项目为匿名只读，任何写操作仍需登录与授权。

## 实现模块拆分（Implementation Module Split）

产品层：本次用户确认将 filehub 实现拆成三个独立任务：

1. `001-filehub-core-platform`（本任务，服务后台 `filehub-server`）：P-01 ~ P-07 七个子模块，外加公开 API 契约；
2. `002-filehub-web`（页面/管理后台）：React 前端页面，独立部署、不依赖服务端静态托管；
3. `003-filehub-cli`（发布客户端）：跨平台 CLI。

三个任务共享一份 API 契约，模块之间只通过公开接口交互。生产模块文档（`docs/modules/` 下三个模块边界）在设计阶段落地。

任务层：用户进一步确认服务后台（本任务）实现时内部拆成如下七个子模块，全部属于单一 `filehub-server` Rust crate，是 crate 内的子 mod；提案项与之一一对应：

| 子模块 | 对应提案项 | 职责 |
|--------|------------|------|
| 账号模块 | P-01 `fh-server-account` | 账号身份、配置驱动初始化、直接导出 `sfo-account` 的登录/session HTTP 接口（`AccountServer::register_server`）；session 凭据（`Authorization` 头传输，不用 cookie） |
| 权限管理模块 | P-02 `fh-server-permissions` | 权限数据存储与校验服务、账号角色/项目协作角色、协作者授权、统一访问判定 |
| token管理模块 | P-03 `fh-server-tokens` | JWT 形态 token 的创建/列表/撤销/轮换、属性（名称/权限/签名密钥）存储与修改、过期策略、token session 与用户登录 session 的凭据类型区分 |
| 文件管理模块 | P-04 `fh-server-files` | `.tar.gz` 物理文件存储、原子写入、SHA-256 校验、下载流与路径防穿越 |
| 项目版本信息及其文件模块 | P-05 `fh-server-versions` | 版本元数据、版本不可覆盖、latest 语义、版本与文件关联及原子发布协调（依赖 P-04） |
| 项目管理模块 | P-06 `fh-server-projects` | 项目创建/列表/删除、public/private 可见性控制（依赖 P-05、P-02） |
| http接口模块 | P-07 `fh-server-http` | `/api/v1` 路由/DTO/错误映射、授权中间件接线、服务装配与 API 契约 |

子模块边界约定：项目版本信息及其文件模块只持有版本记录与文件标识，不直接操作物理字节；文件管理模块只提供文件级能力，不感知版本语义；token管理模块只负责 token 生命周期与权限数据，业务放行判定统一收敛到权限管理模块；http接口模块负责对外装配。

依赖顺序（用户已确认）：项目版本信息及其文件模块依赖文件管理模块；项目管理模块依赖项目版本信息及其文件模块；提案项编号按依赖顺序排列（文件管理 -> 项目版本信息及其文件 -> 项目管理）。

代码归属（用户已确认）：七个实现子模块（账号、权限管理、token管理、文件管理、项目版本信息及其文件、项目管理、http接口）全部位于服务后台的 `filehub-server` crate 内，是其中的子 mod（对应 `server/src/<module>/` 目录），不是独立的 crate、进程、服务或部署单元；http接口模块同样是 crate 内子 mod，对外暴露 v1 API 服务入口。具体 mod 命名与文件布局在设计阶段定稿。

## Requirement Review

需求整体合理，前后端拆分后边界清晰：认证、授权、项目/版本/产物属服务后台；页面与 CLI 只消费公开 API，前端资源由 `002-filehub-web` 独立部署。

关键取舍与建议方向：

- 所有受保护路径共用一个授权中间件与“匿名只读 public / 强制授权 private”的两级判断，避免 public/private 边界出现分支漏洞；
- 会话传输（用户已确认）：session 通过 HTTP `Authorization` 头获取，不使用 cookie；
- 权限模块定位（用户已确认）：权限管理模块主要提供权限数据存储和校验服务，判断用户或 token 是否有权限访问某个功能或数据；业务模块不自行拼装权限判断；
- 子模块依赖顺序（用户已确认）：项目版本信息及其文件模块依赖文件管理模块；项目管理模块依赖项目版本信息及其文件模块；提案项编号与依赖顺序一致（文件管理 -> 项目版本信息及其文件 -> 项目管理）；
- 异步接口约定（本轮用户确认）：服务端各子模块 trait 接口方法中可能执行 IO（SQLite、物理文件、验签/密钥读取等）的一律声明为 `async fn`，纯计算方法保持同步；account 模块无自建 trait，其初始化方法（配置/SQLite）也声明为 async，避免 IO 语义不一致与运行时线程阻塞。
- 代码形态（用户已确认）：服务后台为单一 `filehub-server` Rust crate，七个实现子模块均为 crate 内的子 mod，不拆成独立 crate/进程/服务；
- 账号来自配置而非注册：配置项为 `[users]`（用户名/密码或密码哈希），启动时幂等初始化；
- 公开 API 采用版本化路径（如 `/api/v1/...`），契约先定稿再实现，降低 web/CLI 的并发改动成本；
- token 权限模型按用户确认参考 GitHub token：访问范围选择（全部/指定项目）+ 项目级 permissions（`metadata`/`artifacts`/`administration`）+ 账号级创建/删除项目权限；具体权限常量与操作映射在设计阶段冻结为访问矩阵；
- 用户/账户权限模型按用户确认参考 GitHub 账户权限管理：账号角色 `owner`/`member` + 项目协作角色 `read`/`write`/`admin`（简化自 GitHub 仓库访问角色，去掉 triage/maintain），并明确“token 权限不得超过所属用户权限”的二次限制；角色模型（定义、归属、判定）统一落在权限管理模块，授权以项目为载体，不放入账号模块；
- 技术栈（用户已确认，第六次修订）：Rust + `sfo-http` HTTP 服务器（监听、TLS/HTTPS、路由注册与中间件链路均以 `sfo-http` 实现，不采用 Axum 自建装配）+ SQLite + 本地文件存储（`data_dir` 配置）；管理后台为 React 构建产物（来源见 `002-filehub-web`）；
- 依赖约束（用户已确认，第六次修订）：HTTP 服务器统一使用 `sfo-http`；账号与会话复用 `sfo-account`（v0.2+）；协作者管理 HTTP 接口使用 `sfo-http` 实现；Rust 侧日志统一使用 `sfo-log`；依赖来源与版本（crates.io / Git / 本地路径）在设计阶段锁定。

### 待确认问题（Open questions）

用户已确认：技术栈、发布格式（统一 `.tar.gz`）、token 过期策略（不收紧，最长 1 年 + 可选不过期）、账号来源（配置指定，首版无注册）、三模块独立建任务。

用户已确认的范围调整：服务后台不处理管理后台静态托管，前端由 `002-filehub-web` 独立交付与部署。

用户已确认的实现拆分：服务后台内部按账号模块、权限管理模块、token管理模块、文件管理模块、项目版本信息及其文件模块、项目管理模块、http接口模块七个子模块实现；提案项按此重新组织（P-01 ~ P-07），版本元数据与文件存储职责分离。

用户已确认的子模块依赖顺序：项目版本信息及其文件模块依赖文件管理模块；项目管理模块依赖项目版本信息及其文件模块。

用户已确认的代码形态：七个实现子模块都属于 server crate，是其中的子 mod；不拆成独立 crate、进程或服务。

用户已确认的账号接口：账号子模块对外提供登录接口与 session 验证接口。

用户已确认的会话传输方式：session 通过 HTTP `Authorization` 头获取，不使用 cookie。

用户已确认的权限模块职责：权限管理模块主要提供权限数据存储和校验服务，判断用户或 token 是否有权限访问某个功能或数据。

用户已确认的版本语义：模块版本不可覆盖；同一项目下版本号唯一，重复发布被拒绝。

用户已确认的依赖约束：后台账号使用 `sfo-account` 库实现；权限管理模块的协作者管理 HTTP 接口使用 `sfo-http` 库实现；Rust 项目日志使用 `sfo-log` 库。

原待确认建议（第六次修订定稿）：HTTP 服务器统一由 `sfo-http` 实现后，账号模块的登录/session 验证 HTTP 接口与 token 管理模块的 token 管理 HTTP 接口随之统一使用 `sfo-http`（`sfo-account::AccountServer::register_server` 本身即基于 `sfo-http` 装配），不再作为待确认项。

用户已确认的扩展方向：用户权限参考 GitHub 账户权限管理（账号角色 + 项目协作角色）。本任务的默认细化：账号角色 `owner`/`member`、项目协作角色 `read`/`write`/`admin`，不做 Organization/Team 层级；角色模型（含账号角色）的定义与归属统一放在权限管理模块（P-02），授权以项目为载体，账号模块不承载角色；若用户期望更简单（如仅账号级创建/删除项目、不做项目协作者授权），需在最终确认中说明。

待确认问题（2026-08-19 修订，影响本提案重新确认）：「登录 session 需要能够修改」的具体语义：
- 方案 A（推荐）：session 修改落实为账号模块（P-01）的 session 生命周期管理——如 session 列表、逐会话撤销/登出、TTL/续期（视 `sfo-account` 支撑能力定稿）；tokens.md 边界声明同步为「session 归 P-01」，本提案中 token 权限修改归 P-03 不受影响。
- 方案 B：token 关联/绑定登录 session 且绑定关系可修改（改动 P-03 数据模型与接口，范围更大）。
- 方案 C：仅确认现有「登出/过期使 session 失效」即满足「可修改」，不新增 session 管理接口。

已按用户澄清写入的语义：token 为服务端签发的 JWT 凭据（Bearer 调用 API）；服务端存储 token 属性（名称、权限范围、验签公钥等）；授权/认证须区分 token session（`Principal::Token`）与用户登录 session（`Principal::User`）；token 属性（名称/权限/过期/密钥轮换）可修改。

确认结果（2026-08-19）：「登录 session 修改」按方案 A 定稿——账号模块（P-01）提供 session 生命周期管理（session 列表、逐会话撤销/登出；TTL/续期沿用 `sfo-account` 的 refresh JWT 能力，不新造模型）；tokens.md 边界声明同步为「session 归 P-01」。
方案 A 修订（2026-08-19 第四次）：当前用户要求「design/account.md 中不需要 SessionService，直接导出 sfo-account 中的 http 接口就好」；P-01 下架自定义 `SessionService` 与 session 列表/逐会话撤销/登出，`sfo-account` 现役 HTTP 接口（登录、账号信息获取、refresh 续期，过期/解码失效）即最终 session 语义，待本次重新确认后定稿。

确认结果（2026-08-19）：签名密钥策略定稿——每 token 独立密钥对，签发/重签 JWT 时临时生成，私钥签名后立即丢弃、不落库，服务端仅保存验签公钥；「明文仅一次性返回」相应表述为「不保存可重放历史 JWT 明文、属性变更/轮换重签后新 JWT 一次性返回且旧 JWT 立即失效」。

确认结果（2026-08-19 第三次修订）：「过期时间只有生成的 JWT 才有效，token 本身没有过期时间」——token 记录不保存过期字段，`expires_at` 仅作为签发参数写入 JWT `exp`（最长 1 年或不过期由服务端签发时校验）；`TokenIssued` 返回本次 JWT 的 exp 供客户端参考，`TokenSummary` 不展示过期时间；过期校验只发生在 `resolve` 的 JWT exp 检查上；此语义同步落到 design/tokens.md 与 design.md。

除 CLI 命令面命名（属于 `003-filehub-cli`）外，本任务无剩余未决问题。

## Proposal Items

每个提案项均给出稳定 `proposal_id` 与实现侧 `change_id`，后续设计/测试/验收按 `change_id` 追踪；七个提案项与服务后台七个子模块一一对应。下表为提案项汇总，详细要求见各 P-n 小节。

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-01 | fh-server-account | 账号身份、登录接口、session 验证接口（直接导出 `sfo-account` 的 HTTP 接口）；凭据经 HTTP `Authorization` 头传输，不用 cookie | 只做身份与会话，不承载角色模型 | 直接复用 `sfo-account`，不自建 SessionService 与 session 生命周期路由 | 登录建 session、session 验证/恢复、refresh 续期与过期失效均可用正反例验证 | 不做注册、后台建号、角色定义、自建登出/session 列表/逐会话撤销 |
| P-02 | fh-server-permissions | 权限数据存储与校验服务，判定用户或 token 是否有权访问功能/数据 | 角色与授权关系归属本模块，授权以项目为载体 | 统一授权入口，避免各接口自行拼装权限 | 访问矩阵与越权拒绝可验证 | 不做凭据生命周期（归 P-03） |
| P-03 | fh-server-tokens | JWT 形态 token 的创建/列表/撤销/轮换、属性（名称/权限/验签公钥）修改、过期仅由 JWT exp 承载（token 本身无过期）；区分 token session 与用户登录 session 的凭据类型 | 只做 token 生命周期与权限数据，放行判定归 P-02；登录 session 归 P-01 | token JWT 一次性返回；服务端仅存验签公钥（私钥即弃）；权限修改即时生效；过期不入 token 记录 | 过期（JWT exp）、撤销、轮换、越权拒绝、属性修改即时生效、凭据类型区分且不互混可验证 | 不做业务放行判定、不管理用户登录 session |
| P-04 | fh-server-files | `.tar.gz` 物理存储、原子写入、SHA-256、下载流与路径防穿越 | 只提供文件级能力，不感知版本语义 | 字节内容与文件标识解耦，供版本模块复用 | 发布/下载哈希一致、无半成品文件可验证 | 不做版本与项目语义 |
| P-05 | fh-server-versions | 版本元数据、不可覆盖、latest 语义、版本-文件关联与原子发布协调 | 依赖 P-04，不直接操作物理字节 | 版本记录只引用文件标识，避免职责漂移 | 409 拒绝、latest、原子发布可验证 | 不做文件物理管理 |
| P-06 | fh-server-projects | 项目 CRUD、public/private 可见性控制 | 依赖 P-05 与 P-02，只做项目归属与可见性 | 可见性判定收敛到权限核心，项目模块不重复实现 | 匿名 public 只读、private 强制授权可验证 | 不做版本、文件与权限数据的存储 |
| P-07 | fh-server-http | `/api/v1` 路由/DTO/错误映射、授权中间件接线与服务装配 | 只做对外装配，凭据统一从 HTTP `Authorization` 头读取 | 路由与契约先冻结，降低 002/003 联动成本 | 契约文档与整链路联调可验证 | 不托管前端静态资源、不下沉业务逻辑 |

### P-01 fh-server-account：账号模块

- 账号身份（基于 `sfo-account` 的 `Account` trait、`DefaultAccountManager` 等）：配置驱动初始化（`[users]` 用户名/密码或密码哈希，首版无注册），密码散列存储；
- 对外提供登录与 session HTTP 接口（第四次修订）：直接导出 `sfo-account` 的 HTTP 接口——`AccountServer::register_server` 挂载 `POST /account/login`（登录返回 session + refresh_session）、`GET /account/get_account_info` 与 `POST /account/get_account_info_of_session`（获取当前账号）、`POST /account/refresh_session`（续期）；客户端后续请求通过 HTTP `Authorization` 头携带凭据（如 `Bearer <session-token>`），不使用 cookie；
- session 验证/恢复供授权中间件与前端复用（第四次确认）：直接复用 `sfo-account` 的 `AccountManager::decode_session`，不自定义 `SessionService`，不保留独立 `JwtSessionVerifier` 设计；
- 登录 session 修改（第四次修订）：按当前用户要求收缩——账号模块不提供 session 列表/逐会话撤销/登出等自建生命周期管理，`sfo-account` 已有能力（会话解码、refresh 续期、过期失效）即 P-01 的 session 语义；
- 凭据类型区分（第四次修订）：登录 session JWT 由 `AccountModule::decode_session`（`sfo-account` 解码）校验，token JWT 由 token 模块另行解析/验签，授权中间件以 `Principal::User(...)`（登录 session）与 `Principal::Token(...)`（token session）区分两类凭据且互不混用；
- 登录与 session HTTP 接口属于本模块，但直接由 `sfo-account::AccountServer::register_server` 装配（基于 `sfo-http`），filehub 不自写 session handler；
- 不承载角色模型：账号角色（`owner`/`member`）与项目协作角色（`read`/`write`/`admin`）的定义、归属与判定统一在权限管理模块（P-02）管理，授权以项目为载体。

### P-02 fh-server-permissions：权限管理模块

- 模块定位：主要提供权限数据存储与校验服务——存储账号角色/项目协作角色等授权关系数据，对外提供统一权限判断入口，判定“用户或 token 是否有权限访问某个功能或数据”；
- 统一授权模型与访问矩阵：账号角色（`owner`/`member`）与项目协作角色（`read`/`write`/`admin`）的定义和归属均在本模块，授权以项目为载体，不属于账号模块；访问矩阵 = 账号角色 × 项目协作角色 × token 二次限制；token 权限不超过其所属用户权限（token 权限数据由 P-03 管理）；
- 项目协作角色 `read`/`write`/`admin` 的授权/改级/移除与授权关系数据（GitHub 协作者语义）；协作者管理 HTTP 接口属于本模块，基于 `sfo-http` 库实现；
- 权限判定核心（统一授权中间件/服务），供项目、版本、文件与 http 模块复用；授权变更即时生效。

### P-03 fh-server-tokens：token管理模块

- token 本质与凭据形态（本轮澄清）：token 是服务端签发的 JWT 凭据，经 `Authorization: Bearer` 调用 API；服务端存储 token 记录（所属用户、名称、权限范围、验签公钥），凭据 JWT 创建/轮换时一次性返回，不保存可重放历史明文；
- token 生命周期：创建/列表/撤销/轮换；服务端存储属性含验签公钥（签名私钥签发后即弃、不落库）、名称、权限范围；重签/轮换时生成新密钥对并替换验签公钥，旧签发 JWT 立即失效；
- 过期时间承载（本次修订）：token 本身无过期时间，过期只存在于生成 JWT 的 `exp` 声明；`expires_at` 仅作为签发参数写入 JWT exp，token 记录不保存过期字段，TokenSummary 也不展示过期时间（创建/update/rotate 的返回 `TokenIssued` 可携带本次 JWT 的 exp 供客户端参考）；
- token 属性修改（本轮修订）：token 创建后可修改名称与权限数据（全部/指定项目访问范围、项目级 `metadata`/`artifacts`/`administration`、账号级 `projects: create|delete`），修改即时生效；不得使 token 权限超过其所属用户权限（越权判定仍由 P-02 执行）；
- 区分 token session 与用户登录 session（本轮澄清）：token JWT 与登录 session JWT 在 claims、存储与解析路径上分离——认证中间件解析时将 token 凭据构造成 `Principal::Token`（token 会话），登录 session 凭据构造成 `Principal::User`（用户登录会话），授权判定与审计按凭据类型区分，不能互相冒充/混用；
- 过期策略：支持“不过期”或自定义过期且最长 1 年，服务端校验，不信任客户端；
- token 权限数据：访问项目范围选择（全部/指定项目）+ 项目级 `metadata`/`artifacts`/`administration` + 账号级 `projects: create|delete`（参考 GitHub token）；越权判断由 P-02 权限核心执行；
- token 管理 HTTP 接口属于本模块；HTTP 实现统一使用 `sfo-http`（第六次修订定稿，与 HTTP 服务器装配一致）。

### P-04 fh-server-files：文件管理模块

- `.tar.gz` 物理文件管理：`data_dir` 存储布局、路径防穿越、文件存储路径与唯一文件标识的映射；
- 上传收流、下载流式输出、SHA-256 计算与完整性校验（下载内容与发布时一致）；
- 原子写入（临时文件 -> 校验 -> 落位）与失败/孤儿文件清理；只提供文件级能力，不感知版本语义。

### P-05 fh-server-versions：项目版本信息及其文件模块

- 依赖 P-04 文件管理模块提供文件级能力，不直接操作物理字节；
- 版本元数据：版本号、发布时间、文件大小、SHA-256 等；版本与文件的关联记录（文件标识指向 P-04 管理的物理内容）；
- 版本不可覆盖：同一 `<project>:<version>` 一经发布即不可变，重复发布返回 409，不影响既有版本；
- 版本列表、按版本下载、省略版本时的最新版本语义（最新版 = 最近发布的版本，按发布时间倒序取最近一次发布）；
- 原子发布协调：权限校验 -> 文件入库（P-04）-> 版本记录落库，杜绝可读取的半成品版本。

### P-06 fh-server-projects：项目管理模块

- 依赖 P-05 项目版本信息及其文件模块（版本集合归属）与 P-02 权限核心（授权判定）；
- 项目创建/列表/删除、public/private 可见性控制；
- public 项目匿名只读、private 项目强制授权（判定调用 P-02 权限核心）；
- 项目与协作者授权关系、版本集合的归属模型。

### P-07 fh-server-http：http接口模块

- HTTP 服务装配（第六次修订）：以 `sfo-http` 为唯一 HTTP 服务器实现——监听配置、TLS/HTTPS、路由注册与中间件链路均基于 `sfo-http`，filehub 不自写 HTTP 服务器装配；会话与 token 凭据统一从 HTTP `Authorization` 头读取（不使用 cookie）；
- `/api/v1` 路由注册与请求/响应 DTO、HTTP 错误映射（401/403/404/409/422 等）；
- 授权中间件接线（调用 P-02 权限判定核心）、统一日志（`sfo-log`）与请求上下文；
- 输出文档化 API 契约供 web（002）与 CLI（003）共同消费；不托管任何前端静态资源。

## Success Criteria

可见结果与必须的证据：

1. 子模块结构：`filehub-server` 源码按账号、权限管理、token管理、文件管理、项目版本信息及其文件、项目管理、http接口七个子模块组织，全部是该 crate 内的子 mod，与依赖顺序一致（版本模块依赖文件管理模块，项目模块依赖版本模块）；版本模块不直接操作物理字节，文件模块不感知版本语义，token生命周期收敛于token管理模块，角色模型与权限判定收敛于权限管理模块（授权以项目为载体，账号模块不承载角色），http接口模块只做对外装配。
2. 服务端独立可验证：配置账号可通过 `/account/login` 登录建立 session，经 `sfo-account` 的账号信息/会话校验接口恢复会话，refresh 续期与过期/解码失效按 `sfo-account` 语义可正反例验证；未登录访问 private 项目/下载返回 401/403；public 项目匿名可浏览版本并下载。
3. token 授权生效：token 为可调用 API 的 JWT 凭据（Bearer）；按 GitHub 风格权限模型校验；仅 `artifacts: read` 的 token 不能发布、不能读取未授权项目；未授予 `projects: delete` 的 token 不能删除项目；过期、撤销、轮换、越权全部被拒绝；权限矩阵测试通过。
   修订增量：token 权限修改后新权限即时生效、旧权限不残留回退窗口；修改后的权限仍不超过所属用户权限；名称/权限等属性变更（含涉及重签的场景）正反例均可验证；重签/轮换生成新密钥对、服务端不持久化签名私钥、旧 JWT 立即失效可验证；过期只由 JWT `exp` 判定、token 记录无过期字段可验证；请求认证能明确区分 token session 与用户登录 session，两类凭据不可互冒（含 token 无法当作登录 session 恢复当前用户、登录 session 无法当作 token 使用的反例）。
4. 用户授权生效：`owner` 可把项目授权给其他配置用户并设置 `read`/`write`/`admin`；被授权 `read` 的用户不能发布版本；未授权用户不能访问 private 项目；`member` 默认不能创建/删除项目；token 权限不超过其所属用户权限。
   修订增量（第四次）：不验收自建 session 列表/逐会话撤销/登出；验收 `sfo-account` 直接导出的登录、会话/账号信息校验、refresh 续期与过期/解码失效正反例。
5. 版本与产物：文件与目录均以 `.tar.gz` 发布；下载内容 SHA-256 与发布时一致；同一版本号重复发布被拒绝（409）且既有版本内容不变；省略版本号可下载该项目最新版本；并发/中断场景不产生可读取的半成品版本。
6. 前端解耦：服务端不托管任何前端资源；`002-filehub-web` 以独立静态站点部署后，通过配置的 API base URL 可完成整链路联调。
7. 公开 API 契约：v1 API文档化，且与 `002-filehub-web`、`003-filehub-cli` 的契约测试（夹具/桩）一致。
8. 交付证据：仓库内自动化测试（单元/集成/DV，经 `test-run.sh`/`test-run.py` 可运行）覆盖上述正反例；high-risk 全生命周期文档（proposal/design/testing/acceptance）齐全并逐级校验通过。
9. 接口并发语义：设计文档中服务端各子模块所有可能执行 IO 的 trait/接口方法（permissions、tokens、files、versions、projects 的 service trait；account 的初始化/decode_session）均为 `async fn` 签名；http 模块公共入口保持 async；纯计算辅助方法保持同步，实现阶段按此冻结。

非目标成功证据：本任务不要求自带管理后台页面成品或 CLI 成品作为交付物（分别由 002/003 验收）。

## Risks

- 安全边界（高）：密码散列、session 与 token 两类 JWT 凭据、凭据库与签名密钥材料、统一授权、public/private 边界均为高价值攻击面；设计阶段明确访问矩阵，测试阶段用独立反例覆盖。
- 凭据与签名密钥管理（高，本轮修订新增）：token/session 均为 JWT 凭据，token 签名私钥签发后即弃、服务端仅存验签公钥；每次重签/轮换生成新密钥对并替换验签公钥、旧 JWT 立即失效，该策略纳入设计与测试。
- 权限模型复杂度（中高）：账号角色、项目协作角色与 token 权限三层叠加，访问判断必须收敛到单一授权中间件并固化为访问矩阵，避免各接口自行拼装权限。
- 外部依赖（中）：`sfo-account`/`sfo-log` 为内部 crate，需锁定来源与版本（crates.io/Git/本地路径），避免供应链漂移；本机工作区已有源码可先用于联调。
- 数据 schema（中高）：首版用户/项目/版本/token 模型决定后续迁移成本；确定所有权与迁移边界。
- 存储与产物完整性（中）：`.tar.gz` 发布/下载两侧校验与原子性；目录打包上限按部署配置。
- 对外 API 合约（中高）：v1 API 一旦发布即承担兼容负担；契约先定稿并与 002/003 同步。
- 模块间集成（中）：API 契约、API base URL 配置与 002/003 的交付节奏需要显式对齐；阶段验收用契约桩隔离。
- 内部子模块边界（中）：版本-文件关联、权限判定复用与 http 装配横跨多个子模块；设计阶段固定内部接口（文件标识、版本记录、错误码约定），避免职责漂移导致权限或产物完整性问题。
- 部署与运维（中）：HTTPS、会话/token 经 `Authorization` 头传输、`data_dir` 配置与备份策略纳入设计。
