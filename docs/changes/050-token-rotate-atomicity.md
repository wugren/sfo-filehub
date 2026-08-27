# Token 轮换状态转换原子化：不再返回不可用 JWT

- Status: complete
- Owner module: filehub（filehub-server tokens 子模块）
- Task manifest: `docs/versions/v0.1/modules/filehub/050-token-rotate-atomicity/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/050-token-rotate-atomicity/proposal.md`
- Affected paths: `server/src/tokens/service.rs`、`server/src/tokens/model.rs`、
  `server/src/tokens/http.rs`、`docs/api/v1-contract.md`、
  `server/tests/unit/tokens.rs`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 缺陷定位：`rotate` 先读记录（`load_token_row`）、生成密钥并签名，再单独执行
  不带条件的 `UPDATE tokens SET public_key_pem = ...`。读取与写入之间没有事务/
  条件边界，导致：已撤销 token 仍能 rotate（`resolve` 随后因 `revoked_at`
  拒绝其 JWT）；两个并发 rotate 都能成功、后写覆盖公钥使先前响应 JWT 立即
  失效；rotate 与 revoke 并发时可能返回不可用 JWT。
- 修复方案（与设计文档 `tokens.md` 的「rotate/revoke 同一事务」约束一致并
  补强并发 rotate）：
  - `rotate` 保留「读取→生成→签名」顺序，但将读取到的当前
    `public_key_pem` 作为 CAS 快照（公钥只被 rotate 替换，天然是版本号）；
  - 写入改为 `BEGIN IMMEDIATE` 单写者事务内的条件 UPDATE：
    `WHERE id = ? AND owner_id = ? AND revoked_at IS NULL AND public_key_pem = ?`
    （CAS）。同一状态出发的并发 rotate 只有一个能提交——后提交者公钥快照
    失配得到 0 行；
  - UPDATE 0 行时在事务内复查 `revoked_at` 区分错误：不存在/已撤销 →
    `NotFound`（404），token 仍存在且未撤销（并发 rotate 先提交）→ 新增
    `Conflict`（409，语义化重试）；
  - `TokenErrorKind` 增加 `Conflict` 并在 HTTP 层映射 `ApiError::conflict`，
    `docs/api/v1-contract.md` 的通用错误与 rotate 行同步记录 404/409 语义；
  - 回归测试覆盖：已撤销 rotate 拒绝、并发 rotate 单一获胜者（5 轮
    `tokio::join!`）、CAS 拒绝过期公钥快照、顺序轮换不回归。

## Risk Screen

- Public contract, protocol, or CLI change: yes——rotate 端点在此前未定义的
  竞态分支上新增 409（并发轮换冲突）与 404（不存在或已撤销）错误语义，已在
  `docs/api/v1-contract.md` 记录；成功路径响应形状不变，合法串行轮换行为不变。
- Persistent data, schema, or migration change: no——`tokens` 表与
  `token_scopes` 无 schema/迁移变化，仅写入条件与事务边界收紧。
- Security, privacy, or trust-boundary change: no——修复关闭「返回成功但 JWT
  立即可用」的凭据生命周期竞态；不新增权限、不改变校验顺序、不接触隐私数据。
- Concurrency, lifecycle, or runtime integration change: yes——本次修复正是
  rotate/revoke/rotate 并发边界：`BEGIN IMMEDIATE` 单写者锁串行化写入，
  CAS 使并发 rotate 至多一个成功；争锁失败走既有 Db 错误路径（可重试），
  不产生假成功。
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests tokens::`
  （8 项含 3 个新增回归全部通过）；`env -u HTTP_PROXY -u HTTPS_PROXY -u
  http_proxy -u https_proxy -u no_proxy -u ALL_PROXY -u all_proxy cargo test
  -p filehub-server` 全量 84 项通过（62 unit + 2 dv + 20 api_integration）；
  `cargo check -p filehub-server --tests` 通过；`cargo clippy -p filehub-server
  --tests` 无新增告警（4 个 tokens 相关告警均为在制改动既有项）
- Result: pass
- Residual risk or follow-up: 并发 rotate 落败方从「200 + 立即不可用 JWT」变为
  显式 409，调用方需按冲突重试；admin-web/CLI 未增加 409 客户端重试（提案非
  目标）；沙箱并行跑 api_integration 时登录用例偶发 502（本地 127.0.0.1:1091
  代理环境干扰，单跑与去代理全量均通过），与本任务改动无关，见完成报告 F 记录。
