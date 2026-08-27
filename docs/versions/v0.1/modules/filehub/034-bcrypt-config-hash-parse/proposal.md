task_manifest: task.yaml
status: approved
---

# 配置 password_hash 升级为完整 bcrypt 解析校验

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户回复「确认」，确认采纳提案并接受建议的 standard 层级。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 这是对 `server/src/account/mod.rs` 中 `seed_user` 认证配置校验路径的生产代码
    bugfix，且改变的是服务启动期对 `password_hash` 配置的接受/拒绝行为，处于安全
    相关边界内，不完全满足 trivial 对“无 security/privacy 实质影响”的界定；按
    standard 默认 bounded bugfix 流程执行（pre-edit 基线 -> 实现 -> 验证 -> 变更
    记录 -> completion-report）。
  - 未发现 high-risk 触发边界：不改公开 API/协议/CLI 契约，不涉及 schema/迁移与持久
    数据，不新增依赖，不改变登录校验（`FilehubAccount::verify_password`）与
    sfo-account 的运行时语义，不改构建产物/发布流程。
- Proposal and tier confirmation: 2026-08-25 用户回复「确认」，以 standard 终值批准，
  随后按 lower-tier 流程执行（pre-edit 基线 -> 实现 -> 验证 -> 变更记录 ->
  completion-report）。

## Background and Goal

- 现象（低危，用户报告）：`server/src/account/mod.rs:94-101` 的 `seed_user` 对配置
  `user.password_hash` 只检查 `hash.starts_with("$2")`。形如 `$2-invalid` 的非法编码
  会通过启动校验并被落库；此后每次登录时 `server/src/account/store.rs:47-52` 的
  `verify_password` 调用 `bcrypt::verify`，解析失败返回 `Err(InvalidHash)`，再被
  `unwrap_or(false)` 吞成“密码不匹配”，该账号所有登录永久静默失败，且与密码错误
  无法区分、无日志线索。
- 目标：启动阶段仅接受完整合法的 bcrypt 编码串（结构、版本、长度、cost、base64 全部
  合法）；任一维度非法时以明确错误拒绝启动并指出用户与原因，不再把不可登录的账号
  写入 users 表。

## Scope

### In scope

- `server/src/account/mod.rs` 的 `seed_user` `password_hash` 分支：改为调用 bcrypt
  crate 自身的解析（`HashParts::from_str`）做完整结构校验，并校验 cost 在
  4..=31；失败时返回包含用户名与解析原因的启动错误；
- 保留 `password` 分支与 >72 字节校验不变；
- `server/tests/unit/account.rs` 增加/调整回归测试：`$2-invalid`、被截断的合法 hash、
  越界 cost 等非法编码在 init 阶段报错；合法 bcrypt hash 仍可启动且登录成功；
- 按 standard 流程新增 `docs/changes/034-bcrypt-config-hash-parse.md` 与任务包
  `completion-report.md`。

### Out of scope

- 不修改 `store.rs::verify_password` 的 `unwrap_or(false)` 语义与登录错误码
  （登录侧仍 fail-closed，本次只保证合法编码才进入运行时）；
- 不修改 sfo-account、schema/迁移、http/API 契约、admin-web、CLI 或示例配置文档；
- 不新增依赖（复用已有 `bcrypt = "0.19"`）。

### Boundary with neighboring modules

- 校验只消费配置并决定是否写入 users 表；后续登录验证路径不变，合法 hash 的行为与
  现状完全一致。

## Requirement Review

- 需求合理：与用户报告及 030 验收记录中的 F-3 一致；把“前缀检查”升级为“完整解析”，
  使启动校验与运行时 `bcrypt::verify` 对编码合法性的判断一致。
- 方向权衡：
  - 采用剖析构而不执行 bcrypt 计算：`HashParts::from_str` 只校验格式，不会对
    `$2b$31$...` 这类合法但高 cost 的 hash 执行 2^31 次计算，避免启动阻塞
    （若改用 `bcrypt::verify` 校验会触发实际散列计算）；
  - 结构解析之外补充 cost 4..=31 检查：`split_hash` 只解析数字不校验范围，
    越界 cost 会在登录时被 `verify` 拒绝并再次静默失败，故纳入启动校验；
  - 错误消息沿用“must be a bcrypt encoded string”的既有措辞并带上底层解析错误，
    既有 `rejects_non_bcrypt_config_hash` 测试断言可保持成立。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-bcrypt-config-hash-parse | `user.password_hash` 配置在 `seed_user` 中用 bcrypt crate 完整解析（结构/版本/长度/base64）并校验 cost 4..=31，非法编码在 init 阶段报错且不落库 | 仅 `server/src/account/mod.rs` 与 `server/tests/unit/account.rs`；password 分支与登录验证语义不变 | 用 `HashParts::from_str` 只解析不计算，避免高 cost 启动阻塞；登录侧保持 fail-closed | 非法编码用例（`$2-invalid`、截断 hash、越界 cost）init 报错；合法 hash 启动 + 登录成功；`cargo test -p filehub-server` 通过 | 不改登录验证/错误码、schema、依赖、契约文档 |

## Success Criteria

- Concrete system-visible result: 配置含 `$2-invalid`（或其它以 `$2` 开头但编码非法的
  hash）时服务启动直接失败并给出明确的 bcrypt 编码错误；配置合法 bcrypt hash 时行为
  与现状一致，登录成功。
- Required evidence:
  - `server/tests/unit/account.rs` 新增非法编码用例先 red（现状通过启动校验）后 green；
  - 合法 bcrypt 配置 hash 用例保持 green（启动 + `login` 成功）；
  - `cargo test -p filehub-server` 全量通过；
  - `docs/changes/034-bcrypt-config-hash-parse.md` 与 `completion-report.md` 记录实现
    与独立缺陷发现结论。
- Explicit non-goals: 不改变登录验证的吞错/失败语义；不新增配置格式或新的 CLI 入口。

## Risks

- 低：行为变化仅影响“配置了非法 bcrypt hash”的启动场景——从“带病启动后账号不可登录”
  变为“启动期明确失败”，是配置契约内的严格化，无数据迁移；
- 低：新检查复用到 bcrypt crate 官方解析器，不手写格式判断；仅校验不计算，不会因
  高 cost hash 阻塞启动；
- 无 schema/迁移、公开契约、依赖变化；回归测试覆盖非法前缀、截断 hash、越界 cost 与
  合法 hash 四条路径。
