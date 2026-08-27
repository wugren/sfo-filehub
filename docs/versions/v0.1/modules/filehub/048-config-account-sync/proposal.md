---
task_manifest: task.yaml
status: approved
---

# 048-config-account-sync：配置账号密码变更生效与删除停用（安全评审中危 #6）

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-26 回复「确认」，确认采纳本提案
  （配置同步校验/更新密码 hash、删除账号停用、不含立即吊销既有 token 的会话
  边界）并接受建议的 standard 层级。
- Tier rationale / triggered boundaries:
  - 不满足 trivial：本修复涉及 persistent schema 变更（`users` 增加 `active`
    列及幂等迁移）与安全/信任边界行为变更（配置密码同步语义、账号停用后登录
    拒绝），trivial 明确排除 schema/migration 与 security/privacy 影响；
  - 未触发 high-risk：变更收敛在 filehub 单项目内；`active` 为带默认值的新增
    列，启动时幂等 ALTER，不重写既有 0001-0007 迁移；不改公开 API/CLI 契约、
    不新增依赖、不改变文件/项目/版本数据语义、不修改 vendored `sfo-account`
    （避免影响该 crate 的其它嵌入方如 vpn-server）；与已完成同类安全修复
    （030 bcrypt 升级、046 登录加固）同构，采用 standard 连续流程并附定向
    回归测试。

## Background and Goal

- 现象（安全评审中危 #6，原句）：
  - `seed_user()` 发现同名账号后立即返回，甚至不会校验新配置的 bcrypt hash
    （证据：`server/src/account/mod.rs:73` 附近）；
  - Docker 每次启动根据 `FH_ADMIN_PASSWORD` 重新生成配置
    （证据：`docker/entrypoint.sh:63`），但已有数据库仍继续使用旧密码；
  - 从配置删除账号也不会停用数据库中的账号；
  - 当前又没有密码重置或账号删除接口，因此启动配置是账号管理的唯一事实来源。
- 目标：把「启动配置即事实来源」真正闭环——同名已存在账号按新配置校验并更新
  bcrypt hash；配置中已删除的账号在启动时停用（`active=0`，登录拒绝、账号查询
  视为不存在）；账号重新加入配置后恢复。修复后 `FH_ADMIN_PASSWORD` 或配置中的
  `password/password_hash` 变更在下一次启动即生效，无需手工改库。

## Scope

### In scope

1. schema（`fh-account-sync-schema`）：
   - 新增 `server/migrations/0008_accounts_active.sql`：`users` 增加
     `active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1))`；
   - `AccountModule::init` 在 0002 之后用 `PRAGMA table_info(users)` 守卫幂等
     执行 ALTER（SQLite 不支持 `ADD COLUMN IF NOT EXISTS`），新旧库同一路径；
   - 不修改既有 0001-0007 迁移文件。
2. store（`fh-account-sync-schema`）：
   - `FilehubAccount` 增加 `active: bool`；序列化跳过该字段且缺失时默认
     `true`，保证升级前已签发的 JWT 仍可解码（旧 claims 无此字段）；
   - `row_to_account`、`new_uncommitted`、`update_account` 携带并写回 `active`；
   - `get_account_by_name` 过滤 `active = 1`：停用账号在登录时走「账号不存在」
     等成本 dummy bcrypt 校验（与 046 语义一致，不引入枚举计时侧信道）；
   - 新增仅供 seed 使用的 `get_managed_account_by_name`（不过滤 active，用于
     更新/恢复已停用账号）与 `deactivate_not_in(names)`（把不在配置中的账号
     置为 `active=0`）。
3. seed 同步（`fh-account-sync-seed`，`server/src/account/mod.rs`）：
   - 抽出 `resolve_password_hash`（72 字节限制、bcrypt `HashParts` 完整解析、
     cost 4..=31 校验），创建与更新路径共用；
   - 同名已存在：`password` 配置先 `bcrypt::verify` 旧 hash，不匹配才重新
     hash 并通过 `update_account` 落库（避免密码未变时每次重启改写 hash）；
     `password_hash` 配置与库中 hash 不一致时校验后落库；两种情况同时恢复
     `active=1`；
   - 全部配置账号同步后，把 DB 中不在配置名单的账号一次性置为不活跃。
4. 停用语义（`fh-account-sync-seed`）：停用账号无法登录、对既有账号查询/授权
   存在性校验表现为不存在；既有 `project_grants`/`tokens`/`projects` 等外键
   引用不受破坏（保留行，不硬删除）。
5. 回归测试（`fh-account-sync-tests`，`server/tests/unit/account.rs`）：
   - 改密码 → 旧密码登录失败、新密码成功、库中 hash 已更新；
   - 密码未变 → 二次启动 hash 不变（idempotent，无写库 churn）；
   - 已存在账号配置非法 `password_hash` → 启动报错（不再静默跳过）；
   - 配置删除账号 → 重启后登录失败、`get_account_by_name` 返回 None；重加配置
     后恢复登录；
   - fresh 库与含既有数据的库迁移均幂等（连续两次 init 成功）。

### Out of scope（非目标）

- 不新增密码重置、账号删除的 HTTP/CLI API：本次以配置同步闭环解决事实来源
  问题；交互式改密/删除接口属于新产品能力，单独另行提案；
- 不修改 vendored `sfo-account`（`refresh_session` 等保持纯 JWT 语义），避免
  影响该 crate 的其它嵌入方；
- 不实现 session 立即吊销：停用后新登录与账号查询立即被拒，但升级前已签发、
  未过期的 session/refresh token 在各自有效期到期前仍可由 JWT 解码端读取；
  「停用范围是否包含立即吊销既有 token」见下方待确认问题；
- 不硬删除账号行，不引入级联清理；删除账号及其关联数据属独立任务。

## Requirement Review

- 合理性：评审发现的不一致真实存在——`seed_user` 的幂等只处理「存在即跳过」，
  把配置降级为「仅首次创建」，与 entrypoint 每次重生成配置的行为直接冲突；
  配置确实是当前唯一的账号管理通道。
- 关键权衡：
  - 启动成本：对每个配置账号做一次 bcrypt verify（或 hash），配置文件级账号
    数量很小（通常 1-2 个），cost=12 单次约 60-100ms，可接受；
  - 停用 vs 删除：停用保留行与外键引用，账号重加配置可整体恢复；与评审原文
    「停用」一致；
  - 不在配置即停用：当前应用无注册接口，DB 账号全部由配置 seed 产生，因此
    「配置名单为全量事实来源」成立，不会误伤正常数据。
- 方向选择：启动时配置同步（校验+更新 hash、停用缺失账号），最小改动且不改
  公开契约。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-account-sync-schema | `users` 新增 `active` 停用列（0008 幂等迁移，PRAGMA 守卫）+ `FilehubAccount.active` 字段；store 读写/过滤（`get_account_by_name` 仅活跃账号）/`get_managed_account_by_name`/`deactivate_not_in` | 仅 account 子模块；不动 0001-0007；不改 vendored sfo-account | 新增列带默认值 1，旧库自动补列、旧 JWT 缺失字段按活跃解码 | 迁移幂等（连续 init）；active 不进 JWT/响应；停用账号登录走等成本 dummy 校验 | 不改既有迁移文件；不做账号硬删除/级联清理 |
| P-002 | fh-account-sync-seed | 同名已存在账号校验并同步密码/password_hash（>72 字节与完整 bcrypt 解析/cost 4..=31 校验）、恢复 active=1；每轮 init 停用不在配置名单的账号 | 配置为账号唯一事实来源（当前无注册/重置/删除 API） | 启动时每账号一次 bcrypt verify（未变不重写 hash）；停用保留行与外键引用 | 改密码后旧密码登录失败、新密码成功；非法 hash 对已存在账号启动报错；删除账号后登录失败/查询 None，重加配置恢复 | 不新增密码重置/账号删除 API；不立即吊销已签发 JWT（会话边界由用户确认） |
| P-003 | fh-account-sync-tests | 回归用例：改密生效/旧密拒绝、hash 幂等、非法 hash 拒绝、删除停用+重加恢复、迁移幂等 | server/tests/unit/account.rs 内 | 以同一 DB 二次 assemble 模拟重启，避免启动真实进程的 flake | `cargo test -p filehub-server --test unit_tests` 全绿；canonical 集成入口通过 | 不新增 HTTP 集成用例（登录/会话已有 045/046 覆盖） |

## Success Criteria

- 同一 DB 重启验证：
  1. `alice/alice-pass` 启动后改配置为 `alice/alice-pass-new` 重启：旧密码登录
     失败，新密码成功，库中 `password_hash` 已更新；
  2. 配置含 alice+bob 启动后移除 bob 重启：bob 登录返回 err=10（用户名或密码
     错误），`get_account_by_name("bob")` 为 None；把 bob 加回配置重启后恢复
     登录；
  3. 已存在 alice 时把配置 `password_hash` 改为非法 bcrypt 字符串：启动报错，
     不再静默跳过；
  4. 密码未变更二次启动：`password_hash` 保持不变（无写库 churn）；
  5. `cargo test -p filehub-server` account 定向测试全绿，并通过仓库 canonical
     测试入口（`./test-run.sh` 的 filehub unit/account 对应项）。
- 非目标证据：不新增 API 文档/契约条目，不改 v1-contract，不改 vendored
  `sfo-account`。

## Risks

- 迁移幂等：SQLite 不支持 `ADD COLUMN IF NOT EXISTS`，一律以 PRAGMA 列探测
  守卫执行 ALTER；连续启动、fresh/已有库同路径，均有测试覆盖。
- 停用语义风险：若部署侧绕过配置直插 `users` 行，该行会被视为「不在配置」并
  停用；当前产品无注册接口，此语义与「配置为事实来源」一致，提案已载明。
- 会话边界风险：停用只切断登录/账号查询；已签发 token 到期前仍可解码
  （refresh/API 认证路径不受影响，见非目标）。如需立即吊销，认证桥需增加
  active 校验或改造 vendored refresh 语义（后者影响跨 crate，需单独评估）。
- 无硬删除，不存在级联数据丢失；旧库升级自动补列，无需人工重建。
