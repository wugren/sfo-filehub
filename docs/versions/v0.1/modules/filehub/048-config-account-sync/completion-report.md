# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/048-config-account-sync.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `server/migrations/0008_accounts_active.sql`（新增）：`users.active` 停用列；
    `AccountModule::init` 以 PRAGMA 列探测守卫幂等 ALTER（SQLite 无
    `ADD COLUMN IF NOT EXISTS`），新库/已有库同路径，不改写 0001-0007；
  - `server/src/account/store.rs`：`FilehubAccount.active`（JWT/响应不序列化，
    旧 token 缺失默认活跃）纳入读写；`get_account_by_name` 过滤 `active=1`，
    停用账号走 cost=12 dummy 等成本校验；新增 `get_managed_account_by_name` 与
    `deactivate_not_in`；
  - `server/src/account/mod.rs`：`resolve_password_hash` 创建/更新共用（72 字节
    限制 + HashParts 完整解析 + cost 4..=31）；同名已存在账号不再直接返回——
    `password` 先验旧 hash、`password_hash` 始终完整校验，不一致才落库；更新
    路径恢复 `active=1`；每轮 init 收尾把不在配置名单的账号停用（保留行与引用）；
  - 测试：`server/tests/unit/account.rs` 新增 4 个回归用例并增强幂等断言，
    `second_init_is_idempotent` 验证密码未变不重写 hash。
- Handoff:
  `cargo test -p filehub-server --test unit_tests` 57/57 通过（account 15 项）；
  `cargo test -p filehub-server --test api_integration -- login` 4/4 通过；
  `python3 harness/scripts/test-run.py filehub integration`（canonical 串行）
  全绿，run artifact `.harness/test-results/test-runs/20260826T044302Z-filehub-integration.json`；
  `lower-tier-check.py pre-edit/completion` 通过。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-account-sync-schema | active 列+幂等迁移；FilehubAccount.active 携带/写回；常规查询过滤停用账号；停用方法 | proposal.md Item 1 | 0008 + `ensure_account_active_column` + store.rs 字段/查询/update/新方法；单元断言 active 不进 claims、二次 init 不重写 hash | 匹配 | pass |
| fh-account-sync-seed | 已存在账号校验并同步密码 hash、恢复激活；配置缺失账号启动停用 | proposal.md Item 2 | `upsert_seed_user`/`resolve_password_hash`/`deactivate_not_in`；改密、非法 hash 拒绝、删除停用/重加恢复用例通过 | 匹配 | pass |
| fh-account-sync-tests | 改密生效、hash 幂等、非法 hash 拒绝、删除停用/重加恢复、迁移幂等 | proposal.md Item 3 | account.rs 新增 `config_password_change_applies_on_reinit`、`config_hash_change_applies_and_invalid_hash_rejected_on_existing_account`、`removed_from_config_is_deactivated_and_readd_restores` 及 `second_init_is_idempotent` 增强 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|--------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `seed_user` 创建/更新两分支、`upsert_seed_user` 的 verify/重写判定、`resolve_password_hash` 校验链、`deactivate_not_in` 的 SQL、`ensure_account_active_column` PRAGMA 守卫、登录 `get_account_by_name` 过滤 + dummy 校验 | 反向推演：空配置名单是否合法（`UPDATE ... WHERE active=1` 全停用，SQL 有效）；名单仅 1 人时 `NOT IN (?)` 是否成立（测试覆盖通过）；停用账号重入配置是否恢复（用例覆盖）；>72 字节密码是否被跳过校验（两路径均先拒绝，verify 不可绕过）；重复用户名是否引入崩溃（更新路径幂等、不再触发 create 冲突，属记录的行为细化）；已停用行存在时 create_account 内部去重是否会 INSERT 冲突（seed 先经 get_managed 命中则绝不进入 create） | 无绕过；重复用户名从「启动报错」变为「幂等覆盖/无操作」属接受的边界细化（F-2） | pass |
| boundaries-and-failure-paths | 迁移幂等（连续二次 init、fresh/已有库）、旧 JWT claims 无 active 字段的解码兼容、停用后常规查询 None 与登录统一 err=10、`active` serde 不泄漏、update_account 同时写 active | 挑战：重复启动时 ALTER 是否重跑（PRAGMA 守卫跳过，second_init 测试通过）；旧 token 缺失 active 是否误判停用（`skip_serializing + default true`，session_payload 单测断言 active 不进序列化输出）；停用账号熵化路径与 046 dummy 语义是否冲突（同一「账号不存在」分支，等成本，不新增侧信道）；`deactivate_not_in` 事务边界（启动期串行执行、错误向上传播中止启动，无半可用进程） | 边界无缺口；serde 默认值路径由会话解码兼容设计保证（旧 claims 无字段 → default true） | pass |
| regression-and-side-effects | 全量 unit 57/57、login 集成 4/4、canonical `filehub integration` 串行全量（server api 20 + cli api/cmd/e2e）全绿、工作树 diff 范围（仅 account 模块 3 文件 + 新迁移）、Cargo.lock 与 vendored sfo-account 未变 | 排查既有调用方：`get_account_by_name` 仅 login/tests/seed 使用，seed 已切换 managed 方法；CLI/web 不读取 active；`second_init_is_idempotent` 既有语义保持增强；并行全量 api_integration 在本共享工作树重跑两次失败集不同（502/EOF），与 040 F-3/043 记录的并行干扰一致，canonical 串行全绿 | 无回归；并行全量 api_integration 偶发 502/EOF 为在制任务已知干扰（F-1），非本次改动引入 | pass |

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests` 57/57；
  `cargo test -p filehub-server --test api_integration -- login` 4/4；
  `python3 harness/scripts/test-run.py filehub integration` 退出码 0（canonical
  串行入口，含 server/cli 全部集成与 e2e）；`lower-tier-check.py --profile
  pre-edit` 与 `--profile completion` 通过
- Result: pass
- Exception reason: 共享工作树仍含 025-047 等在制未收尾任务，全量并行
  `api_integration` 偶发 502/EOF 属既有环境干扰（040 F-3 / 043 已记录），本任务
  以 canonical 串行入口与定向用例作为验收证据。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本工作树两次并行全量 `api_integration` 失败集不同（首次 6 项、次次 4 项，均为 502/EOF） | 在制任务共享/资源干扰导致并行偶发失败；canonical `--test-threads=1` 全量 20/20 通过，非本次代码缺陷 | no |
| F-2 | low | `upsert_seed_user` 与 `deactivate_not_in` 组合 | 配置里重复用户名不再触发启动报错，而是幂等覆盖/无操作；本产品配置级账号数量很小且语义更稳，属接受的边界细化 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 批准提案的 3 个 change_id 全部落地并经定向验证：配置密码/password_hash
  变更在重启后生效、非法 hash 不再被已存在账号跳过、配置删除的账号停用登录、
  重加配置恢复，迁移幂等且不动既有 schema 与 vendored crate；独立缺陷复核未
  发现阻塞项，F-1 为在制工作树已知并行干扰，F-2 为已记录的行为细化。
