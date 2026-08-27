# 项目查询分页与 `/projects/{id}` 直查修复

- Status: complete
- Owner module: filehub（filehub-server projects 子模块）
- Task manifest: docs/versions/v0.1/modules/filehub/040-project-query-pagination/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/040-project-query-pagination/proposal.md
- Affected paths: server/src/projects/mod.rs、server/src/projects/service.rs、
  server/src/projects/http.rs、docs/api/v1-contract.md、server/tests/unit/projects.rs、
  server/tests/api_integration.rs、server/tests/dv_tests.rs（仅适配调用签名）
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- `ProjectService` 新增 `get(project_id, actor)`，`list` 改为
  `list(actor, limit, offset) -> ProjectPage { items, total }`；
- 把 `decide_project_access` 的可见性判定下沉为单一 SQL 过滤条件
  （visibility / owner / `EXISTS project_grants` / token 项目范围），列表
  固定 2 次 SQL（COUNT + 分页查询），单项目固定 1 次授权过滤直查；
- HTTP：`GET /projects` 解析可选 `limit`（默认 100、上限 500）/`offset`，
  非法参数 422，响应保持 `Project[]` 并附加 `X-Total-Count`；
  `GET /projects/{id}` 与 `POST .../visibility` 响应改经 `get()`，不再复用
  全表 `list()`；401/404 信息隐藏语义不变；
- 契约文档同步新增分页参数、总量头与直查说明；admin-web/CLI 保持既有
  `Project[]` 解析，不随本任务改 UI（记录为后续任务）；
- `list` 签名变更后 `server/tests/dv_tests.rs` 做最小适配（传 limit/offset、
  读 `.items`），无行为变更。

## Risk Screen

- Public contract, protocol, or CLI change: yes（有利后兼容扩展：新增可选查询
  参数与 `X-Total-Count` 响应头；成功响应形状 `Project[]` 不变，web/CLI
  既有解析继续可用）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: yes（消除列表/单项目的全表扫描与
  响应体放大，访问语义与 `decide_project_access` 保持一致并经权限用例交叉验证）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no（projects 子模块内收敛；
  permissions 只读端口与依赖方向不变）

## Verification

- Targeted check:
  - `cargo check -p filehub-server`：库与测试目标编译通过（8 个 warning 均为
    在制 tokens 模块既有未使用项，本任务未引入新 warning）；
  - `cargo test -p filehub-server --test unit_tests -- --test-threads=1`：
    48/48 通过（含新增 `project_list_pagination_and_get`）；
  - `cargo test -p filehub-server --test api_integration -- --test-threads=1`：
    7/7 通过（含新增 `project_list_pagination_and_single_get`）；
  - `cargo test -p filehub-server --test dv_tests dv_persistence_across_reopen`：
    通过（`list` 新签名最小适配）。
- Result: pass
- Residual risk or follow-up: 默认 limit 会截断不传参旧调用，web/CLI 显式翻页/
  按名过滤（CLI `resolve_project`）列为后续任务；SQL 过滤与权限判定等价性由
  既有 permissions 用例 + 新增直查/分页回归持续守护。
