# 025-token-project-scope-enforcement 验收报告

## Findings

| ID | Severity | Owning Stage | Correctness Category | Evidence | Problem | Blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-000 | none | none | overall | 独立反例搜索覆盖 proposal/design/实现/测试与契约检查（resolve 链路、checker 判定、负向编译夹具、removed-symbol 扫描、统一测试运行产物） | 未发现本任务范围内的缺陷；两处存量非目标边界（Feature 级 delete 路径、空 Specified 输入）已在对应缺陷发现分类与后续任务建议中记录，不作为本任务缺陷 | no |

## Object and Scope

- Task manifest: task.yaml
- Review date: 2026-08-23
- In-scope implementation:
  - `server/src/tokens/model.rs`：TokenPayload 移除 scopes，TokenPrincipal 新增 project_scope
  - `server/src/tokens/service.rs`：create/update/rotate 签发载荷收窄；resolve 从 token_scopes/tokens 读权限
  - `server/src/model/principal.rs`：Principal::Token 新增 project_scope
  - `server/src/http/auth.rs`：认证桥透传 project_scope
  - `server/src/permissions/checker.rs`：Project 资源访问的 fail-closed 项目范围校验
  - `docs/api/v1-contract.md`：JWT claims 不再携带权限属性说明
- Review mode: independent falsification（acceptance 独立阶段从提案/设计原文出发，对 resolve、checker、签发三条链路做反例搜索并核对消费者迁移与契约检查，未直接采信实现阶段自评；单智能体环境由 acceptance owner 执行独立审查）

## Requirement Coverage

| change_id | Requirement or Boundary | Source | Implementation Evidence | Finding | Status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-token-permissions-server-side | JWT 不含权限属性；TokenPrincipal/Principal::Token 携带 scopes+project_scope；resolve 数据库权威 | `proposal.md` P-001 + `design/tokens.md`、`design/model.md`、`design/http.md` | `server/src/tokens/model.rs:31-46`（TokenPayload 无 scopes、TokenPrincipal 带 project_scope）；`server/src/tokens/service.rs` create/update/rotate 载荷收窄、resolve 读取 project_scope 并调用 load_scopes；`server/src/http/auth.rs:34-40` 透传 | 无缺陷；新增回归测试 token_permissions_are_read_from_db_not_jwt 同时断言 claims 无权限字段与 DB 权威 | pass |
| fh-token-project-scope-enforce | checker 对 Token 访问 Project 资源先校验 project_scope 包含目标项目（fail closed） | `proposal.md` P-002 + `design/permissions.md` | `server/src/permissions/checker.rs`（token_in_project_scope + `can_access` Project 分支先于 scope/用户权限判定）；回归测试 token_project_scope_restricts_access_outside_scope 覆盖 Specified 内外 x read/write/admin 与 All 对照 | 无缺陷；Feature 级 delete 路径为提案明示非目标的存量边界，已在后续任务建议中记录 | pass |
| fh-token-project-scope-tests | 回归测试覆盖三层与 JWT 无权限路径；更新 API 契约文档 | `proposal.md` P-003 + testing.md/testplan.yaml | `server/tests/unit/{tokens,permissions,versions}.rs` 更新与新增断言；`docs/api/v1-contract.md` claims 说明；统一运行 `.harness/test-results/test-runs/20260823T093746Z-filehub+025-token-project-scope-enforcement-all.json` 7/7 通过（4 contract + 3 level） | 无缺陷 | pass |

## Independent Defect Discovery

| Category | Applicable Scope | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|------------------|--------------------|-------------------|----------------------------------|--------|
| requirement-and-behavior | 提案 P-001/P-002/P-003 与用户三条要求（修复 project_scope、权限不进 JWT、scopes 也去掉且不考虑兼容） | `proposal.md` Scope/Success Criteria、`server/src/tokens/{model,service}.rs`、checker.rs、测试 | 正反两侧核对：用户要求的每一项都有实现与断言；未发现超出非目标（Feature 级动作、session 语义、schema）的行为变更 | 无缺陷；删除路径非目标已记 F-001 | pass |
| logic-and-control-flow | resolve 判序与 checker 分支顺序 | `server/src/tokens/service.rs:323-377`、`server/src/permissions/checker.rs:107-143` | 构造分支假想：claims 校验、DB 读取、project_scope->scope->用户权限三层顺序、Specified 集合 contains | 顺序固定为拒绝优先，任何一层失败返回 false/认证失败；未发现漏放行分支 | pass |
| boundary-and-input | Specified/All 边界、claims 缺字段、非法存储值 | `model/scope.rs` FromStr、`tokens/model.rs` serde、新增断言 | Specified 不含目标、空集合语义、旧 JWT 未知字段、非法 project_scope 存储值 | 空集合经 UI 约束且 FromStr 拒绝（fail-closed）；旧 claims 未知字段被 serde 忽略；无绕过 | pass |
| state-and-data-integrity | tokens/token_scopes 持久化数据与重签/轮换 | `server/src/tokens/service.rs` create/update/rotate/resolve、token_lifecycle 测试 | 权限变化后旧 JWT 是否仍有效、DB 与 claims 是否双源、事务一致性 | 权限变更重签换公钥使旧 JWT 失效；resolve 以 DB 为准无双源；事务边界未变 | pass |
| error-handling-and-recovery | resolve 失败路径与错误映射 | `service.rs` resolve 各 Err 分支、tokens/http.rs token_error_to_api | 无 token/已撤销/验签失败/claims 不一致/DB 读取失败 | 失败时返回错误，不产生放行 principal；HTTP 映射未变 | pass |
| resource-lifetime-and-cleanup | 资源获取释放与事务生命周期边界 | 变更 diff（仅只读 SQL + 既有事务） | 是否引入新句柄/任务/定时器/连接 | not-applicable: 本次变更未新增任何资源获取/释放、任务或连接生命周期管理，只读查询复用 pool，事务沿用既有 begin/commit；无新增清理路径可审查 | not-applicable |
| concurrency-and-ordering | token 解析与重签轮换的并发及顺序边界 | service.rs update/rotate、resolve 读库路径、SQLite 事务 | 并发 update/revoke 下旧 JWT 时效、读读一致性 | resolve 每次独立读库取最新公钥/撤销/权限；重签与撤销的既有顺序语义未改；未发现新竞态 | pass |
| interface-and-compatibility | 服务端类型形状与 HTTP/JWT 契约 | `model/principal.rs`、`tokens/model.rs`、tests、负向夹具 `testing/negative-old-token-principal.sh`、consumer-closure-check | 消费者是否漏迁移、旧形状是否仍被引用、HTTP JSON 与 trait 签名是否变化 | 三个仓库内构造点全部迁移；负向编译夹具确认旧 Principal::Token 被拒；removed-symbol 扫描通过；HTTP JSON 与 trait 签名不变；JWT claims 收窄不构成对外契约（用户明确不考虑兼容） | pass |
| security-and-capacity | 授权边界、越权路径与资源耗尽风险面 | checker.rs、projects/service.rs、versions/service.rs、permissions 测试 | 尝试构造 scope 外项目 + 各种 scope/用户权限组合的绕过；检查 Feature 与 Project 两条资源路径 | 范围内 Project 资源路径通过 fail-closed 校验无绕过；存量非目标边界：`projects/service.rs:127-133` delete 仅走 Feature 判定（账号 Owner + scope），后续任务建议补充项目级/项目范围校验；无注入/遍历/放大类新暴露 | pass |
| test-adequacy | 新增与既有测试能否暴露缺陷 | `server/tests/unit/{tokens,permissions,versions}.rs` 断言、testplan.yaml 4 个 contract 步骤、测试运行产物 | 评估缺失 claims 字段、DB 权威、范围外拒绝、重签失效、旧形状编译失败是否可见 | 各失败模式均有可直接失败的断言或编译检查；既有 unit/dv/integration 覆盖回归；未发现可逃逸的重要失败面 | pass |

## Document Consistency

| Document | Source | Implementation Consistency | Finding | Status |
|----------|--------|----------------------------|---------|--------|
| design | `design.md`（含 design/tokens.md、design/model.md、design/permissions.md、design/http.md） | 实现遵循设计：TokenPayload 去 scopes、resolve 查库、Principal 透传、checker fail-closed、Consumer Migration Closure 三行旧符号全部迁移 | 无 mismatch | pass |
| testing | `testing.md` + `testplan.yaml` | 新增断言与测试文档一致；Direct Change Coverage/Case-Type 表与 testplan 步骤一一对应；统一入口全部通过 | 无 mismatch | pass |

## Result Summary

- Overall result: accepted
- Outcome: 本任务范围内授权修复通过独立反例搜索、回归测试与契约检查，无阻塞缺陷，接受交付
- Blocking issues: none recorded
- Next action: 完成 lifecycle 收尾并从任务索引移除；后续任务建议（不阻塞本次交付）：1) `projects.service.rs` delete 增加项目级/项目范围校验；2) 服务端拒绝空的 `Specified` 项目集合输入

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 授权绕过主路径（JWT 去权限、resolve 数据库权威、checker 项目范围 fail-closed）已实现并有单元反例断言 + 负向编译夹具 + removed-symbol 扫描 + 全套回归通过；验收反例搜索未发现本任务范围内缺陷；两处存量非目标边界已记录为后续任务建议。
