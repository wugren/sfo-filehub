---
task_manifest: task.yaml
status: approved
---

# 密码散列改为 bcrypt（不兼容存量数据）

Risk profile: not-created（standard 层级不创建 risk-profile）

## Approval Record

- approver: user
- approval_date: 2026-08-24
- user_statement: 用户 2026-08-24 回复「确认」，确认采纳提案（不兼容存量数据、
  bcrypt 保存密码、移除 salt 字段/列），并接受建议的 standard 层级。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 改动集中在单一 crate（`filehub-server`）的 account 子模块与对应测试；
  不触及已处于在制状态的 tokens/permissions/projects/http 等未提交改动；
  - 命中安全边界（凭据存储）与持久化字段语义（删除 salt 字段/列），但仓库
    处于 greenfield 阶段且用户明确声明不需要兼容存量数据，不存在已上线用户、
    回滚协调或存量哈希回退问题；
  - 新增 bcrypt 依赖属于成熟、广泛使用的标准 crate，风险可控；
  - 有明确的定向验证信号（账号单测 + 登录回归）。
  - 若后续出现真实部署数据需要保留，将触发 high-risk 升级并增加回迁设计。
- Proposal and tier confirmation: 用户 2026-08-24 回复「确认」——确认提案并
  选择 standard 层级；standard 层级跳过设计/测试/验收分期文档与风险档案，
  剩余风险（存量账号作废、登录延迟、bcrypt 供应链）记录在 change record 与
  completion-report 中显式标注。

## Background and Goal

- 背景：评审中危 #6 指出 `server/src/account/store.rs` 使用单轮
  SHA-256（`SHA256(password + ":" + salt)`）保存密码，缺乏抗暴力破解成本；
  SQLite 泄露后可高速离线枚举（详见 `docs/changes/029-public-read-logged-in.md`
  之前的 server 评审记录与本提案的现场复现）。
- 当前用户要求：改为 bcrypt 保存密码，且不需要兼容存量数据。
- 目标：新创建/重置的账号一律使用 bcrypt（`$2b$...` 自含盐格式）散列并校验；
  移除不再使用的 SHA-256 逻辑与独立 salt 字段/列；在不接触其他未提交改动
  的前提下用测试证明登录/会话回归正常。

## Scope

### In scope

- fh-bcrypt-hash-and-verify：
  - `server/src/account/store.rs`：删除 `password_hash_hex`，改为
    bcrypt 编码与校验；`verify_password` 使用 bcrypt verify（忽略外传
    timestamp salt，与现状一致）；
  - `server/src/account/mod.rs::seed_user`：
    - 配置提供 `user.password` 时用 bcrypt 散列落库（cost=12，编码自含盐）；
    - 配置提供 `user.password_hash` 时按 bcrypt 编码串保存，并在初始化时
      校验必须是 `$2` 开头，否则启动报错；
    - 密码超过 bcrypt 72 字节上限时启动报错（避免静默截断）。
- fh-bcrypt-dependency：
  - `server/Cargo.toml` 新增 `bcrypt` crate，`Cargo.lock` 随之更新。
- fh-bcrypt-salt-column-drop：
  - `FilehubAccount` 删除 `salt` 字段，`server/migrations/0002_accounts.sql`
    删除 `users.salt` 列（新建库不再有该列）；`store.rs` 的 SELECT/INSERT/
    UPDATE 与 `tests/unit/account.rs` 测试同步调整。存量库不提供迁移脚本
    （用户明确不需要兼容），如需复用旧库请删除后重建。
- fh-bcrypt-account-tests：
  - 更新 `server/tests/unit/account.rs`：断言落库密码为 `$2b$`/`$2a$` 格式、
    正确/错误口令验证结果、超长密码与非法 password_hash 配置在 init 时报错、
    session 序列化仍不暴露凭据；
  - 登录/会话/幂等初始化回归由现有 account 单测与 API 集成测试覆盖。

### Out of scope

- 存量 SHA-256 哈希的任何兼容/迁移逻辑（用户明确不需要）；
- 修改 `sfo-account` crate、登录 API 契约、会话 JWT 或 token 体系；
- 登录限流、失败锁定等接口防护（记录为后续建议，不随本任务落地）；
- `docs/api/v1-contract.md` 与前端/CLI 行为（无公共接口变化）；
- 本仓库当前未提交的 tokens/permissions/uploads 等其它在制改动。

## Requirement Review

- 需求评估：合理。bcrypt 是非 memory-hard 但抗 GPU 通用的成熟慢哈希，
  相比单轮 SHA-256 将离线枚举成本提高约 5~6 个数量级，符合“中危 #6”的
  修复方向；用户选择 bcrypt（而非 Argon2id）符合“引入最少且成熟”的取舍。
- 关键取舍：
  - 选择 bcrypt 而不是 Argon2id：Argon2id 更抗专用硬件，但新增依赖与参数
    配置更多；bcrypt 编码自含 cost/salt，实现与运维最简单；
  - 删除 salt 字段/列：bcrypt 编码内嵌盐，独立性盐无使用方；greenfield +
    不兼容存量使删除零成本；
  - 72 字节上限：显式拒绝超长密码而不是静默截断或“先 SHA-256 再 bcrypt”
    （后者会引入新组合语义，不做）。
- 未解决问题：无。

## Proposal Items

| proposal_id | change_id | requirement | success_evidence |
|-------------|-----------|-------------|------------------|
| FHP-bcrypt-hash | fh-bcrypt-hash-and-verify | account 子模块改用 bcrypt 散列/校验；password 路径 >72 字节启动报错；password_hash 路径必须是 bcrypt 编码 | seed 用户密码落库为 `$2` 开头的 bcrypt 串且登录成功；超长密码与非 bcrypt 配置 hash 在 init 阶段报错 |
| FHP-bcrypt-dep | fh-bcrypt-dependency | 新增 bcrypt 依赖并锁版本 | `server/Cargo.toml` 声明 bcrypt，Cargo.lock 锁定对应版本且 `cargo check` 通过 |
| FHP-bcrypt-schema | fh-bcrypt-salt-column-drop | 移除 salt 字段与 users.salt 列 | 新建库 `users` 表无 `salt` 列；store 的 SELECT/INSERT/UPDATE 与序列化不再引用 salt |
| FHP-bcrypt-tests | fh-bcrypt-account-tests | 账号单测更新并覆盖新边界 | `cargo test -p filehub-server` 全绿，account 用例断言 bcrypt 格式、正误口令、超长密码、非法 hash 配置 |

## Success Criteria

- 新库创建后 `users` 表不含 `salt` 列；配置密码落库为 bcrypt 编码串；
- `verify_password` 对正确口令通过、错误口令失败；配置错误（超长密码、
  非 bcrypt 的 password_hash）在 init 阶段给出明确错误；
- `cargo test` 通过，其中 account 单测覆盖上述新边界，API 集成登录回归通过；
- 除本提案列出的文件外，不修改其它路径；不触碰当前工作区其它未提交改动。

## Risks

- 存量库中旧 SHA-256 用户将无法登录（用户明确接受“不需要兼容”）；若有真实
  部署数据需另行创建回迁任务；
- bcrypt 72 字节截断风险通过启动期显式拒绝消除；
- 新增 crates.io 依赖（供应链/构建图变化），随 workspace 锁文件审计；
- 登录开销从微秒级提升到约 100ms 量级（cost=12），登录接口防护（限流）未随
  本任务落地，记录为后续建议。
