# 配置 password_hash 升级为完整 bcrypt 解析校验

- Status: complete
- Owner module: filehub（filehub-server account 子模块）
- Task manifest: docs/versions/v0.1/modules/filehub/034-bcrypt-config-hash-parse/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/034-bcrypt-config-hash-parse/proposal.md
- Affected paths: `server/src/account/mod.rs`、`server/tests/unit/account.rs`
- Explicit tier override: none（用户 2026-08-25 确认提案并接受 standard）
- Expanded high-risk packet: none

## Approach

- `server/src/account/mod.rs::seed_user` 的 `user.password_hash` 分支：删除
  `starts_with("$2")` 前缀检查，改为 `hash.parse::<bcrypt::HashParts>()` 完整解析
  （复用 bcrypt 0.19.3 官方解析器：60 ASCII 字节、`$` 分隔位、2a/2b/2y/2x 版本、
  数字 cost、BCRYPT base64 的 salt/hash），并检查 cost 落在 4..=31；任一维度非法时
  init 阶段返回包含用户名与底层解析原因的错误，账号不落库、服务不启动；
- 采用结构解析而非 `bcrypt::verify` 验算：合法但高 cost（如 31）的 hash 不会在
  启动阶段触发 2^31 次散列计算，避免把启动校验变成自伤式阻塞；
- `server/tests/unit/account.rs` 新增三条回归：`$2-invalid`、40 字节截断 hash、
  `$2b$99$`（真实 salt+hash 替换 cost）均在 init 阶段被拒绝；既有“合法 bcrypt
  配置 hash 启动 + 登录成功”用例保持通过。

## Risk Screen

- Public contract, protocol, or CLI change: no（登录/会话 API 形状与错误语义不变；
  仅启动期配置校验更严格并给出明确错误）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: yes（收紧 `password_hash` 配置接受
  条件：`$2` 前缀但编码非法的 hash 不再带病启动，init 阶段直接拒绝；不改变登录侧
  `verify_password` 的 fail-closed 语义，不新增信任边界）
- Concurrency, lifecycle, or runtime integration change: no（校验仅发生在启动
  seed，无并发/后台任务/运行时集成改动）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no（复用既有 bcrypt 0.19 依赖，不新增/不升级）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: 先新增回归并验证 red（3 条新用例失败、2 条既有拒绝用例通过），
  落实现后 `cargo test -p filehub-server --test unit_tests unit::account` 9/9 通过；
  `cargo test -p filehub-server` 全量通过（exit 0，unit_tests 38/38）；变更的两个
  文件 `rustfmt --edition 2024 --config skip_children=true --check` 通过，未触碰
  worktree 中其它在制未提交文件
- Result: pass
- Residual risk or follow-up: 已存在同名账号时 `seed_user` 的幂等分支会直接返回，
  后续启动不会重新校验配置中变更后的 `password_hash`（对合法/非法变更均生效）——
  属既有幂等设计、非本次引入，已记录为完成报告 F-1；登录侧非法编码一律失败
  （fail-closed），启动校验未覆盖的库内篡改仍不会放行登录。
