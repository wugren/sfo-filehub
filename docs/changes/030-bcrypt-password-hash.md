# 密码散列改为 bcrypt（不兼容存量数据）

- Status: complete
- Owner module: filehub（filehub-server account 子模块）
- Task manifest: `docs/versions/v0.1/modules/filehub/030-bcrypt-password-hash/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/030-bcrypt-password-hash/proposal.md`
- Affected paths: `server/src/account/store.rs`、`server/src/account/mod.rs`、
  `server/migrations/0002_accounts.sql`、`server/Cargo.toml`、`Cargo.lock`、
  `server/tests/unit/account.rs`
- Explicit tier override: none（用户 2026-08-24 确认提案并接受 standard）
- Expanded high-risk packet: none

## Approach

- `server/Cargo.toml` 新增 `bcrypt = "0.19"`（锁到 0.19.3），`Cargo.lock`
  同步；`getrandom` 保留（tokens 子模块仍在使用）。
- `server/src/account/store.rs`：
  - 删除 `hex_encode`/`sha256_hex`/`random_hex`/`password_hash_hex` 与
    `sha2` 导入，新增 `bcrypt_hash()`（cost = `bcrypt::DEFAULT_COST`=12）；
  - `FilehubAccount` 删除 `salt` 字段，`verify_password` 改用
    `bcrypt::verify`（错误/空 hash 一律返回 false）；
  - SELECT/INSERT/UPDATE 移除 `salt` 列。
- `server/src/account/mod.rs::seed_user`：
  - `user.password` 路径：>72 字节直接返回配置错误（明示 bcrypt 上限），否则
    bcrypt 散列落库；
  - `user.password_hash` 路径：必须是 `$2` 开头的 bcrypt 编码串，否则启动报错；
  - 缺失二者仍报 `must set password or password_hash`。
- `server/migrations/0002_accounts.sql` 删除 `users.salt` 列（新建库生效；
  存量库不写迁移脚本，用户明确不需要兼容，复用旧库需删除重建）。
- `server/tests/unit/account.rs`：seed 断言改为 bcrypt 格式；新增非 bcrypt
  配置 hash 拒绝、超长密码拒绝、bcrypt 配置 hash 可登录三个边界用例。

## Risk Screen

- Public contract, protocol, or CLI change: no（登录/会话 API 形状与错误语义不变；
  仅账号初始化校验新增两条配置错误信息）
- Persistent data, schema, or migration change: yes（删除 `users.salt` 列；
  新建库即生效，存量库需重建——用户已确认不兼容）
- Security, privacy, or trust-boundary change: yes（密码散列由单轮 SHA-256
  升级为 bcrypt cost=12，离线枚举成本提升约 5~6 个数量级；`password_hash`
  配置只接受 bcrypt 编码，杜绝误存裸哈希）
- Concurrency, lifecycle, or runtime integration change: no（散列发生在启动
  seed 与登录校验，无并发资源型改动；每次 log-in 增加约百毫秒级 CPU 开销）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: yes（新增 crates.io `bcrypt` 依赖随 Cargo.lock 锁定；
  旧 SHA-256 账号散列不再可验证，回滚需重建账号）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo check -p filehub-server` 通过；
  `cargo test -p filehub-server` 全绿（27 unit + 2 api_integration + 2 dv，
  account 用例 6/6，其中 3 条为本任务新增边界）；修改文件仅
  `rustfmt` 按 workspace edition=2024 格式化；
- Result: pass
- Residual risk or follow-up: 存量库旧账号不可登录（用户明确接受；重建库后按
  配置重新 seed 即可）；bcrypt cost=12 使登录路径成为可被刷的 CPU 放大点，
  登录限流/失败锁定记为后续任务候选；`password_hash` 外部提供者需遵守 bcrypt
  编码约定，起步校验已保证。
