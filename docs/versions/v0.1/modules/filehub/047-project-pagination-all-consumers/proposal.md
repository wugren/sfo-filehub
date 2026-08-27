---
task_manifest: task.yaml
status: approved
---

# CLI 与管理端其余页面补齐项目分页消费（承接 040/041）

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-26 回复「确认」，确认采纳本提案
  （CLI 分页拉全量、详情页直查、Token 页全量项目、对应测试与契约文档）并接受
  建议的 standard 层级。
- Tier rationale / triggered boundaries:
  - 不满足 trivial：改动跨 CLI（Rust 传输客户端）与 admin-web（TS 前端）两个
    代码栈，并涉及 CLI 行为与管理端多页面交互变更，不是单模块小修补；
  - 未触发 high-risk：服务端公开契约已在 040 冻结（`?limit/offset` +
    `X-Total-Count`、`GET /projects/{id}`），本任务不改服务端路由/响应形状、
    schema/迁移、安全边界、依赖图或构建发布形态，全部收敛在既有契约的消费端
    与对应测试内。

## Background and Goal

- 040 给服务端 `/api/v1/projects` 增加 `limit/offset` 分页与 `X-Total-Count`
  总量头，并新增 `GET /api/v1/projects/{id}` 直查；041 只迁移了 admin-web
  列表页，并在文档中把 CLI、ProjectDetailPage、TokensPage 明确列为后续任务。
- 现状缺陷：
  1. CLI `list_projects`（`cli/src/apiclient/mod.rs:177`）不带分页参数请求
     `/projects`，服务端默认只返回前 100 条，`resolve_project` 从该结果按名
     匹配，第 101 个之后的项目被误报「不存在」；
  2. ProjectDetailPage（`admin-web/src/pages/ProjectDetailPage.tsx:66`）从首屏
     `listProjects` 结果里按 id 查找，超首屏项目显示假 404；
  3. TokensPage（`admin-web/src/pages/TokensPage.tsx:123`）只拿首屏 100 个项目，
     Specified 范围无法选择后续项目，scope 展示也只能对首屏项目显示名字。
- 目标：所有项目消费方都按 040 已冻结契约取到全部可见项目；详情页改用直查
  端点消除首屏依赖；CLI 按名解析在第 100 条之后仍正确工作。

## Scope

### In scope

- CLI（`cli/src/apiclient/mod.rs`）：
  - `list_projects` 改成分页拉取全部可见项目：按页携带 `?limit=&offset=`，
    从 `X-Total-Count` 读取总量继续取下一页；总量头缺失/服务端未升版时回退
    单页结果，保持与旧 mock/旧服务端兼容；
  - `resolve_project` 及 login 校验沿用 `list_projects`，自动获得全量语义。
- admin-web（`admin-web/src/pages/ProjectDetailPage.tsx`）：
  - `ApiClient` 新增 `getProject(bearer, projectId)`（消费 040 的
    `GET /api/v1/projects/{id}`）；
  - 详情页加载改用直查，不再从列表首屏按 id 查找；404/403 错误呈现与现状
    一致。
- admin-web（`admin-web/src/pages/TokensPage.tsx`）：
  - `ApiClient` 新增 `listAllProjects(bearer)`：基于 `listProjectsPage`
    循环拉取全部可见项目（页大小用服务端上限 500，按总量推进；空页即停，
    可防丢失/死循环）；
  - TokensPage token 列表与 TokenFormModal 改用全量项目：Specified 勾选与
    scope 名称展示覆盖全部项目。
- 契约文档 `docs/api/v1-contract.md`：消费对齐段落补 CLI 全量分页、详情页直查、
  TokensPage 全量拉取说明。
- 测试：
  - CLI mock 服务支持 `limit/offset/`x-total-count`，新增跨页按名解析用例，
    保留无总量头回退路径覆盖；
  - admin-web client 单测补 `getProject`/`listAllProjects`（多页、无总量头
    回退、空页终止）；TokensPage 与 ProjectDetailPage 组件测试/契约桩同步
    适配并补用例。

### Out of scope

- 不修改服务端 040 已交付的分页/直查逻辑；
- 不新增服务端按名查询接口（`?name=` 过滤等）：保持公开契约不变，CLI 用
  分页全量在本端完成按名解析；
- 不给 TokensPage 增加项目搜索/虚拟滚动 UI：当前形态是分页拉全量后单选勾选，
  项目量级由服务端可见项目决定；
- 不处理跨域部署下 `X-Total-Count` 的 CORS expose（041 已记录为后续项，本
  任务不触碰服务端/nginx）。

## Boundary with neighboring modules

- CLI `list_projects` 公开签名不变，仍返回 `Vec<ProjectDto>`，只是内部改为
  循环分页；所有命令调用点零签名改动；
- `ApiClient` 增方法不改既有方法；ProjectsPage 既有 `listProjectsPage` 复用，
  `listProjects` 保留供其他调用方与测试使用；
- 服务端是只被消费方：本任务不新增任何服务端路径或响应字段。

## Requirement Review

- 需求合理：040/041 已把服务端与列表页分页打通，剩余的 CLI 与其他页面仍被
  默认 100 条截断，属于同一缺陷的消费端未闭环。
- 方向选择：
  - CLI：采用"分页拉全量"而非新增服务端按名过滤，符合"支持分页"的既有接口
    方向，且服务端契约零改动；单次解析最多请求
    `ceil(可见项目数 / 500)` 次；
  - 详情页：直接使用 040 已提供的 `GET /projects/{id}`，比分页扫描更准、更
    省，同时消除假 404；
  - TokensPage：选择 Specified 需要完整集合，采用固定 500/页循环拉全量，
    空页终止防损坏服务端导致死循环。
- 材料风险/权衡：
  - 全量拉取在项目数很大时会产生多次请求；当前别无选择（无按名/搜索接口），
    且数量级为服务端上限 500 的页次，记录为已知权衡；
  - 无 `X-Total-Count` 的旧服务端/测试 mock 会回退单页，保证兼容不报错。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-cli-project-pagination | CLI `list_projects` 分页拉取全部可见项目（`X-Total-Count` 驱动、缺失回退单页），`resolve_project` 第 101+ 项目按名可解析 | 仅 cli/src/apiclient/mod.rs 与对应 mock/契约文档；公开签名与方法名不变 | 增加分页请求次数换取全量可见项目 | resolve 行为测试跨页通过；实测 mock 无总量头仍按单页回退 | 不加服务端按名过滤；不改命令参数 |
| P-002 | fh-web-project-pagination-consumers | `ApiClient.getProject` + 详情页直查 `GET /projects/{id}`；`ApiClient.listAllProjects` + TokensPage 全量项目勾选/展示 | 仅 admin-web client、两个页面与契约文档；ProjectsPage 分页行为不变 | 详情页零列表依赖；Token 页按需拉取多页 | 超首屏项目详情可打开；Specified 可选择全部可见项目 | 不加项目管理页新 UI；不迁移 listProjects 存量调用 |
| P-003 | fh-pagination-consumer-tests | CLI mock 分页 + 跨页解析用例；admin-web client/组件/契约测试适配与补用例 | 仅任务列出的 cli 与 admin-web 测试文件 | 用契约桩覆盖分页与回退路径 | 目标 cargo 与 npm 用例通过；既有用例零回归 | 不引入 e2e 服务端大批量造数 |

## Success Criteria

- CLI：`list_projects` 返回全部可见项目；`resolve_project` 能解析第 101+ 个
  任意位置的项目名，重名/不存在语义不变；
- ProjectDetailPage：打开第 101+ 个项目不再因首屏截断显示假 404，错误路径
  语义（无权限/不存在）不变；
- TokensPage：项目勾选列表与 Specified scope 名称展示覆盖全部可见项目；
- `ApiClient` 新方法在 `X-Total-Count` 缺失时安全回退，服务端未升版/旧 mock
  场景不抛错；
- 目标验证：`cargo test`（CLI 相关用例）通过；`npm run test:unit`、
  `npm run test:integration`、`npm run build` 通过；
- 按 standard 流程产出 `docs/changes/047-project-pagination-all-consumers.md`
  与任务包 `completion-report.md`，经 `lower-tier-check.py` 校验通过。

## Risks

- 工作树存在 025-046 等在制未提交改动，本任务只改 scope 列出的消费端文件；
  全量测试以目标用例 + `npm run build` + 既有 token/detail 组件回归为准并如实
  记录；
- `X-Total-Count` 依赖服务端 040 契约；旧服务端回退单页等价于现状语义，不
  抛出传输错误（兼容性取舍）；
- 项目量极大时 CLI/Token 页请求次数线性增长（每页 500），记录为已知权衡；
  服务端不承诺按名搜索，否则需新契约（明确 non-goal）。

## Resolved Decisions（2026-08-26 用户「确认」）

- 无待确认问题；实现方向（CLI/Token 页分页拉全量、详情页直查 `GET /projects/{id}`）
  与 040/041 已冻结契约一致，按本提案执行。
