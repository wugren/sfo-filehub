---
task_manifest: task.yaml
status: approved
---

# admin-web 项目列表分页（承接 040 服务端分页）

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-25 回复「确认」，确认采纳本提案
  （ProjectsPage 分页加载与翻页控件、总量显示、边界行为、client 分页方法与
  前端测试）并接受建议的 standard 层级；待确认问题按推荐值执行：每页 10 条、
  不新增服务端 CORS expose。
- Tier rationale / triggered boundaries:
  - 不满足 trivial：涉及前端页面导航/交互（分页控件、空页回退、新建后跳页）、
    i18n 文案与前端单元/集成测试，属于 UI 工作流改动；
  - 未触发 high-risk：后端分页接口与 `Project[]` + `X-Total-Count` 契约已在
    040 落地并测试，本任务不改变公开 API、schema、依赖图或部署结构；改动收敛
    在 admin-web 一个页面、api-client 及对应测试内，且保留既有 `listProjects`
    语义供其他消费方使用。

## Background and Goal

- 040 之后服务端 `GET /api/v1/projects` 已支持 `?limit=`/`?offset=` 并返回
  `X-Total-Count`，但 admin-web `ProjectsPage` 仍调用不传分页参数的
  `listProjects`，只能看到首屏（服务端默认 100 条），列表页没有翻页能力。
- 目标：`ProjectsPage` 使用分页参数加载任意页，显示总量与上下页导航；新建项目
  后跳到末页可见，删除后当前页为空时自动回到上一页；`ApiClient` 新增读取总量
  头的分页方法，并补齐前端单元/契约测试。

## Scope

### In scope

- `admin-web/src/api/contract.ts` / `client.ts`：新增 `ProjectPage` DTO 与
  `listProjectsPage(bearer, {limit, offset})`，请求携带 `?limit=&offset=`，
  从 `X-Total-Count` 响应头解析总量；既有 `listProjects(bearer)` 保持不变，
  供 TokensPage/ProjectDetailPage/既有测试继续使用；
- `admin-web/src/pages/ProjectsPage.tsx`：
  - 分页状态（当前页、总量），加载 `limit`（建议 10）对应页；
  - 表格下方「上一页/下一页」导航与「第 x / y 页 · 共 n 个项目」信息；
  - `count-badge` 改为显示总量；
  - 边界行为：第一页禁用上一页、末页禁用下一页；新建成功后跳到末页；
    删除导致当前页为空且不是第一页时回退一页；
- `admin-web/src/i18n/messages.ts`：新增上一页/下一页/页码信息文案（中英双语）；
- `admin-web/src/styles.css`：增加轻量 `.pager` 布局样式；
- `docs/api/v1-contract.md`：消费对齐段落注明 admin-web 已走分页参数与总量头；
- 测试：
  - `admin-web/tests/unit/client.test.ts`：`listProjectsPage` 拼接
    `limit/offset`、解析 `X-Total-Count`、缺失总量头回退等；
  - `admin-web/tests/integration/contract.test.ts`：契约桩支持
    `limit/offset` 与 `X-Total-Count`，新增分页调用断言；
  - 新增 `admin-web/tests/unit/components/ProjectsPage.test.tsx`：分页控件
    渲染、翻页触发新的 limit/offset 请求、边界禁用、删除后空页回退。

### Out of scope

- 不改服务端 040 已交付的分页/总量逻辑；
- 不迁移 TokensPage 项目选择器与 ProjectDetailPage 的 `listProjects` 首屏消费
  （它们仍沿用既有方法，超首屏场景列为后续任务）；
- 不改 CLI 分页/按名扫描；
- 不加每页条数设置项（固定页大小常量）；
- 不处理跨域部署下 `X-Total-Count` 的 CORS expose（当前 nginx 同源部署；
  如后续跨域独立部署再补，见待确认问题）。

## Boundary with neighboring modules

- `ApiClient` 是 admin-web 唯一 API 传输层：新方法只增不改既有方法签名，
  `TokensPage`/`ProjectDetailPage` 及既有 mock 不受影响；
- 服务端契约已在 `docs/api/v1-contract.md` 冻结（`Project[]` + `X-Total-Count`），
  本任务只按该契约消费，不新增服务端路由/字段。

## Requirement Review

- 需求合理：服务端分页已就绪，前端不消费则 100 条后的项目对管理后台不可见，
  分页能力不完整。
- 方向选择：页大小做成模块常量（默认 10）并复用现有 `Btn`/i18n；删除空页
  回退与新建跳末页是列表页分页的标准边界行为，直接纳入本次交付。
- 材料风险/权衡：
  - 每页 10 条会增加翻页次数，但表格可读性更好；如偏好 20/50 可在确认时
    调整常量；
  - `X-Total-Count` 在浏览器跨域环境读取需要服务端 CORS expose；当前部署
    为 nginx 同源代理，暂不做服务端改动。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-web-project-pagination | `ApiClient` 新增 `listProjectsPage`（limit/offset + `X-Total-Count` 解析），`ProjectsPage` 分页加载、翻页控件、总量显示、空页回退/新建跳末页 | 仅 admin-web client/ProjectsPage/i18n/styles 与契约文档；既有 `listProjects` 不变 | 新增方法 + 页面状态提升；页大小固定为常量 | 页面可翻到任意页；count 显示总量；边界行为正确 | 不做页大小设置项；不迁移 TokensPage/Detail 消费 |
| P-002 | fh-web-project-pagination-tests | client 单测（参数/总量头/回退）、契约桩分页、ProjectsPage 组件测试 | 仅 `admin-web/tests/` 三个文件 | 用契约桩组件测试覆盖翻页与边界 | `npm run test:unit`/`test:integration` 对应用例通过；`npm run build` 类型检查通过 | 不重写既有 TokensPage/Detail 测试 |

## Success Criteria

- `ProjectsPage` 默认加载第 1 页（建议 10 条），`count-badge` 与页码信息显示
  服务端总量；
- 上一页/下一页翻页触发 `?limit=&offset=` 新请求，首末页对应按钮禁用；
- 新建项目成功后跳至末页并可见新项目；删除当前页最后一条且非第一页时回退
  一页；
- `listProjectsPage` 正确读取 `X-Total-Count`；存量 `listProjects` 调用方
  （TokensPage/Detail/既有测试）零变更、零回归；
- 目标命令通过：`npm run test:unit`、`npm run test:integration`、
  `npm run build`（类型检查 + 构建）；
- 按 standard 流程产出 `docs/changes/041-web-project-pagination.md` 与任务包
  `completion-report.md`，并经 lower-tier-check 校验。

## Risks

- `admin-web` 工作树存在 025-039 等在制未提交改动，本任务只改提案 Scope
  列出的文件；全量前端测试可能受在制任务影响，以目标用例 + 类型检查 + 既有
  token/detail 组件测试回归为准并如实记录；
- `X-Total-Count` 在同源 nginx 代理下可读；若未来改成跨域独立部署需服务端
  CORS expose（非阻塞记录）；
- 分页后 TokensPage 项目选择器/ProjectDetailPage 仍只看到首屏，已列为
  non-goal 的后续任务，不在本次验收范围内。

## Unresolved Questions（待用户确认）

1. 每页条数：推荐 10（常量 `PROJECT_PAGE_SIZE`）；也可以改为 20 或 50。
2. 是否本次同时把 `X-Total-Count` 加入服务端 CORS expose（建议否：当前部署
   为 nginx 同源代理，不读该跨域场景；若你们实际有跨域部署则选是）。

## Resolved Decisions（2026-08-25 用户「确认」）

- 每页条数：10（`PROJECT_PAGE_SIZE` 常量）。
- CORS expose：本次不加；跨域独立部署时由后续任务补
  `X-Total-Count` expose 配置。
