# 配置账号密码变更生效与删除停用（048-config-account-sync）

- Status: complete
- Owner module: filehub（filehub-server account 子模块）
- Task manifest: `docs/versions/v0.1/modules/filehub/048-config-account-sync/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/048-config-account-sync/proposal.md`
- Affected paths: `server/migrations/0008_accounts_active.sql`（新增）、
  `server/src/account/store.rs`、`server/src/account/mod.rs`、
  `server/tests/unit/account.rs`、任务包 `task.yaml`/`proposal.md`/
  `completion-report.md`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 迁移：`AccountModule::init` 在 0002 之后以 `PRAGMA table_info(users)` 探测
  `active` 列，缺失时执行新增 `0008_accounts_active.sql`
  （`active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1))`）。SQLite 不支持
  `ADD COLUMN IF NOT EXISTS`，因此幂等性由 Rust 守卫保证；新库与已有库同路径，
  不改写既有 0001-0007 迁移，无数据转换。
- 值对象与 store：
  - `FilehubAccount` 新增 `active: bool`，serde 跳过序列化且缺失时默认 `true`
    （升级前已签发 JWT 的旧 claims 无此字段，仍可正常解码）；
  - `row_to_account`/`new_uncommitted`/`update_account` 携带并写回 `active`；
  - `get_account_by_name` 增加 `AND active = 1`：停用账号在登录时走「账号
    不存在」分支的 cost=12 dummy bcrypt 等成本校验，行为与 046 统一失败语义
    一致，不引入枚举计时侧信道；`Account::verify_password` 额外对
    `!active` 直接失败（纵深防御）；
  - 新增 seed 专用 `get_managed_account_by_name`（不过滤 active）与
    `deactivate_not_in(配置名单)`（把不在名单中的账号置为 `active=0`，保留行与
    全部外键引用，不级联删除）。
- seed 同步：创建与更新共用 `resolve_password_hash`（>72 字节拒绝、bcrypt
  `HashParts` 完整解析、cost 4..=31 校验）；同名已存在账号走
  `upsert_seed_user`：`password` 配置先 `bcrypt::verify` 旧 hash，密码未变且
  账号活跃则不写库（二次启动 hash 不变，无写库 churn）；`password_hash` 配置
  即使与库中 hash 相同也先做合法性与 cost 校验（修复「已存在账号跳过校验」），
  不一致才落库；两种路径都恢复 `active=1`，停用后重新加入配置即整体恢复。
- 测试：`server/tests/unit/account.rs` 新增改密生效/旧密拒绝、hash 变更生效与
  非法值拒绝（已存在账号）、删除停用+重加恢复（行保留）、hash 幂等等用例；
  `second_init_is_idempotent` 增强为断言 hash 不变；`active` 不进 JWT claims。

## Risk Screen

- Public contract, protocol, or CLI change: no（无 API/CLI 改动，v1-contract 不变）
- Persistent data, schema, or migration change: yes（`users` 新增 `active` 列；
  0008 为带默认值的 additive 迁移，启动时幂等 ALTER，旧行默认活跃，无数据迁移）
- Security, privacy, or trust-boundary change: yes（配置密码变更立即生效；配置
  删除的账号停止登录；属已确认 standard 交付的安全修复）
- Concurrency, lifecycle, or runtime integration change: no（seed 在启动阶段
  串行执行，早于服务监听）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no（无新依赖，Cargo.lock 未变；不修改 vendored `sfo-account`）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests` 57/57 通过（含
  account 模块 15 项）；`cargo test -p filehub-server --test api_integration -- login`
  4/4 通过；`python3 harness/scripts/test-run.py filehub integration`（canonical
  串行入口）退出码 0，run artifact
  `.harness/test-results/test-runs/20260826T044302Z-filehub-integration.json`
- Result: pass
- Residual risk or follow-up:
  - 停用只切断登录与账号查询；已签发 session/refresh token 在其 JWT 有效期
    到期前仍可被解码端读取、不立即吊销（用户确认的会话边界非目标；如需立即
    吊销，需在认证桥增加 active 校验或改造 vendored refresh 语义，另行评估）；
  - 并行运行全量 `api_integration` 在本共享工作树偶发 502/EOF（040/043 已记录
    的在制任务干扰项），canonical 串行运行全绿，与本次改动无关。
