# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/040-project-query-pagination.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `server/src/projects/mod.rs`：`ProjectService` 新增
    `get(project_id, actor)`，`list` 改为 `list(actor, limit, offset) ->
    ProjectPage { items, total }`；
  - `server/src/projects/service.rs`：可见性判定下沉为单一 SQL 过滤
    （public / owner / `EXISTS project_grants` / token 项目范围与 scope），
    列表固定 2 次 SQL（COUNT + `ORDER BY id LIMIT ? OFFSET ?`），单项目
    `get` 固定 1 次授权过滤直查；token `Specified(空集合)` 语义与
    `token_in_project_scope` 一致（不放行），避免生成非法 `IN ()`；
  - `server/src/projects/http.rs`：`GET /projects` 支持可选
    `?limit=`（默认 100、上限 500）/`?offset=`（默认 0，非法 422），响应保持
    `Project[]` 并附加 `X-Total-Count` 头；`GET /projects/{id}` 与
    `POST .../visibility` 响应改经 `get()` 直查，401/404 语义不变；
  - `docs/api/v1-contract.md`：新增分页参数、总量头与直查说明；
  - `server/tests/unit/projects.rs` / `api_integration.rs`：新增分页、直查、
    token 范围与空集合边界、非法参数、visibility 回归用例；
    `server/tests/dv_tests.rs` 仅适配 `list` 新签名。
- Handoff:
  - `cargo check -p filehub-server` 通过（无本任务新增 warning）；
  - `cargo test -p filehub-server --test unit_tests -- --test-threads=1`
    48/48 通过；
  - `cargo test -p filehub-server --test api_integration -- --test-threads=1`
    7/7 通过；
  - `cargo test -p filehub-server --test dv_tests dv_persistence_across_reopen`
    通过。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-project-query-pagination | 列表单次 SQL 授权过滤 + `limit`/`offset` 分页 + `X-Total-Count`；`GET /{id}` 与 visibility 更新响应改直查；401/404 语义不变 | proposal.md P-001 | service.rs `auth_filter`/COUNT+LIMIT 查询与 `get` 直查；http.rs 分页参数、非法 422、总量头与直查 handler；契约文档同步 | 匹配 | pass |
| fh-project-query-pagination-tests | 分页/直查/权限过滤等价回归（含 token 空 Specified 边界与非法参数） | proposal.md P-002 | unit `project_list_pagination_and_get`、api `project_list_pagination_and_single_get`、dv 签名适配；unit 48/48、api 7/7 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | 逐分支对照 `auth_filter` 与 `decide_project_access`：Anonymous/User/Token 的 public、owner、grant、token scope 与 project_scope 判定顺序；sqlx 动态 SQL 只拼接固定片段，所有值均走绑定参数 | 反向推演：token 缺 `metadata:read` 时必须先于 project_scope 短路；public 项目在 token `Specified` 范围外必须排除；任意 grant 角色都可 `metadata:read`；空 `Specified` 集合不能放行也不能生成 `IN ()`（实现期间发现并修复，新增回归断言） | 三身份过滤语义与权限判定一致；空集合边界已处理并被用例覆盖；动态 SQL 无用户输入拼接；list 固定 2 次 SQL、get 固定 1 次 | pass |
| boundaries-and-failure-paths | 复查 limit=0/501、负 offset、非数字 limit、缺省 limit/offset、越界 offset 空页、匿名 private 401、已认证不可见/不存在 404、visibility 切换后匿名 401、token 范围直查 | 边界推演：`limit` 上限由 HTTP 层统一校验，服务层 `u32` 类型杜绝负数；`ORDER BY id` 保证分页稳定；`X-Total-Count` 用同一过滤条件的独立 COUNT；offset 越界返回空 items 但 total 正确 | 全部边界用例在 api/unit 中断言通过；无 off-by-one；信息隐藏语义（匿名 401、已认证 404）保持 | pass |
| regression-and-side-effects | 全库运行 unit 48/48、api 7/7；dv 受影响签名用例单独通过；检查 `ProjectService::list` 全部调用方（http.rs 3 处、dv/unit 适配）；检查 admin-web/CLI 消费形状仍为 `Project[]` | 排查：`list` 签名变更是否遗漏调用点（rg 全库逐一核对）；响应形状/`X-Total-Count` 是否破坏 web/CLI 既有解析；SQL 过滤是否改变 permissions 模块行为（未改 checker/ProjectAccess） | 调用点全部适配；契约形状未破缺，web/CLI 无需改代码；既有 permissions/projects/tokens/versions/upload 用例零回归；全量 dv 未重跑为预知在制并发问题（见 Verification/Exception） | pass |

## Verification

- Targeted check:
  - `cargo check -p filehub-server`：编译通过；
  - `cargo test -p filehub-server --test unit_tests -- --test-threads=1`：
    48 通过 / 0 失败（含新增 `project_list_pagination_and_get`）；
  - `cargo test -p filehub-server --test api_integration -- --test-threads=1`：
    7 通过 / 0 失败（含新增 `project_list_pagination_and_single_get`）；
  - `cargo test -p filehub-server --test dv_tests dv_persistence_across_reopen`：
    通过（`list` 新签名最小适配）。
- Result: pass
- Exception reason: 全量 dv_tests 未重跑：该套件存在同工作树另一在制事务化重构
  导致的单连接池超时（039 任务已记录）；本次仅重跑受签名变更影响的
  `dv_persistence_across_reopen` 并通过。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | admin-web `count-badge`/ProjectsPage 与 CLI `resolve_project` 均不传分页参数 | 默认 limit=100 后，旧调用只看到首屏，超出页项目不可见；已列为用户确认的后续任务（前端翻页/CLI 按名过滤或分页扫描），属已知取舍而非本任务缺陷 | no |
| F-2 | low | server CORS `expose_headers` 配置未含 `X-Total-Count`；admin-web 当前未读取该头 | 前端接入分页 UI 跨域读取总量头时需在服务端 expose；当前不阻塞接口契约（curl/同源可读） | no |
| F-3 | low | COUNT 与分页查询为两次独立 SQL，未置于快照事务 | 并发写入时 `total` 与当页 items 可能短暂不一致；对分页导航语义可接受，无需锁/事务同步 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001/P-002 全部落地：列表由 1+2N 查询收敛为 2 次固定 SQL，单项目与
  visibility 更新响应不再全表 `list()`，改为授权过滤直查；分页参数、上限、
  `X-Total-Count` 与 422 校验按批准方案实现，401/404 与权限语义保持等价；
  unit 48/48、api 7/7、dv 受影响用例通过；独立缺陷发现覆盖行为逻辑、边界
  路径与回归副作用，发现项 F-1~F-3 均为非阻塞记录并已列入后续任务。
