# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/029-public-read-logged-in.md

## Delivery Summary
- Outcome: `SqlitePermissionChecker::can_access` 的 `Resource::Project` 分支为
  已登录身份补齐 public 只读可见性：User 对 public 项目直接放行
  `metadata:read`/`artifacts:read`；Token 保持 `project_scope -> token scope`
  前置硬校验后对 public 放行只读。Anonymous、private 项目与写/管理动作行为
  不变。`projects.list`、`versions.list/get` 与下载路由统一经
  `can_access(metadata:read)`，登录后无需授权关系即可读取 public 项目的
  bug 关闭。
- Handoff: 修复、回归测试与标准流程产物均已完成；改动只落在权限检查器与
  相关测试，未触碰在途未提交的 025 token 项目范围改动的其它文件。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-permissions-public-read-login | User/Token 在各自前置校验后对 public 放行只读，private 与写/管理不变 | proposal.md P-001 | `checker.rs` Project 分支新增两处 public 只读放行，位于 token 的 project_scope/读 scope 校验之后、`project_permission` 之前；HTTP 读取/下载路由沿用 `can_access(metadata:read)` 生效 | 匹配 | pass |
| fh-permissions-public-read-tests | 无授权 User/Token 读 public 回归 + project_scope/缺 scope 边界 + list 过滤 | proposal.md P-002 | `server/tests/unit/permissions.rs` 增补 matrix 与 Specified 边界断言、只带单个读 scope 不放大断言；`server/tests/unit/projects.rs` 增补无授权 member list 断言 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `checker.rs` 完整 diff 与 can_access 三个 principal 分支；design/projects.md:70 与 001 permissions.md 访问矩阵；025 token project_scope 设计与测试 | 构造输入矩阵逐分支推演：public+匿名/无授权 User/All 范围 token 读放行；public+写/管理仍拒绝；private 无授权读拒绝；项目不存在返回 false；token 缺读 scope 仍拒绝；metadata-only token 的 artifacts:read 拒绝 | 所有输入路径与已确认契约一致，未发现漏放行或误拒绝分支 | pass |
| boundaries-and-failure-paths | versions/http.rs 下载与获取路由、versions/service.rs list/get、projects/service.rs list（全部走 metadata:read）；Visibility 枚举只有 Public/Private；db 查询错误传播路径 | 检查 public 只读放行是否误伤非读 action、Specified 范围外 public 是否被放宽、DB/查询失败是否掩盖为 false、无权用户项目 GET 保持 404 语义 | 边界无缺口：非读 action 必须满足角色/owner；Specified 范围外仍拒绝；错误仍传播；下载路由当前按 metadata:read 判定（存量行为，本次修复使其对登录身份同样生效） | pass |
| regression-and-side-effects | `cargo test -p filehub-server` 全量（24 unit + 2 api_integration + 2 dv）通过；git diff 确认仅 checker.rs 与两个测试文件被本任务修改；rustfmt 输出核对 | 逐项验证是否影响 JWT/claims、DB schema、HTTP 形状、项目删除/可见性切换、token 生命周期测试；核对新增代码块与 rustfmt 一致且未运行全仓格式化触碰在途改动；确认既有未提交 025/026 等脏文件未被改动 | 未发现回归：全量 server 测试通过，改动边界与提案一致，其余在途工作树文件保持原样 | pass |

## Verification
- Targeted check: `cargo test -p filehub-server`（24 unit + 2 api_integration +
  2 dv 全部通过）；追加 metadata-only token 断言后重跑
  `cargo test -p filehub-server --test unit_tests` 通过；跟踪 HTTP
  项目/版本/下载路由确认读取判定入口
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | versions/http.rs 下载路由经 `versions.get` 的 metadata:read 判定，而未单独调用 artifacts:read | 与 001 矩阵「artifacts:read 对应下载」措辞存在存量偏差；public 下载对匿名与登录身份当前均按 metadata:read 放行，不阻塞本修复 | no |
| F-2 | low | api_integration 用例覆盖登录/token 流程但未专门断言「无授权登录用户读 public 返回 200」 | 端到端场景由权限单元断言 + 全量集成回归推断；如需 HTTP 级显式断言可后续补一条集成用例 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 修复按已确认提案 P-001/P-002 精确落地，User/Token 公众可见性与
  token project_scope、读 scope 边界均有直接断言；全量 server 测试通过，
  三类独立缺陷发现（行为逻辑、边界失败路径、回归副作用）均 pass，剩余仅为
  两条非阻塞存量说明。
