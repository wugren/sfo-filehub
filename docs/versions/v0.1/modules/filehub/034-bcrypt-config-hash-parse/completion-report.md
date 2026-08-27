# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/034-bcrypt-config-hash-parse.md

## Delivery Summary

- Outcome:
  - `server/src/account/mod.rs::seed_user` 的 `user.password_hash` 配置校验从
    `starts_with("$2")` 升级为 bcrypt crate 完整解析（`HashParts::from_str`：60
    ASCII 字节、`$` 分隔、2a/2b/2y/2x 版本、数字 cost、BCRYPT base64），并补充
    cost 4..=31 范围检查；`$2-invalid`、截断 hash、越界 cost 均在 init 阶段返回
    明确错误且不落库；
  - 登录/会话运行时语义未动：`store.rs::verify_password` 与 sfo-account 调用链
    保持 fail-closed；合法 bcrypt 配置 hash 启动并可登录；
  - account 单测 9/9 全绿，其中 3 条为本任务新增回归（非法 `$2` 前缀、截断 hash、
    越界 cost）。
- Handoff: `cargo test -p filehub-server --test unit_tests unit::account` 9/9
  通过；`cargo test -p filehub-server` 全量通过（exit 0，unit_tests 38/38）；
  变更限定为 `server/src/account/mod.rs`、`server/tests/unit/account.rs` 与两个
  任务文档，未触碰 tokens/permissions/uploads 等其它在制未提交内容。

## Proposal Consistency

| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-bcrypt-config-hash-parse | `password_hash` 用 bcrypt 完整解析并校验 cost 4..=31，非法编码 init 报错且不落库；合法 hash 登录不变；password 分支与登录验证不动 | proposal.md P-001 + In scope + Success Criteria | `mod.rs::seed_user` 改用 `HashParts` 解析与 cost 检查；`account.rs` 新增 3 条非法编码回归 + 既有合法 hash 登录用例 | 匹配 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `mod.rs::seed_user` 三分支、bcrypt 0.19.3 `split_hash`/`verify` 解析规则、`store.rs::verify_password` 与 sfo-account 0.2.0 login 调用链 | 逐分支推演 `$2-invalid`、截断 40 字节、`$2b$99$`+真实 salt/hash、合法 `$2b$12$`、password 路径 >72 字节、password/password_hash 均缺失 | 非法编码全部在 init 阶段以含“bcrypt/cost”的错误拒绝；合法 hash 启动、登录成功；无路径绕过新校验 | pass |
| boundaries-and-failure-paths | cost 4/99/00、`$2`/`$2a$`/`$2x$`/非 ASCII、错误消息兼容既有 `rejects_non_bcrypt_config_hash` 断言、已存在账号的幂等 short-circuit | 检查 HashParts 解析通过但 `bcrypt::verify` 会拒绝的越界 cost 是否仍遗留静默失败；检查空串/前缀/长度边界在解析层全部拦截；检查已存在同名账号时后续启动是否重新校验新配置 | 越界 cost 由新检查在 init 拒绝；解析层拦截长度/版本/base64 不合法；账号已存在时幂等分支不重新校验配置（既有设计，记为 F-1，非本次引入） | pass |
| regression-and-side-effects | git 本地改动范围、`cargo test -p filehub-server` 全套、rustfmt 仅对变更文件、既有 account/session/refresh 链路 | 核对新校验是否影响合法 seed/二次 init/登录/会话/项目与权限模块；确认未连带格式化或改动其它在制文件 | 38 unit + 其它测试二进制全绿；二次 init 与登录回归通过；diff 仅含预定源文件与任务文档 | pass |

## Verification

- Targeted check: 新增回归先 red（3 条失败）后 green；`cargo test -p filehub-server
  --test unit_tests unit::account` 9/9 通过；`cargo test -p filehub-server` 全量
  通过（exit 0，unit_tests 38/38）；变更文件 rustfmt check 通过
- Result: pass
- Exception reason: not-applicable

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `seed_user` 先 `get_account_by_name` 命中即返回（幂等 seed），本次未改动该前置分支 | 同名账号已存在时，后续启动不会重新校验配置中变更后的 `password_hash`，非法新配置被幂等分支静默忽略（对合法变更同样生效）；属既有设计、非本次引入 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 提案 P-001 交付点全部落地：非法编码在启动期明确拒绝、合法配置行为不变、
  login/会话链路无回归；独立缺陷发现覆盖行为逻辑、边界失败路径与回归副作用，
  唯一记录项 F-1 为既有无阻塞边界，不阻止收尾。
