---
task_manifest: task.yaml
status: approved
---

# filehub 页面/管理后台（filehub-web）提案

## Approval Record

- approver: user
- approval_date: 2026-08-20
- user_statement: 确认，自动完成任务
- 确认决策：技术栈 React、三模块拆分、版本发布仅 CLI、服务后台不托管前端资源、token 修改（重签）与轮换入口纳入首版（按提案建议）。
- Risk profile: ./risk-profile.yaml

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: high-risk
- 触发边界/理由：管理后台承载登录界面、token 管理界面、private 项目数据展示与下载入口，属于安全敏感 UI 工作流；同时依赖服务端认证/授权契约，与 `001-filehub-core-platform`、`003-filehub-cli` 共享公开 API 边界。命中 security/privacy 与 ui-workflow 等实质性风险类别，按 high-risk 全流程（提案 -> 设计 -> 实现 -> 测试 -> 验收）执行。
- 本次需求修订说明：002 原本处于提案草稿阶段，001 server 与 v1 契约已按 `docs/api/v1-contract.md` 交付；本修订把页面需求与当期 server 实际接口行为对齐，不改变技术栈、交付形态与 high-risk 判定。
- 确认陈述：本提案需获得当前用户明确确认后才能进入执行；用户可选择按本提案确认、以替换层级（trivial/standard/high-risk）确认，或要求修订提案。

## Background and Goal

filehub 需要一个管理后台页面：用户登录后可以看到自己可见的项目、查看每个项目的版本情况，并下载指定版本的二进制文件；同时提供 token 管理与项目可见性/协作者管理入口。本任务交付 React 管理后台（`filehub-web`），只负责页面、状态与交互；认证、授权、数据存储、版本与产物管理全部由服务后台（`001-filehub-core-platform`）提供，发布客户端（`003-filehub-cli`）单独交付。本修订版需求严格对齐 001 当期交付的 v1 契约（`docs/api/v1-contract.md`）与 server 代码行为。

## Scope

### In scope

1. 登录与会话接入（按当期 server 的 sfo-account 接口）
   - 用户名/密码登录页：`POST /account/login`，body `{user_name,password,timestamp}`（unix 秒），响应为 sfo-http 包装 `{err,msg,result:{session,refresh_session}}`；以 `err==0` 判定成功，失败错误态展示 `msg`；
   - 当前用户展示：`GET /account/get_account_info`（Bearer session），result 仅含 `{id,name,session_id}`；页面只展示 id/name，账号角色（owner/member）不通过 API 返回，不展示、不做本地权限判断；
   - 受保护路由守卫与 401 续期：session 失效时用 `POST /account/refresh_session`（Bearer refresh_session）续期一次，失败后回登录页；
   - 登出：当前 server 无会话注销端点，登出 = 清除前端本地会话凭据并回到登录页；
   - 会话凭据经 HTTP `Authorization: Bearer` 头传输，不使用 cookie；凭据仅存浏览器内存/sessionStorage，不写入 localStorage。
2. 项目页（按 /api/v1/projects 契约）
   - 项目列表：`GET /api/v1/projects` 返回当前身份可见项目（匿名仅 public；登录/ token 为 public + 自己拥有或被授权的 private），字段 `{project_id,name,visibility,owner}`（owner 为数字用户 id）；文案“可见项目”，展示 visibility 与 owner id；
   - 创建入口：`POST /api/v1/projects` body `{name,visibility?}`；越权（member/无 projects:create scope）提交后展示服务端 403；
   - 可见性切换：`POST /api/v1/projects/{id}/visibility` body `{visibility}`（当期 server 以 POST 承载更新语义，前端按 POST 调用），需 `administration`；
   - 删除入口（确认交互）：`DELETE /api/v1/projects/{id}`（账号级 `projects:delete`，项目 admin 不自动获得删除权）。
3. 版本与下载页（按 /api/v1/projects/{id}/versions 契约）
   - 版本列表：`GET .../versions` -> `VersionRecord[]`（`project_id,version,file_id,sha256,size,published_at`，按发布时间倒序）；
   - 单版本/最新：`GET .../versions/{version}`，支持 `latest` 关键字；
   - 下载：`GET .../versions/{version}/download`（支持 latest），响应 `application/gzip` 与 `attachment; filename="{project_id}-{version}.tar.gz"`；public 匿名可下载，private 下载需带 Authorization；页面采用可携带凭据的下载实现（fetch -> blob）。
4. token 管理页（按 /api/v1/tokens 契约，仅用户 session 可用）
   - 创建 token：body `{name,project_scope?,scopes?,expires_at?}`；`project_scope` 的 JSON 为 `"All"` 或 `{"Specified":[<project_id>,...]}`（页面内按项目列表勾选后序列化为该形式）；`scopes` 取六固定值 `metadata:read/artifacts:read/artifacts:write/administration/projects:create/projects:delete`；`expires_at` 为 RFC3339 UTC 或 null（不过期），最长 1 年；201 响应 `{token_id,jwt,name,expires_at}`，JWT 明文仅展示一次、不落库；
   - token 列表：`GET /api/v1/tokens` -> `TokenSummary[]`（无过期字段，页面不展示过期列）；创建/修改/轮换响应中的 `expires_at` 仅作一次提示；
   - 修改与轮换（服务端已支持，建议首版纳入）：`POST /api/v1/tokens/{id}`（仅 name 变更不重签并返回 TokenSummary；含 scopes/project_scope/expires_at 变更则重签并返回新 JWT 一次，旧 JWT 立即失效；`expires_at:null` 表示不修改，重签不带 exp 时按当前服务端语义为不过期）、`POST /api/v1/tokens/{id}/rotate`（返回新 JWT 一次，轮换后不过期，旧 JWT 立即失效）；
   - 撤销：`DELETE /api/v1/tokens/{id}`，204，撤销后立即失效。
5. 项目协作者管理页（按 /api/v1/projects/{id}/collaborators 契约）
   - 协作者列表：`GET .../collaborators` -> `Collaborator[]`（`{user_id,role}`，role 为 `read|write|admin`）；项目 owner 为隐式 admin 且不出现在列表中；
   - 添加/改级：`PUT .../collaborators/{user_id}` body `{role}`（upsert 语义，重复 user_id 即改级）；移除：`DELETE .../collaborators/{user_id}`；对 owner 行执行授予/修改/移除由服务端返回 403；
   - 当前 server 无用户目录/用户名查询 API：页面以数字 user_id 输入与展示，不做用户名搜索或自动补全。
6. 构建与交付（不变）
   - React 工程与路由/状态管理；`admin-web/dist/` 独立静态站点交付，不依赖服务后台静态托管；
   - API base URL 配置化；开发环境代理到服务后台。

### Out of scope / non-goals

- 服务端认证、授权、token、项目、版本与产物 API 的实现（归属 `001-filehub-core-platform`）；
- 版本发布/上传入口（首版只由 `003-filehub-cli` 发布，后台只查看与下载）；
- 用户注册、管理员用户管理、用户目录/按用户名查询接口（server 当前不提供）；
- 服务端会话注销/登出端点（当前不提供，登出仅本地清除）；
- GitHub Organization/团队层级与邀请流程（不做组织层级，项目协作授权见 P-04）、i18n 多语言、PWA/离线能力、换肤等非必需体验增强；
- 服务端数据模型与存储设计。

### 相邻边界

- 页面不做业务权限判断，只按服务端返回结果展示与放行；member/无权限用户提交管理操作时呈现 401/403 错误态；
- token/session JWT 明文只在创建/修改/轮换响应中展示一次，前端不落库、不写入 console 日志与本地存储；
- 错误契约分两套：`/api/v1/*` 统一 `{"error","message"}` + HTTP 状态码（401/403/404/409/422/5xx）；`/account/*` 为 sfo-http 包装 `{err,msg,result}` 且登录失败仍返回 HTTP 200，需按 `err` 判断；
- private 项目数据访问由服务端授权决定 401/403，前端只呈现错误态。

## 实现模块拆分（Implementation Module Split）

三模块实现由用户确认：

1. `001-filehub-core-platform`（服务后台 `filehub-server`）：认证/授权、项目/版本/产物 API，不包含前端托管；
2. `002-filehub-web`（本任务，页面/管理后台）：React 管理页面，独立部署；
3. `003-filehub-cli`（发布客户端）：跨平台 CLI。

本任务与 `001-filehub-core-platform` 只通过公开 v1 API 契约交互；API base URL 为显式联调契约，前端资源由本任务独立交付与托管。

## Requirement Review

需求合理：管理后台聚焦“查看、下载、管理 token 与可见性/协作者”，业务规则与权限全部收敛到服务后台，页面承担表达与交互，职责边界清晰。

本修订版按当期 server 的关键对齐点：

- sfo-account 接口使用 `{err,msg,result}` 包装且登录失败仍为 HTTP 200，需要专门错误适配；/api/v1 使用独立 JSON 错误体；
- 当前用户信息不含账号角色，页面不能展示或依据角色本地放行；
- token 列表不返回过期字段，只有签发响应返回 `expires_at`；
- 协作者接口只返回 user_id，无用户名/用户目录 API；
- 更新语义端点（visibility、token 属性）当前以 POST 提供；
- server 无登出端点，登出仅本地清凭据。

关键取舍与建议方向：

- 权限的真实性：前端不做本地放行，所有敏感数据请求由服务端 401/403 兜底；
- 部署契约：页面作为独立静态站点交付，API base URL 指向服务后台；服务端不涉及前端托管；
- token 明文处理：仅在一次性的创建/修改/轮换响应中显示，用户关闭页面后不可再查看；
- 协作者管理按 user_id：无用户目录 API，页面明确提示输入数字 user id，移除 GitHub 式用户名选择交互。

### 待确认问题（Open questions）

用户已确认：技术栈（React）、三模块拆分为独立任务、版本发布只由 CLI 承担、服务后台不托管前端资源（页面独立部署）。账号角色 `owner`/`member`、项目协作角色 `read`/`write`/`admin` 已由 001 落地，本任务保留项目协作者管理页。

已确认问题：token 修改（重签）与轮换入口纳入首版（服务端已支持；遵守 JWT 明文仅展示一次与「轮换后不过期」「重签后旧 JWT 立即失效」提示）。本提案已无未决问题。

## Proposal Items

每个提案项均给出稳定 `proposal_id` 与实现侧 `change_id`，后续设计/测试/验收按 `change_id` 追踪。

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-01 | fh-web-login | 登录页、当前用户展示、受保护路由、401 续期一次与本地登出；凭据经 Authorization 头传输且不落 localStorage | 不实现服务端会话；账号角色不展示；登出仅清除本地凭据（server 无登出端点） | 凭据保存在内存/sessionStorage 并支持 refresh 续期，换取刷新后无需重复登录 | 未登录访问受保护页回登录页；登录按 `{err,msg,result}` 的 err==0 判定 | 用户注册、账号角色管理、服务端会话注销 |
| P-02 | fh-web-project-versions | 可见项目列表/创建/删除/可见性切换界面；版本列表（倒序）与指定版本/latest 下载 | /api/v1/projects 与 versions 契约；更新语义端点以 POST 调用；下载需可携带 Authorization | 列表文案为“可见项目”，owner 按 user_id 展示 | 端到端登录->项目->版本->下载可用；越权请求呈现 401/403 | 版本上传/发布（仅 CLI）；项目管理只读之外的本地放行 |
| P-03 | fh-web-token-manage | token 创建（名称/项目范围/六固定 scope/过期时间）、列表、修改/轮换（建议）、撤销；JWT 明文仅展示一次 | 仅用户 session 可管理；列表无过期字段；`expires_at:null` 表示不修改；最长 1 年 | 纳入修改/轮换以对齐服务端能力，代价是重签后旧 JWT 失效与不过期语义的提示复杂度 | 创建展示明文一次；列表无过期列；轮换后旧 JWT 立即失效；撤销后列表移除 | token 管理 API 实现；明文持久化/日志记录 |
| P-04 | fh-web-members | 按 user_id 查看/添加/改级/移除项目协作者（read/write/admin） | 无用户名/用户目录 API；owner 为隐式 admin 且不在列表；owner 行管理返回 403 | 用户反馈以数字 user_id 输入，换取首版无用户目录约束下的可用性 | 协作者增改删正常；无 administration 权限显示 403 | 用户名搜索/自动补全；用户目录 API |

### P-01 fh-web-login：登录与会话接入

- 登录页：`POST /account/login`（body `{user_name,password,timestamp}`），按 sfo-http 包装 `{err,msg,result}` 判定成功并保存 session/refresh_session；失败错误态展示 `msg`；
- 当前用户展示（id/name）与受保护路由守卫；`GET /account/get_account_info` 或 `POST /account/get_account_info_of_session` 初始化当前用户；
- session 401 时用 refresh_session 续期一次，失败回登录页；登出 = 清除本地凭据（服务端无登出端点）；
- 会话凭据经 HTTP `Authorization` 头透传；不使用 cookie/localStorage；不展示账号角色（API 不返回）。

### P-02 fh-web-project-versions：项目、版本与下载页面

- 项目列表/创建/删除/可见性切换界面：`GET`/`POST /api/v1/projects`、`POST .../visibility`（POST 承载更新语义）、`DELETE ...`；文案为“可见项目”，owner 按 user_id 展示；
- 项目版本列表（倒序，含 size/sha256/published_at）与指定版本/latest 下载入口；下载响应 `{project_id}-{version}.tar.gz`（`application/gzip`），页面采用可携带 Authorization 的下载实现。

### P-03 fh-web-token-manage：token 管理页面

- 创建 token：名称、项目范围（all/指定项目）、六固定 scope 选项、过期时间（RFC3339/不过期，最长 1 年）；201 返回 JWT 明文仅展示一次；
- token 列表：名称、项目范围、scopes、创建/更新时间（无过期列）；
- 修改/轮换（建议首版纳入）：重签返回新 JWT 一次并明示旧 JWT 立即失效；轮换后不过期；仅名称修改不重签；
- 撤销与错误态；token 管理仅用户 session 可操作。

### P-04 fh-web-members：项目协作者管理页面

- 查看项目协作者：`user_id` + 角色（`read`/`write`/`admin`），owner 不出现在列表、不展示用户名；
- 添加/改级（PUT，按数字 user_id 输入）、移除（DELETE）；owner 行管理返回 403 错误态；
- 无 `administration` 权限访问时呈现 403。

## Success Criteria

可见结果与必须的证据：

1. 页面端到端可用（配合服务后台真实 API 或契约桩）：登录（按 `err==0` 包装判定）-> 可见项目列表 -> 项目版本列表 -> 下载指定版本二进制（private 带 Bearer，public 匿名可下载）；
2. 未登录访问受保护页面被引导到登录页；登出后本地凭据清除并回到登录页；session 401 时 refresh 续期一次成功，续期失败回登录页；
3. token 管理页可按名称/项目范围/权限/过期时间创建并展示明文一次；列表不展示过期列；修改/轮换（如纳入）返回新 JWT 一次且旧 JWT 立即失效提示正确；撤销后列表移除；
4. 项目可见性切换与删除入口可用，并正确反映服务端结果；越权（member/无 scope）显示 401/403 错误态；
5. 项目协作者管理页按 user_id 添加/改级/移除；owner 行不可管理；无 `administration` 权限显示 403；
6. 管理后台可作为独立静态站点部署，API base URL 指向服务后台后可完成登录/项目/版本/下载整链路；
7. 交付证据：前端组件/集成测试经仓库 `test-run.sh`/`test-run.py` 可运行；与 `docs/api/v1-contract.md` 保持一致的契约测试通过（含 POST 语义端点与 sfo-account 包装）；high-risk 全生命周期文档齐全并逐级校验通过。

非目标成功证据：本任务不验收服务端认证/授权逻辑，也不验收 CLI 发布能力。

## Risks

- 权限敏感 UI（中高）：登录、token 管理与 private 数据页面可能泄露信息或引导越权操作；只展示服务端权限结果，禁止前端本地放行。
- token/session 明文（高）：创建/修改/轮换响应的一次性 JWT 若被记录（console/日志/本地存储）会扩大泄露面；制定显示与清理约定；会话凭据不写入 localStorage。
- 服务端无登出/会话撤销（中）：登出仅前端清除本地凭据，泄露的 session 在过期前仍可能被使用；凭据短期保存并限制存储范围。
- 账号角色不对外暴露（中）：member 与无 scope 用户在页面上无法预判权限，按钮提交后展示 401/403；错误态必须明确。
- 协作者无用户名（中）：无用户目录 API，只能按数字 user_id 输入/展示；避免界面暗示可按用户名管理。
- 两套错误契约（中）：`/api/v1` 与 `/account/*` 错误格式与 HTTP 状态语义不同，前端需统一适配层并有契约测试覆盖。
- token 重签/过期语义（中）：scope 变更立即重签、旧 JWT 失效；轮换后不过期；`expires_at:null` 表示不修改；表单与提示需与服务端语义一致。
- 下载实现（中）：浏览器普通链接无法携带 Authorization 头，需 fetch -> blob 等实现；下载文件名/类型纳入契约测试。
- 联调契约（中）：服务后台 API 或 API base URL 约定变更会波及本任务与 `003-filehub-cli`；契约文档先行、并行开发用契约桩。
- 前端依赖与构建（中）：npm 依赖供应链与构建结果可复现性纳入设计；锁定依赖并校验构建输出。
