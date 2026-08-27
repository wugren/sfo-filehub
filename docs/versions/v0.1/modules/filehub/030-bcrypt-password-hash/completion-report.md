# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/030-bcrypt-password-hash.md

## Delivery Summary
- Outcome:
  - filehub-server 账号密码改用 bcrypt（bcrypt 0.19.3，cost=12）：`store.rs`
    删除单轮 SHA-256 与独立 salt，`verify_password` 使用 `bcrypt::verify`；
    配置种子账号时密码 >72 字节启动报错、`password_hash` 必须是 `$2` 开头
    的 bcrypt 编码串；
  - `FilehubAccount` 删除 `salt` 字段，`users` 表删除 `salt` 列（新库生效，
    按用户要求不提供存量兼容/迁移）；
  - 账号测试 6/6：含存量格式断言更新与 3 条新增边界（非 bcrypt 配置 hash 拒绝、
    超长密码拒绝、bcrypt 配置 hash 登录成功）。
- Handoff: `cargo test -p filehub-server` 全绿（27 unit + 2 api_integration +
  2 dv）；`cargo check` 通过；未触碰 tokens/permissions/uploads 等其它在制
  未提交内容（git diff 限定为 6 个预定文件）。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-bcrypt-hash-and-verify | bcrypt 散列/校验；password 路径 >72 字节报错；password_hash 路径必须 bcrypt 编码 | proposal.md FHP-bcrypt-hash + In scope | `store.rs::bcrypt_hash`/`verify_password`、`mod.rs::seed_user` 三分支 | 匹配 | pass |
| fh-bcrypt-dependency | 新增 bcrypt 依赖并锁版本 | proposal.md FHP-bcrypt-dep + In scope | `server/Cargo.toml` + `Cargo.lock`（0.19.3） | 匹配 | pass |
| fh-bcrypt-salt-column-drop | 移除 salt 字段与 users.salt 列 | proposal.md FHP-bcrypt-schema + In scope | `store.rs`/`FilehubAccount`/`0002_accounts.sql` 同步删除；SELECT/INSERT/UPDATE 更新 | 匹配 | pass |
| fh-bcrypt-account-tests | 单测覆盖 bcrypt 格式、正误口令、超长密码、非法 hash 配置、登录回归 | proposal.md FHP-bcrypt-tests + Success Criteria | `unit/account.rs` 6 用例全绿 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `store.rs` 全函数、`mod.rs::seed_user` 三分支、sfo-account 0.2.0 的 `Account::verify_password` 调用点 | 逐分支推演：空 password_hash、错误口令、正确口令、bcrypt 配置 hash、password/password_hash 均缺失五种输入 | 全部按预期返回（false / 登录成功 / 启动报错），login/session/refresh 链路无旁路 | pass |
| boundaries-and-failure-paths | 72 字节边界、`$2` 前缀校验、空字符串、salt 列删除后的 row 解码/INSERT/UPDATE/序列化 | 检查 73/72 字节、非 bcrypt 前缀、`$2` 伪造值、旧库（有 salt 列）行为是否被额外兼容 | >72 字节与非 `$2` 前缀在启动期拒绝；新库无 salt 列路径全通过；旧库按用户要求明确作废（不提供兼容代码） | pass |
| regression-and-side-effects | git diff 全量、`cargo test -p filehub-server` 全套、sfo-account 登录/会话用例、其它未提交在制文件 | 核对 tokens/permissions/uploads/account 其它模块是否被连带改动；`getrandom`/`sha2` 依赖去向；CLI/前端是否受影响 | 27 unit + 2 dv + 2 integration 全绿；仅 6 个预定文件变更；`getrandom` 仍被 tokens 使用、`sha2` 仍在 storage 使用，无死依赖残留；sfo-account 与 API 契约未动 | pass |

## Verification
- Targeted check: `cargo test -p filehub-server --test unit_tests account` 6/6
  通过（含 3 条新增）；`cargo test -p filehub-server` 全量 31 项全通过；
  `cargo check -p filehub-server` 通过
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 提案明确「不兼容存量数据」；`0002_accounts.sql` 未提供 ALTER | 存量库（含 `salt` 列与 SHA-256 哈希）复用后插入会失败或旧账号不可登录；需删库重建重新 seed，按用户确认属于预期行为 | no |
| F-2 | low | `bcrypt::DEFAULT_COST`（12）每次登录约百毫秒级 CPU | 登录端点可被批量请求放大 CPU，属既有缺失（本项目无登录限流）；已在 change record 记为后续任务候选 | no |
| F-3 | low | `password_hash` 配置仅校验 `$2` 前缀 | 外部提供的 bcrypt 串若被篡改会直接登录失败，无静默降级；符合 fail-closed 目标 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 四项提案交付点全部落地并经全量测试验证；独立缺陷发现覆盖行为逻辑、
  边界失败路径与回归副作用，未发现阻塞性缺陷；F-1~F-3 均为用户已确认的预期
  行为或非阻塞后续项。
