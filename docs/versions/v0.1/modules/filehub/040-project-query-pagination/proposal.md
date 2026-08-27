---
task_manifest: task.yaml
status: approved
---

# 项目查询分页与 `/projects/{id}` 直查修复（评审第 7 项）

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-25 回复「确认」，确认采纳本提案
  （项目列表单次 SQL 过滤 + limit/offset 分页 + X-Total-Count；`/projects/{id}`
  与 visibility 更新响应改为直查；默认 limit 100、上限 500；前端分页 UI
  本次不实现）并接受建议的 standard 层级。
- Tier rationale / triggered boundaries:
  - 不满足 trivial：改动涉及公开 v1 契约（`GET /projects` 新增可选
    `limit`/`offset` 与总量元数据）、安全边界（限制无上限列表扫描与响应体放大），
    并需要同步契约文档与回归测试；
  - 未触发 high-risk：修复消除而非新增攻击面；推荐方案 A 不改变成功响应形状
    （保持 `Project[]`，仅新增可选查询参数与 `X-Total-Count`）；不改
    schema/迁移、不新增依赖、不引入部署/回滚协调；影响收敛在 projects 子模块、
    契约文档与既有测试内。

## Background and Goal

- 现象（评审第 7 项，中危）：`SqliteProjectAccess::list_projects`
  （server/src/projects/service.rs）先读全部项目；`SqliteProjectService::list`
  再对每个项目调 `can_access`，每次权限检查重新查询该项目，登录用户还额外查询
  grant，形成 1 + 2N 次 SQL（匿名 1 + N），并返回无上限的响应体。
- `/api/v1/projects/{id}`（server/src/projects/http.rs GET 分支）没有直查，
  而是先调用完整 `list()` 再用 `find()` 过滤；`POST .../visibility` 返回更新后
  记录时同样复用 `list()`。因此单项目请求也要全表扫描全部项目并做全部权限判定。
- 目标：列表用一次按可见性过滤的 SQL 分页（默认/上限 limit，返回总量）；
  单项目接口直接按 id 查询，不再随项目总数线性放大查询与响应体。

## Scope

### In scope

- `server/src/projects/mod.rs` / `service.rs`：
  - `ProjectService` 新增 `get(project_id, actor)`，用同一可见性过滤 SQL
    （visibility / owner / `EXISTS project_grants` / token 项目范围）直接查询
    目标项目；
  - `list(actor, limit, offset)` 返回分页结果：一次
    `SELECT ... ORDER BY id LIMIT ? OFFSET ?` + 一次带同一过滤条件的
    `SELECT COUNT(*)`（共 2 次查询）；
  - `POST .../visibility` 响应改经 `get()` 取更新后记录，不再触发全表 `list()`。
- `server/src/projects/http.rs`：
  - `GET /projects` 解析可选 `limit`/`offset`（非法 422；limit 默认 100、
    上限 500），响应保持 `Project[]`，附加 `X-Total-Count`（方案 A，见
    待确认问题）；
  - `GET /projects/{id}` 改为调用 `get()`：命中返回 200；匿名不命中或
    private 返回 401；已认证返回 404，保持现状语义。
- `docs/api/v1-contract.md`：更新项目列表/单项目路由备注与分页参数说明。
- `server/tests/unit/projects.rs` / `server/tests/api_integration.rs`：
  分页（默认 limit、offset、total、稳定排序）、直查语义（owner/member/public/
  匿名 private/token 项目范围/不存在）与 visibility 响应回归；
  `server/tests/dv_tests.rs` 仅适配 `list` 新签名。

### Out of scope

- 不实现「所有登录用户可无上限创建项目」的配额或频率限制（评审句中的独立
  问题，需要产品口径另行确认）；
- 不改 `tokens`/`versions`/`collaborators` 等其它列表接口的分页；
- 若维持方案 A，不改 admin-web/CLI 的分页 UI 与适配逻辑——默认调用只显示
  第一页，显式翻页/按名过滤列为后续任务；
- 不触碰 025-039 等在制未提交任务改动；不运行仓库级格式化。

## Boundary with neighboring modules

- 权限语义唯一收口仍为 `SqlitePermissionChecker` 的
  `decide_project_access`；SQL 过滤只是把同一判定下沉为 join/exists，实现后
  用既有 permissions 用例 + 新增直查回归交叉验证等价性；
- permissions 侧的 `ProjectAccess` 只读端口保持现状，projects 子模块新增
  查询不改变模块依赖方向。

## Requirement Review

- 需求合理：列表一次过滤 + 分页把 1+2N 查询收敛为常数次；单项目接口不应
  再依赖全表扫描。`get()` 直接查询并保持 401/404 信息隐藏语义。
- 方向选择：SQL 过滤优先于「全量 list 后逐条 can_access」，否则 N+1 依旧；
  `ORDER BY id + LIMIT/OFFSET` 提供确定顺序与稳定分页。
- 材料风险/权衡：
  - 默认 limit 会截断现有 web/CLI 不传参调用（首屏只显示一页）；
    `X-Total-Count` 使客户端可继续翻页，前端分页 UI 列为后续任务；
  - token `Specified(ids)` 项目范围需并入过滤条件，保持契约语义不变
    （空集合等价 All）；
  - 偏移分页对极大 offset 仍会扫描前序行，当前项目量级不构成问题；如需要
    游标分页可后续独立任务。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-project-query-pagination | 列表单次 SQL 过滤 + `limit`/`offset` 分页 + total；单项目与 visibility 更新响应改为直查 | 仅 projects 子模块与契约文档；响应保持 `Project[]` | 新增查询参数与总量响应头属于有利后兼容的契约扩展 | list 固定 2 次 SQL、`GET /{id}` 固定 1 次授权 SQL；权限结果与现有一致 | 不限制项目创建配额；不重排其它列表接口 |
| P-002 | fh-project-query-pagination-tests | 分页与直查语义回归，并验证 SQL 过滤与权限判定等价 | `server/tests/unit/projects.rs`、`server/tests/api_integration.rs` 新增/回归；`server/tests/dv_tests.rs` 仅签名适配 | 复用既有权限判定作为 SQL 过滤的可执行基准 | 新增用例通过；既有 projects/permissions/API 用例不回归 | 不新增 mock 基建或新依赖 |

## Success Criteria

- `GET /api/v1/projects` 未带参数时返回默认 limit 内条目并带 `X-Total-Count`；
  `?limit=&offset=` 生效；非法参数 422；limit 上限 500；
- 列表/单项目权限结果与现有一致：public 匿名可见；owner/任意 grant 协作者对
  session 可见；token 受 scopes 与项目范围约束；不存在或不可见的详情在已认证
  时 404、匿名时 401；
- `GET /api/v1/projects/{id}` 与 `POST .../visibility` 不再触发全表 `list()`；
- 目标命令通过：`cargo test -p filehub-server` 的 projects/permissions/
  api_integration 相关用例及编译检查（全量结果受 036 等在制任务影响，实现阶段
  实测后如实记录）；
- 按 standard 流程产出 `docs/changes/040-project-query-pagination.md` 与任务包
  `completion-report.md`（中文正文），并经 lower-tier-check 校验。

## Risks

- 工作树存在大量未提交在制改动（025-039 等），本任务只改提案 Scope 列出的
  文件；全量测试可能受在制任务影响，以目标用例 + 编译 + 既有 projects/
  permissions 回归为准并如实记录；
- 默认分页会截断不传参数的旧调用，web/CLI 的显式翻页/按名过滤暂列后续任务
  （若用户确认方案 B，则消费者同步修改上升为本任务 in-scope）；
- SQL 过滤必须与 `decide_project_access` 保持等价，用既有权限单测 + 新增
  直查回归交叉验证，防止权限语义漂移。

## Unresolved Questions（待用户确认）

1. 列表响应形状（二选一）：
   - A.（推荐）保持 `200 Project[]` + `?limit/offset` + `X-Total-Count`：
     完全向后兼容，web/CLI 不改代码即可继续工作，首屏展示默认页；
   - B. 改为 `200 {"items": Project[], "total": <count>, "limit": <n>,
     "offset": <n>}` 分页对象：语义更标准，但属 v1 破缺，需同步修改
     admin-web/CLI 解析与契约测试。
2. 是否需要本次顺带实现 admin-web 的分页 UI（推荐否：服务端分页先落地，
   前端翻页单独任务）。

## Resolved Decisions（2026-08-25 用户「确认」）

- 响应形状：A。保持 `200 Project[]` + `?limit/offset` + `X-Total-Count`，
  web/CLI 继续按既有 `Project[]` 解析，默认调用只展示第一页。
- admin-web 分页 UI：本次不实现；显式翻页/CLI 按名扫描列为后续任务。
