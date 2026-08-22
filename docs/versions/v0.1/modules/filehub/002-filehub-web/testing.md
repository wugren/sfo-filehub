---
task_manifest: task.yaml
status: approved
---

# filehub-web 测试设计

Risk profile: ./risk-profile.yaml

## Test Document Index

| Document | Topic | Scope |
|----------|-------|-------|
| 本文件 | 前端逻辑单元测试、生产构建 DV、v1 契约桩集成测试 | admin-web 全部四个 change |

## Unified Test Entry

- Machine-readable task plan: `testplan.yaml`
- Task all: `UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/002-filehub-web all`
- Single-task boundary: 只运行本任务 plan 的 unit/dv/integration 步骤，不选 package/module 或 `all all`。
- 测试实现均经 `tests/vitest.config.ts`/`tests/dv/verify-dist.mjs` 注册到 testplan 步骤，无独立 ad hoc 入口。

## Submodule Tests

| Submodule | Responsibility | Detailed Test Doc | Required Behaviors | Edge/Failure Cases | Test Type | Test Files | Status | Gap / Manual Reason |
|-----------|----------------|-------------------|--------------------|--------------------|-----------|------------|--------|---------------------|
| api-client | DTO/URL/两套响应适配/Bearer/下载 | 本文件 | 登录包装解包、错误分类、project_scope JSON、下载头 | err!=0、403/404/422、网络失败 | unit+integration | `tests/unit/client.test.ts`、`tests/unit/errors.test.ts`、`tests/integration/contract.test.ts` | ready | |
| session | 登录/续期/登出/存储 | 本文件 | 凭据保存/恢复/清除、refresh 成功与失败、登出 | 存储不可用、refresh 失败 | unit | `tests/unit/session.test.ts` | ready | |
| projects | 项目/版本/下载页面 | 本文件 | 可见性 POST 语义、下载带 Bearer | 下载 401、404 | integration | `tests/integration/contract.test.ts` | ready | |
| tokens | token 管理页面 | 本文件 | 创建/列表无过期/改名不重签/重签/轮换/撤销 | token 不存在、owner 操作 | unit+integration | `tests/unit/client.test.ts`、`tests/integration/contract.test.ts` | ready | |
| collaborators | 协作者管理 | 本文件 | user_id upsert/移除、owner 403 | 非法 user_id、越权 | integration | `tests/integration/contract.test.ts` | ready | |
| build | 独立静态构建/交付 | 本文件 | 生产构建与 dist 结构 | 构建失败非零退出 | dv | `tests/dv/verify-dist.mjs` | ready | |

## Module-Level Tests

| Test Item | Covered Boundary | Entry | Expected Result | Test Type | Test File/Script | Status | Gap / Manual Reason |
|-----------|------------------|-------|-----------------|-----------|------------------|--------|---------------------|
| 登录成功/失败 | 页面与 sfo-account 包装 | `tests/unit/components/LoginPage.test.tsx` | 失败展示 msg，成功跳转 next | unit | vitest | covered | |
| 路由守卫 | anonymous/authenticated | `tests/unit/components/ProtectedRoute.test.tsx` | 匿名回 /login，认证放行 | unit | vitest | covered | |
| API 传输契约 | client 与请求语义 | `tests/unit/client.test.ts` | 包装/错误/body 序列化 | unit | vitest | covered | |
| 生产构建链路 | 整模块运行 | `npm run test:dv` | tsc+vite 成功且 dist 非空 | dv | `tests/dv/verify-dist.mjs` | covered | |
| 端到端契约 | 契桩全流程 | `npm run test:integration` | 登录→项目→token→协作者→下载 | integration | vitest | covered | |

## External Interface Tests

| Interface | Responsibility | Success Cases | Failure/Edge Cases | Test Type | Test Doc/File | Status | Gap / Manual Reason |
|-----------|----------------|---------------|--------------------|-----------|---------------|--------|---------------------|
| `/account/login|refresh_session|get_account_info` | 包装解包 | err==0 返回凭据 | err!=0 仍 HTTP 200 → AuthError | integration/unit | `tests/integration/contract.test.ts`、`tests/unit/client.test.ts` | covered | |
| `/api/v1/projects*` | 项目/可见性/删除 | 列表/创建/visibility POST/删除 | 404/403 | integration | `tests/integration/contract.test.ts` | covered | |
| `/api/v1/tokens*` | token 生命周期 | 创建/列表/改名/重签/轮换/撤销 | token not found、无过期字段 | integration/unit | 同上 | covered | |
| `/api/v1/projects/{id}/collaborators*` | 协作者 | upsert/移除 | owner 403 | integration | 同上 | covered | |
| `/api/v1/projects/{id}/versions*`、download | 版本/下载 | Bearer 下载 200 | 匿名 401、404 | integration | 同上 | covered | |

## Direct Change Coverage

| change_id | design_source | validation_id | testplan_level | testplan_step_id | Gap? | Gap / Manual Reason |
|-----------|---------------|---------------|----------------|------------------|------|---------------------|
| fh-web-login | `design/session.md` + `design/api-client.md` | VAL-LOGIN | integration | web-integration | no | |
| fh-web-project-versions | `design/projects.md` + `design/api-client.md` | VAL-PROJECT | integration | web-integration | no | |
| fh-web-token-manage | `design/tokens.md` + `design/api-client.md` | VAL-TOKEN | integration | web-integration | no | |
| fh-web-members | `design/collaborators.md` + `design/api-client.md` | VAL-MEMBERS | integration | web-integration | no | |

## Case-Type Coverage

| change_id | case_type | required | validation_id | level | status | gap_manual_reason |
|-----------|-----------|----------|---------------|-------|--------|-------------------|
| fh-web-login | normal | yes | VAL-LOGIN | integration | covered | |
| fh-web-login | boundary | yes | VAL-UNIT-CONTRACT | unit | covered | 登录 body 字段与 timestamp 序列化在 client.test.ts 覆盖 |
| fh-web-login | negative | yes | VAL-LOGIN | integration | covered | 错误凭据 err!=0 包装 |
| fh-web-login | error | yes | VAL-UNIT-ERROR | unit | covered | ApiError 分类与消息回退 |
| fh-web-login | compatibility | yes | VAL-LOGIN | integration | covered | sfo-http 包装、无登出端点的本地登出语义 |
| fh-web-login | lifecycle | yes | VAL-LOGIN | unit | covered | session.test.ts 的登录/恢复/登出/refresh 生命周期 |
| fh-web-login | cross-module | yes | VAL-LOGIN | integration | covered | 契约桩登录→受保护资源链路 |
| fh-web-project-versions | normal | yes | VAL-PROJECT | integration | covered | 项目列表/创建/可见性/删除/版本/下载 |
| fh-web-project-versions | boundary | yes | VAL-PROJECT | manual | manual | 空名/超长名等输入边界由服务端校验，未在契约桩单列自动化用例 |
| fh-web-project-versions | negative | yes | VAL-PROJECT | integration | covered | 删除后列表移除；owner 可见性语义 |
| fh-web-project-versions | error | yes | VAL-UNIT-CLIENT | unit | covered | 下载 422/网络失败分类 |
| fh-web-project-versions | compatibility | yes | VAL-PROJECT | integration | covered | visibility 以 POST 调用、下载附件头 |
| fh-web-project-versions | lifecycle | yes | VAL-PROJECT | manual | manual | 删除确认与页面加载状态未组件级自动化 |
| fh-web-project-versions | cross-module | yes | VAL-PROJECT | integration | covered | projects→versions→download 契桩全流程 |
| fh-web-token-manage | normal | yes | VAL-TOKEN | integration | covered | 创建/列表/改名/重签/轮换/撤销 |
| fh-web-token-manage | boundary | yes | VAL-TOKEN | unit | covered | project_scope All/Specified/空、expires null |
| fh-web-token-manage | negative | yes | VAL-TOKEN | integration | covered | 不存在的 token → 404 |
| fh-web-token-manage | error | yes | VAL-UNIT-CLIENT | unit | covered | 更新响应 TokenIssued/TokenSummary 判定 |
| fh-web-token-manage | compatibility | yes | VAL-TOKEN | integration | covered | 列表无 expires_at；重签/轮换旧 JWT 失效语义由契约桩断言 |
| fh-web-token-manage | lifecycle | yes | VAL-TOKEN | manual | manual | JWT 一次性展示、复制与关闭交互未在 jsdom 自动化 |
| fh-web-token-manage | cross-module | yes | VAL-TOKEN | integration | covered | tokens 与 project_scope 项目列表联动 |
| fh-web-members | normal | yes | VAL-MEMBERS | integration | covered | upsert/改级/移除 |
| fh-web-members | boundary | yes | VAL-MEMBERS | manual | manual | 空/非数字 user_id 前端校验在组件层未单独自动化（逻辑简单为正整数校验） |
| fh-web-members | negative | yes | VAL-MEMBERS | integration | covered | owner 行 403 |
| fh-web-members | error | yes | VAL-MEMBERS | manual | manual | 页面级 403 错误态呈现未组件级自动化；错误体分类由 errors.test 覆盖 |
| fh-web-members | compatibility | yes | VAL-MEMBERS | integration | covered | PUT upsert 与 DELETE 语义 |
| fh-web-members | lifecycle | yes | VAL-MEMBERS | manual | manual | 移除确认交互未自动化 |
| fh-web-members | cross-module | yes | VAL-MEMBERS | integration | covered | project 选择→collaborators 列表链路 |

## Design Element Coverage

| element_type | design_source | derived_cases | level | status | gap_manual_reason |
|--------------|---------------|---------------|-------|--------|-------------------|
| parameter-domain | 设计 `## File-Level Interfaces`/`design/api-client.md` | project_scope All/Specified/空；formatBytes 边界；登录 timestamp；token expires null | unit | covered | |
| state-transition | 设计 `## State and Ownership` session stateDiagram | anonymous→authenticated→anonymous；refresh 成功/失败 | unit | covered | |
| failure-path | 设计 `## Key Flows` refresh/下载失败分支 | 401 refresh 失败回登录；下载 401/网络错误分类 | unit | covered | |
| error-handling | `design/api-client.md` 错误分级 | 401/403/404/409/422/transport 分类与消息回退 | unit | covered | |
| invariant | 设计 `## State and Ownership` Invariants | sessionStorage 不写 localStorage；JWT 仅签发响应；下载带 Bearer；列表无过期列 | integration | covered | 存储断言在 session.test、下载/无过期列断言在 client.test 与 contract.test |
| concurrency | 设计无并发/重入声明（`## State and Ownership`、`design/session.md` 续期策略） | not-applicable: 前端单线程 fetch，401 refresh 由 withAuthRetry 串行化，无共享可变并发状态声明 | - | not-applicable | 设计中未声明 race/reentry/ordering 约束 |

## Validation Rationale

| Behavior or Risk | Validation Signal | Why This Is Sufficient | Gap / Manual Reason |
|------------------|-------------------|------------------------|---------------------|
| 两套响应/错误格式适配 | client/errors 单测 + 契约桩错误语义 | 直接命中 ApiError 分类与解包分支 | |
| 会话凭据受限存储与续期 | session.test 断言 sessionStorage/登出/续期 | 单测覆盖全部存储分支 | |
| token 明文一次性展示 | 创建/重签/轮换响应断言与页面仅展示一次 | 契约桩断言 jwt 仅在签发响应；页面交互为 manual | jsdom 剪贴板限制 |
| POST 更新语义与无过期字段 | 契约桩断言 visibility POST、token 列表无 expires_at | 直接对应 v1 契约事实 | |
| 下载 Bearer 与附件语义 | contract.test 断言 Authorization 头与 blob 字节 | 直接对应 artifacts:read 边界 | |
| 独立构建交付 | test:dv 构建并校验 dist | 直接证明可复现交付物 | |

## Unit Tests

| Function or Unit | Branch or Condition | Covered Behavior | Test File | Status | Gap / Manual Reason |
|------------------|---------------------|------------------|-----------|--------|---------------------|
| `ApiError.fromV1` | 401/403/404/409/422/其他状态分支 | kind 映射与消息回退 | `tests/unit/errors.test.ts` | covered | |
| `encodeProjectScope` | all / specified 分支 | "All" 与 {Specified} | `tests/unit/contract.test.ts` | covered | |
| `describeProjectScope` | All/空/非空分支 | 展示文案 | 同上 | covered | |
| `formatBytes`、`formatTime` | 边界与非法输入分支 | 单位显示与回退 | 同上 | covered | |
| `ApiClient.login` | err==0 / err!=0 分支 | 包装解包与 AuthError | `tests/unit/client.test.ts` | covered | |
| `ApiClient.raw` | ok / 非 ok / 网络异常 / 超时分支 | 错误分级 | 同上 | gap | 超时（AbortController 触发）未注入；需真实计时等待，成本高于收益，转 manual |
| `ApiClient.updateToken` | TokenSummary / TokenIssued 分支 | 改名不重签、scope 变更重签 | 同上 | covered | |
| `ApiClient.download` | 200 / 4xx / 网络失败分支 | blob 与错误 | 同上 | covered | |
| `SessionStore` | 存储成功/字段缺失/JSON 损坏分支 | 持久化与恢复 | `tests/unit/session.test.ts` | covered | |
| `SessionStore.refreshOnce` | 成功 / 失败分支 | 凭据替换与登出 | 同上 | covered | |
| `withAuthRetry` | 未登录 / auth 后刷新成功 / 刷新失败 | 401 后恰一次重试、失败传播原错误且登出 | `tests/unit/session-retry.test.ts` | covered | |
| `TokensPage.scopeError` | 指定项目 / 全项目 / 空指定列表分支 | 阻止空 project_scope 提交（防止服务端列表 422） | `tests/unit/tokens-scope.test.ts` | covered | |
| `ProtectedRoute` | anonymous / authenticated 分支 | 重定向与放行 | `tests/unit/components/ProtectedRoute.test.tsx` | covered | |
| `LoginPage` | 提交成功 / 失败分支 | 导航与错误态 | `tests/unit/components/LoginPage.test.tsx` | covered | |
| `TokensPage` | JWT 卡片展示 / 关闭 / 复制分支 | 一次性明文交互 | 无 | gap | jsdom 剪贴板与一次展示时序未自动化，转 manual |
| `MembersPage` | user_id 正整数校验分支 | 输入校验 | 无 | gap | 组件层未单测，逻辑为 Number 校验，契约桩已覆盖越权路径，转 manual |
| `ProjectsPage` | 删除确认分支 | window.confirm 交互 | 无 | gap | 浏览器确认框依赖用户交互，转 manual |

## DV Tests

| Workflow | Kind | Entry | Expected Result | Test File or Script | Status | Gap / Manual Reason |
|----------|------|-------|-----------------|---------------------|--------|---------------------|
| 构建生命周期（重复构建） | lifecycle | `npm run build` 连续两次 | dist 覆盖式重建成功、无残留进程 | `tests/dv/verify-dist.mjs` 前置步骤 | covered | |
| 生产构建 | main | `npm run test:dv`（tsc + vite build） | 构建成功且失败时非零退出 | `tests/dv/verify-dist.mjs` 前置步骤 | covered | |
| dist 静态产物 | main | `verify-dist.mjs` | index.html 挂载点与 js/css 资源存在且非空 | 同上 | covered | |
| 构建失败信号 | failure | `npm run build` 退出码 | 编译/打包错误终止流程 | 同上 | covered | |

## Integration Tests

| Contract or Flow | Modules Involved | Success Case | Failure Case | Test File | Status | Gap / Manual Reason |
|------------------|------------------|--------------|--------------|-----------|--------|---------------------|
| 登录包装 | api-client ↔ sfo-account | err==0 解包凭据 | HTTP 200 + err!=0 → AuthError | `tests/integration/contract.test.ts` | covered | |
| 项目/可见性/删除 | api-client ↔ projects | 列表/创建/visibility POST/删除 | 删除后列表移除断言 | 同上 | covered | |
| token 生命周期 | api-client ↔ tokens | 创建/列表/改名/重签/轮换/撤销 | token not found → 404 | 同上 | covered | |
| 协作者 | api-client ↔ collaborators | PUT upsert/改级/移除 | owner 行 → 403 | 同上 | covered | |
| 版本与下载 | api-client ↔ versions/storage | Bearer 下载字节一致 | 匿名 private → 401 | 同上 | covered | |
| 错误语义 | api-client ↔ 契约桩 | 未知路由 → 404 | 404 kind/status 断言 | 同上 | covered | |

## Regression Focus

- 登录失败仍 HTTP 200 的包装错误不得被当作成功；
- token 列表不得出现 `expires_at` 列；
- visibility 与 token 属性更新必须使用 POST 语义端点；
- 下载必须携带 Bearer（public 匿名除外）且文件名遵循 `{project_id}-{version}.tar.gz`；
- refresh 只允许一次重试，失败后清除本地凭据。

## Definition of Done

- [x] Testing docs 覆盖全部 6 个直接子模块
- [x] 测试文档未拆分、内容 < 1000 行
- [x] `testplan.yaml` 与声明的测试入口一致
- [x] `testplan.yaml` 存在且通过 schema 校验
- [x] 测试实现全部注册到 `harness/scripts/test-run.py` 可见的 task plan（<module>/<task-name> all）
- [x] 新单测均在 `admin-web/tests/` 专用测试目录，未在 src 生产文件内新增 inline 测试
- [x] task all 只运行本任务 plan
- [x] 未选择 package/module 或 all all 运行面；无 breaking/migration API、crate-root 导出或构建面变更，无需额外 consumer-closure 契约步骤
- [x] 模块级测试覆盖关键边界与失败路径
- [x] 外部接口有契约级测试
- [x] 单测覆盖核心分支；未覆盖分支以 per-branch 原因记录
- [x] DV 覆盖主流程与失败信号
- [x] 集成覆盖每个消费接口的成功与失败语义
- [x] `## Design Element Coverage` 六类元素均映射或带具体 not-applicable 原因
- [x] 每个 change_id 在 proposal/design/testing/testplan 中一致出现
- [x] manual 层在 testing.md 与 testplan.yaml 中均有原因（testplan 无 manual 层；testing.md 的 manual 行均给出原因）
- [x] 相关自动化测试通过（unit/dv/integration 已在本任务运行）
