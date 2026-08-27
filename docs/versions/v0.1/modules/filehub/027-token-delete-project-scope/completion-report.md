# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/027-token-delete-project-scope.md

## Delivery Summary
- Outcome:
  - `server/src/projects/service.rs::delete` 对 `Principal::Token` 追加项目级
    校验：`checker.can_access(Resource::Project(project_id), ADMIN)`（token
    project_scope 包含目标项目 + `administration` scope + 所属用户项目
    admin），范围外/无权限 403、项目不存在 404；User session 路径不变。
  - 空项目集合语义：`Specified([])` 等价 `All`。`ProjectScope::normalize()`
    与 `FromStr` 空值返回 All；create/update 落库前归一化为 `"all"`，旧空串
    数据按 All 解析；docs/api/v1-contract.md 补充语义说明。
- Handoff: `cargo test -p filehub-server` 全绿（23 unit + 2 dv + 2
  integration）；新增两条回归断言覆盖删除矩阵与空集合归一化；既有 026 任务
  包与历史脏文件（filehub-server.json、harness/scripts/edit-guard.py）未触碰。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-token-delete-project-gate | token 删除项目前追加项目级校验；不存在 404、无权限 403；User 路径不变 | proposal.md P-001 + In scope | `projects/service.rs::delete` 先 Feature 校验，再对 Token 走 `can_access(Project, ACTION_ADMIN)`，`project_access.project` 区分 404/403；`projects.rs` 新测试覆盖范围外/缺 administration/All/不存在四种反例 | 匹配 | pass |
| fh-token-empty-scope-all | `Specified([])` 与 All 等价；create/update 归一化落库 | proposal.md P-002 + In scope | `model/scope.rs::normalize` 与 `FromStr` 空值返回 All；`tokens/service.rs` create/update 在 `unwrap_or` 后 normalize；`tokens.rs` 新测试断言 resolve/list 为 All 且 DB 存 `"all"` | 匹配 | pass |
| fh-token-delete-empty-tests | 新增单元回归与契约文档说明 | proposal.md P-003 + Success Criteria | `server/tests/unit/projects.rs`、`server/tests/unit/tokens.rs` 新断言全绿；`docs/api/v1-contract.md` project_scope 语义行已补 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `projects/service.rs::delete` 全函数、`permissions/checker.rs` Project/Feature 分支、tokens create/update 归一化路径 | 逐分支推演：Token 删除在范围为、范围外、缺 administration scope、项目缺失、All 范围五种输入下的返回；空 Specified 经 create/update 后 resolve/list/DB 三处取值 | 五种反例均按预期返回（ok/403/404），空集合三处取值均为 All/`"all"`，无旁路 | pass |
| boundaries-and-failure-paths | `ProjectScope::from_str` 空串/全空分段、`normalize` 非空分支、旧空串 DB 数据解析 | 检查 `","`/空输入/混合空段输入是否被误判为 All 或报错、非空 Specified 是否被误归一化、User delete 路径是否被误改 | 空输入归一化为 All，非空集合保持 Specified，User 路径分支未触碰；无错误边界 | pass |
| regression-and-side-effects | git diff 全量、`cargo test -p filehub-server` 全套、docs/api 修改行 | 核对 025 的 token 权限链路是否受影响（resolve/claims 未改）、versions 删除依赖（引用清理语义不变）、既有项目 CRUD 测试是否仍通过 | 23 unit + 2 dv + 2 integration 全绿；040 系列既有行为未回归；仅本任务范围内文件变更 | pass |

## Verification
- Targeted check: `cargo test -p filehub-server --test unit_tests` 23/23
  通过（含新增 2 条）；`cargo test -p filehub-server` 全套通过
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 提案确认时默认按完整项目级校验（需 administration scope），用户未否定该默认 | 存量仅 `projects:delete` scope 的 token 删除项目将 403；属预期收紧，已在 change record 记录 | no |
| F-2 | medium | `projects/service.rs::delete` 仍不对 User session 做项目权属校验 | User session 删除维持账本 Owner 级 Feature 语义（非本项目范围），已记录为后续任务候选 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 两项需求均已按要求实现并经 23 条单元（含新增删除矩阵与空集合
  回归）+ 2 dv + 2 integration 全绿验证；独立缺陷发现覆盖行为逻辑、边界
  失败路径与回归副作用，未发现阻塞性缺陷；F-1/F-2 为非阻塞说明与后续项。
