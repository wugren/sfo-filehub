# CLI 与管理端其余页面补齐项目分页消费

- Status: complete
- Owner module: filehub（filehub-cli / filehub-web / 003-cli、002-web 前端）
- Task manifest: docs/versions/v0.1/modules/filehub/047-project-pagination-all-consumers/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/047-project-pagination-all-consumers/proposal.md
- Affected paths: cli/src/apiclient/mod.rs、cli/tests/common/mod.rs、
  cli/tests/api_integration.rs、admin-web/src/api/client.ts、
  admin-web/src/pages/ProjectDetailPage.tsx、admin-web/src/pages/TokensPage.tsx、
  admin-web/tests/unit/client.test.ts、
  admin-web/tests/unit/components/TokensPage.test.tsx、
  admin-web/tests/unit/components/ProjectDetailPage.test.tsx、
  admin-web/tests/integration/contract.test.ts、docs/api/v1-contract.md
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- CLI `list_projects` 改为分页循环拉取全部可见项目：固定 `limit=500`、按
  `offset` 推进，从 `X-Total-Count` 响应头读取总量；header 缺失/非法时回退
  单页（与旧服务端/旧 mock 兼容）。公开签名与方法名不变，`resolve_project`
  与 login 校验自动获得全量语义；
- admin-web `ApiClient` 新增 `getProject`（消费 040 的
  `GET /api/v1/projects/{id}` 直查）与 `listAllProjects`（基于
  `listProjectsPage` 以 500/页循环拉全量，空页/达总量即停）；ProjectDetailPage
  改用直查消除首屏依赖，404 展示文案不变；TokensPage 改用全量项目列表，
  Specified 勾选与 scope 名称展示覆盖全部可见项目；
- 测试：CLI mock 支持 query 分页与 `x-total-count`（`tok-paged` 520 个项目
  夹具），新增跨页按名解析用例；admin-web client 单测补 `getProject` 与
  `listAllProjects`（多页、无总量头回退）；TokensPage/ProjectDetailPage
  组件测试与契约桩同步适配；契约文档补消费对齐说明。

## Risk Screen

- Public contract, protocol, or CLI change: yes（CLI 传输行为改变：项目列表
  改为分页拉全量；公开 API 契约不变，仅消费既有 `?limit/offset` 与
  `X-Total-Count`；`list_projects`/`resolve_project` 签名不变）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no（依赖零变更；与旧服务端无总量头时回退单页，保持兼容）
- Material UI, accessibility, localization, or navigation workflow change: yes
  （详情页加载来源改为直查、Token 页项目列表改为全量；无新增导航/交互控件）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no（消费端收敛；
  服务端 040 契约直接复用）

## Verification

- Targeted check:
  - `cargo test -p filehub-cli --test api_integration -- --test-threads=1`：
    16/16 通过（含新增 `resolve_project_scans_paginated_projects_beyond_first_page`
    跨页解析用例，既有 15 例零回归）；
  - `cargo test -p filehub-cli --test cmd_integration -- --test-threads=1`：
    14/14 通过（mock 分页头下真实二进制命令零回归）；
  - `npm run test:unit`：57/57 通过（新增 client `getProject`/`listAllProjects`
    4 例与 TokensPage 跨页 Specified 1 例，页面组件测试同步适配）；
  - `npm run test:integration`：9/9 通过（新增直查/全量拉取断言，契约桩支持
    `GET /projects/{id}`）；
  - `npm run build`：tsc + vite 构建通过。
- Result: pass
- Residual risk or follow-up: CLI/Token 页全量拉取随可见项目数线性增加请求数
  （500/页）；若后续服务端增加按名/搜索过滤接口，可再优化解析路径。
