---
task_manifest: task.yaml
status: approved
---

# Token 轮换状态转换原子化：不再返回不可用 JWT

Risk profile: not-created（标准层级不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 修改点是 filehub-server 的 tokens 子模块 bugfix：修复 rotate 与
    revoke/rotate 之间的非原子状态转换，涉及 token 凭据生命周期（安全/
    授权边界）与持久化写入的并发语义，不满足 trivial 对「无 security/
    concurrency 实质影响」的界定；
  - 未触发 high-risk：无 schema/迁移、无依赖/构建图、无发布/回滚/
    部署语义变更；成功路径 HTTP 契约与 JWT 签发机制不变；仅在一个此前
    未定义的竞态分支上新增 409 conflict 错误（并发 rotate 落败方不再
    返回「200 + 立即不可用 JWT」），与 027/039 等同仓库 token/并发修复
    的 standard 先例一致；
  - 剩余风险（不改变分层）：并发 rotate 的落败方从「假成功」变为显式
    409，调用方需按冲突重试；该项会在 change record 与验收中显式记录。
- Proposal and tier confirmation: 用户 2026-08-26 回复「确认」，确认采纳本
  提案（rotate 原子化 + 并发轮换 409 + 回归测试）并接受建议的 standard 层级。

## Approval Record

- approver: 用户
- approval_date: 2026-08-26
- user_statement: 用户 2026-08-26 回复「确认」，确认采纳本提案并接受 standard
  层级；按提案范围执行 rotate 状态转换原子化修复。

## Background and Goal

- 现象（评审第 4 项，中危）：`server/src/tokens/service.rs:328` 的
  `rotate` 先读记录、生成密钥并签名，再单独执行不带条件的 UPDATE：
  - 已撤销 token 仍可 rotate 并返回 200 与新 JWT，但
    `server/src/tokens/service.rs:390` 的 `resolve` 会立即因
    `revoked_at` 拒绝它；
  - 两个并发 rotate 都能返回成功，但最后一次 UPDATE 覆盖验签公钥，
    其中一个响应中的 JWT 立即失效；
  - rotate 与 revoke 并发时也可能返回已经不可用的 JWT。
- 与 `docs/versions/v0.1/modules/filehub/001-filehub-core-platform/design/
  tokens.md:96` 的约束不一致：该文档要求 rotate 与 revoke 在同一 SQLite
  事务内替换公钥/写 `revoked_at`，`resolve` 以最新记录为准。
- 目标：让 rotate 的状态转换原子化并满足设计约束——只对未撤销 token
  生效；同一状态下发起的并发 rotate 至多一个成功，且任何成功响应返回的
  JWT 在返回时都立即可用；revoke 先提交的 rotate 返回 404，不复写已撤销
  记录。

## Scope

### In scope

- `server/src/tokens/service.rs`：`rotate` 改为「读取当前
  `public_key_pem`（CAS 快照）→ 生成密钥并签名 → `BEGIN IMMEDIATE`
  事务内执行带 `revoked_at IS NULL` 与旧公钥 CAS 条件的 UPDATE → 单写者
  串行化」；UPDATE 影响 0 行时在事务内复查区分「不存在/已撤销 → 404」与
  「并发 rotate 先提交 → 409」；
- `server/src/tokens/model.rs`：`TokenErrorKind` 增加 `Conflict` 与
  `TokenError::conflict` 构造；
- `server/src/tokens/http.rs`：`Conflict` 映射为 `ApiError::conflict`
  （409）；
- `docs/api/v1-contract.md`：rotate 行与通用错误说明补充 404（不存在或
  已撤销）与 409（并发轮换冲突）语义；
- `server/tests/unit/tokens.rs`：新增已撤销 rotate 拒绝、并发 rotate
  单一获胜者、CAS 拒绝过期快照、顺序轮换不回归的回归用例。

### Out of scope

- 不做 schema/迁移（`tokens` 表无变化）；
- 不修改 `create`/`update`/`revoke`/`list`/`resolve` 既有语义（含
  `update` 对已撤销 token 的既有行为、`load_token_row` 中 `revoked_at`
  重复读取的无害冗余）；
- 不修改 JWT 载荷、签发算法、成功响应 JSON 形状与鉴权中间件；
- 不改 admin-web/CLI 行为，不为 409 增加客户端重试逻辑；
- 不触碰 025-049 等在制未提交任务改动；不运行仓库级格式化。

### Boundary with neighboring modules

- 轮换/撤销状态只由 tokens 子模块持有（`tokens.revoked_at` /
  `public_key_pem`），权限与 JWT 解析路径只读消费；本任务不改
  permissions/checker 与认证中间件；
- 409 语义与仓库既有错误分类一致（版本/项目名冲突已使用同一错误码），
  仅扩展 token rotate 的竞态分支。

## Requirement Review

- 需求合理：token 记录只保存一把当前验签公钥，且设计要求旧 JWT 在替换后
  立即失效；因此「同一状态下两个并发 rotate 都返回可用的 200 JWT」在
  当前存储模型下不可能成立，正确语义是让其中一个原子获胜、另一个显式
  拿到冲突错误；
- 方向选择：`BEGIN IMMEDIATE` 让 SQLite 单写者锁串行化 rotate/revoke
  写入；CAS 基于「进入写事务前的公钥快照」是为了覆盖 rotate vs rotate——
  若在事务内读快照，串行化的第二个 rotate 会读到新公钥并继续成功，仍会
  出现「两个 200 + 一个立即失效」；CAS 使仅从同一旧状态出发的并发调用
  只有一个能提交；
- 竞态复查：UPDATE 0 行后在同一事务内重新读 `revoked_at`，把「已撤销/
  不存在」映射 404、把「仅公钥被并发 rotate 替换」映射 409，错误语义
  精确；
- 顺序轮换不受影响：后一次 rotate 读到最新公钥快照，CAS 命中后正常成功，
  行为与既有测试一致（新 JWT 可用、旧 JWT 失效）。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-token-rotate-tx-cas | rotate 在 `BEGIN IMMEDIATE` 事务内以 `revoked_at IS NULL + 旧公钥快照` 做 CAS 更新；0 行时区分 404（撤销/不存在）与 409（并发轮换）；`TokenErrorKind::Conflict` 映射 HTTP 409 | 仅 `server/src/tokens/` 三个文件；成功路径语义不变 | 落败的并发 rotate 返回 409 而非 200；换取「返回的 JWT 立即可用」 | 已撤销 rotate→404；并发 rotate→恰一个 Ok 且其 JWT resolve、另一个 Conflict | 不改 schema、不改 resolve/revoke/create/update |
| P-002 | fh-token-rotate-contract | `docs/api/v1-contract.md` 记录 rotate 的 404/409 语义 | 仅契约文档表格与通用错误小节 | 409 是新出现的竞态分支错误码，如实记录 | 文档行与实现一致 | 不改成功响应形状 |
| P-003 | fh-token-rotate-tests | 单元回归：已撤销拒绝、并发单一获胜者、CAS 过期快照拒绝、顺序轮换不回归 | `server/tests/unit/tokens.rs` | 并发用例为终态不变量断言；确定性由 revoke 用例与 CAS SQL 用例提供 | 新增用例通过；`cargo test -p filehub-server` 既有用例不回归 | 不引入并发测试基建 |

## Success Criteria

- 系统可见结果：
  - 已撤销 token 调用 rotate 返回 404，不再返回 200 + 立即不可用 JWT；
  - 从同一状态并发发起两次 rotate 时，恰好一个 200（其 JWT 立即可
    resolve），另一个 409；不再出现两个 200 后覆盖公钥导致一个 JWT
    失效；
  - rotate 与 revoke 并发时，revoke 先提交则 rotate 404；rotate
    先提交则 revoke 正常生效，合法顺序轮换行为不变；
  - `resolve` 对已撤销/被后续轮换的 JWT 仍按既有规则失败。
- 必需证据：`cargo test -p filehub-server`（或统一 test-run 入口）
  通过，含新增回归断言；既有 token 生命周期/权限/API 用例不回归；验收
  反例搜索（已撤销轮换、并发双轮换、误用 409 掩盖撤销）。
- 显式非目标：不做 schema/迁移、不改 JWT 签发与成功响应形状、不给
  admin-web/CLI 加 409 重试、不改其他模块。

## Risks

- 并发 rotate 落败方行为从「200 + 立即不可用 JWT」变为显式 409 conflict：
  这是「一个 token 只保存一把验签公钥 + 旧 JWT 必须立即失效」设计约束下
  的唯一一致语义，调用方需按冲突重试；409 只出现在此前未定义的竞态分支，
  合法串行轮换不受影响（后一次读到新快照仍然成功）；
- `BEGIN IMMEDIATE` 会让 rotate 持有短暂 SQLite 写锁，与 revoke/其他 rotate
  竞争时可能等待或返回 busy 错误（按 Db 错误路径处理，客户端可重试），
  不会产生静默假成功；
- 本工作树存在大量在制未提交任务改动（025-049 等），本任务只修改提案列出
  的文件，不运行仓库级格式化；全量测试状态受在制内容影响时以定向验证 +
  全量记录为准。
