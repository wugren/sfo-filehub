# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/035-remove-account-level-roles.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - 账号级 `AccountRole{Owner,Member}` 及其全部依赖被移除：`account_roles` 建表、
    `[users].role` 配置、`Principal::User.account_role`、`role_for_user`/`role_of`
    桥接全部删除，`Principal::User` 收敛为 `{ user_id }`；
  - `projects:create` 对所有已登录账号放行（Anonymous 与无 scope 的 token 仍
    拒绝），创建者成为项目 owner；
  - `projects:delete` 改为项目级动作：仅项目 owner 可删，admin 协作者不可删；
    token 需 `projects:delete` + `administration` scope 且所属用户为目标项目
    owner、范围覆盖目标项目；
  - 单元/集成/DV 测试同步：member 创建项目由 403 改为 201（owner 为创建者），
    token 删除矩阵覆盖 owner-only 与 scope 边界；权限设计/模块/API 契约文档更新。
- Handoff: `cargo test -p filehub-server` 全量通过（unit_tests 38、api_integration
  4、dv_tests 2，exit 0）；`admin-web` `npm run test:unit` 44/44 通过；`server/src`
  与实时文档中不再出现 `AccountRole`/`account_role`/`account_roles` 引用；改动未
  触碰 026-034 等在制未提交内容。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-remove-account-level-roles | 删除账号级角色类型/表/配置与注入判定路径，User 不带角色 | proposal.md P-001 | `role.rs`/`config.rs`/`principal.rs`/`0003` 移除账号角色；auth/contract 删除 `role_of`；dep 扫描零残留 | 匹配 | pass |
| fh-project-create-open-to-all | 任意已登录账号可创建并成为 owner；Anonymous/无 scope token 拒绝 | proposal.md P-002 | `checker.rs` Feature 分支 User 放行、token 仅 scope；`permissions.rs` 新用例 + `api_integration` bob 创建 201 且 owner=2 | 匹配 | pass |
| fh-project-delete-owner-only | 删除仅项目 owner；token 需双 scope 且所属用户为 owner、范围覆盖 | proposal.md P-003 | `checker.rs` 新增 `projects:delete` owner 动作；`projects/service.rs` 项目级判定；`projects.rs` 删除矩阵全绿 | 匹配 | pass |
| fh-role-cleanup-tests-docs | 测试与实时文档删除账号角色描述并锁新矩阵 | proposal.md P-004 | 38+4+2 服务端与 44 前端用例通过；design/permissions.md、design.md、model/http/account/testing.md、api-v1-contract、modules/filehub.md 更新 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `checker.rs` can_access 三分支、`projects/service.rs` create/delete、token scope 映射、auth/contract 装配链 | 反向推演：任意 User 是否误链到删除动作、token 是否绕过 scope 或 owner、Anonymous 是否可创建/删除、admin 协作者是否误删 | Feature 分支仅 user 放行创建；删除必须命中 owner 允许列表且 token 过 scope/owner；admin 协作者与 Anonymous 全拒；无绕过路径 | pass |
| boundaries-and-failure-paths | 删除矩阵用例（非 owner/缺 scope/范围外/非 owner token/missing）、配置无 role、0003 只落 project_grants | 检查缺失项目时 User 与 token 的错误语义、旧配置含 role 的 serde 拒绝、存量库残留表是否被引用、scope 变更后 resolve 是否受影响 | token 路径缺失项目保留 not_found；User 路径缺失项目返回 forbidden（permission-first，已记录 F-3）；旧配置 role 键启动失败属预期迁移（F-2）；残留表不引用 | pass |
| regression-and-side-effects | `cargo test -p filehub-server` 全部二进制、admin-web 44 用例、全仓 `rg` 残留扫描、git 未提交在制文件清单 | 核对 token/versions/projects 其余用例是否依赖账号角色、前端是否消费账号角色、026-034 在制改动是否被连带格式化 | 44 服务端 + 44 前端全绿；cli/admin-web 无账号角色引用；除 common/mod.rs rustfmt 外未触碰无关在制内容 | pass |

## Verification

- Targeted check: `cargo test -p filehub-server` 全量通过（unit_tests 38/38、
  api_integration 4/4、dv_tests 2/2）；`admin-web` `npm run test:unit` 44/44；
  `server/src` 与实时 docs 的账号角色引用清零
- Result: pass
- Exception reason: not-applicable

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 0003 迁移仅删除建表语句，未提供 DROP | 已运行过旧 0003 的部署库会残留未被引用的空 `account_roles` 表；运行时无影响，物理清理需另立任务 | no |
| F-2 | low | `UserConfig` 删除 `role` 字段（serde 默认拒绝未知键） | 携带 `"role"` 键的旧配置文件启动失败；仓库内配置与文档已同步，属预期配置迁移 | no |
| F-3 | low | `ProjectService::delete` 对 User 先做项目级权限判定 | 普通用户删除不存在的项目返回 forbidden 而非 not_found（token 路径仍返回 not_found）；语义与既有 set_visibility 的 permission-first 风格一致，未超出提案范围 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 提案 P-001~P-004 全部落地：账号级角色移除彻底、创建对全员开放且创建者
  成为 owner、删除收紧为项目 owner-only 且 token 边界完整，服务端 44 用例与前端
  44 用例全绿；独立缺陷发现覆盖行为逻辑、边界失败路径与回归副作用，F-1~F-3 均为
  无阻塞低危记录项，不阻止收尾。
