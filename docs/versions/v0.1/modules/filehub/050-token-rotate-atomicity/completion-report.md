# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/050-token-rotate-atomicity.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `rotate` 改为先读取当前 `public_key_pem` 作为 CAS 快照，再在
    `BEGIN IMMEDIATE` 单写者事务内执行
    `UPDATE tokens SET public_key_pem=?, updated_at=? WHERE id=? AND
    owner_id=? AND revoked_at IS NULL AND public_key_pem=?`：已撤销 token
    不再能 rotate；同一状态出发的并发 rotate 至多一个成功，败者按事务内
    复查结果返回 404（不存在/已撤销）或 409（并发轮换冲突）；
  - `TokenErrorKind` 增加 `Conflict`，HTTP 层映射 `ApiError::conflict`；
    `docs/api/v1-contract.md` 的通用错误与 rotate 行补充 404/409 语义；
  - 回归用例：`token_rotate_rejects_revoked_token`、
    `token_concurrent_rotates_have_single_usable_winner`（5 轮并发）、
    `token_rotate_cas_rejects_stale_public_key_snapshot` 全部通过，
    既有顺序轮换/撤销/属性修改用例不回归。
- Handoff: `cargo test -p filehub-server` 全量 84 项通过（62 unit + 2 dv +
  20 api_integration，去本地代理环境变量后复跑）；`cargo check -p
  filehub-server --tests` 通过；`cargo clippy -p filehub-server --tests`
  无新增告警。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-token-rotate-tx-cas | rotate 在 `BEGIN IMMEDIATE` 事务内以 `revoked_at IS NULL + 旧公钥快照` 做 CAS 更新；0 行时 404（撤销/不存在）/409（并发轮换）；`TokenErrorKind::Conflict` 映射 HTTP 409 | proposal.md P-001 | service.rs 单事务 CAS UPDATE、model.rs Conflict 变体、http.rs 409 映射；已撤销 rotate→NotFound、并发 rotate 单一获胜者、CAS 过期快照拒绝三个回归用例通过 | 匹配 | pass |
| fh-token-rotate-contract | `docs/api/v1-contract.md` 记录 rotate 的 404/409 语义 | proposal.md P-002 | 通用错误段与 rotate 路由行已补充 404/409 说明，与实现映射一致 | 匹配 | pass |
| fh-token-rotate-tests | 单元回归：已撤销拒绝、并发单一获胜者、CAS 过期快照拒绝、顺序轮换不回归 | proposal.md P-003 | 3 个新增用例 + 既有 token 生命周期用例全部通过；全量 84 项无回归 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | 新 rotate 全流程（快照读取、密钥生成、签名、BEGIN IMMEDIATE、CAS UPDATE、0 行复查、commit/rollback）与 TokenErrorKind/HTTP 映射；对照设计文档 tokens.md 的轮换/撤销约束 | 反向推演四种交错：revoke 先提交、并发 rotate 先提交、顺序轮换、轮换后又 revoke；逐一验证结果分支（404/409/成功）与响应 JWT 可用性 | 无绕过路径：revoke 先提交→404 且不写库；并发 rotate 先提交→409 且不覆盖获胜公钥；顺序轮换读到新快照照常成功；唯一成功方 JWT 立即可 resolve | pass |
| boundaries-and-failure-paths | 0 行复查 SELECT 的 None/Some(revoked)/Some(未撤销) 三态、事务早退依赖 sqlx Drop 回滚、`try_get` 错误传播、409 只映射冲突不掩盖撤销 | 构造三态各自直接用例；检查 UPDATE 0 行后若直接返回 404 会误报已撤销（已改为事务内复查）；确认 `load_token_row` 快照在事务外读取正是并发 detectability 的来源而非缺口；连接池单连接下 join! 双 rotate 5 轮恰好一个胜者 | 三态区分正确：已撤销 token 从不返回 409；并发轮换从不返回 404；失败路径事务 rollback、不残留半更新；无 off-by-one/CAS 误伤 | pass |
| regression-and-side-effects | 既有 token 生命周期/权限/expiry/owner 隔离用例、api_integration 的 rotate 段落（旧 JWT 失效、新 JWT 可用）、契约文档差异、cli/clippy 输出 | 排查 update 是否受 CAS 影响（不碰公钥，无影响）；确认新增 Conflict 未触碰其它错误语义映射（NotFound/InvalidInput/Db 分支不变）；检查并发用例旧码必红性（旧码两 UPDATE 无条件均成功） | 既有 rotate/revoke/update/列表行为无回归；48 项非 token 用例与 20 项集成用例全部通过；clippy 无新增告警；修复仅触及 tokens 子模块与契约行 | pass |

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests tokens::`
  8 项通过（含 3 个新增）；全量 `cargo test -p filehub-server` 84 项通过
  （62 unit + 2 dv + 20 api_integration）；`cargo check -p filehub-server
  --tests`、`cargo clippy -p filehub-server --tests` 通过且无新增告警
- Result: pass
- Exception reason: 无

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 沙箱环境设 `HTTP(S)_PROXY=127.0.0.1:1091` 且 `no_proxy` 条目为 `127.*`；默认并行全量跑 api_integration 时 3 个登录相关用例偶发 502，单跑分别通过，去代理环境变量后全量 84 项通过 | 本地代理对随机端口请求的偶发干扰，属环境级并行抖动；登录路径未在本任务改动范围 | no |
| F-2 | low | 并发 rotate 落败方现在返回 409；admin-web/CLI 未新增 409 重试逻辑（提案非目标） | 客户端遇到冲突需自行按重试语义处理，成功路径不受影响 | no |
| F-3 | low | 并发用例 5 轮 `tokio::join!` 断言单胜者 | 极端调度下若两个预读发生在不同提交区间，可能产生合法「顺序式」双成功；确定性由已撤销用例与 CAS 过期快照 SQL 用例兜底 | no |
| F-4 | low | 基线快照（15:42）与完成校验（15:46）对比显示 `server/src/http/mod.rs`、`server/src/projects/mod.rs`、`server/src/projects/service.rs`、`server/tests/api_integration.rs`、`server/tests/dv_tests.rs`、`server/tests/unit/projects.rs` 六文件在基线捕获后被另一并发会话修改（050-project-delete-files-cleanup 任务在制内容，含 ProjectModule::init 注入 FileStore、项目删除即时回收文件等），本任务全程未触碰这些文件 | 共享工作树并发会话产生基线噪声，变更清单因此包含它们；并发会话与本任务均使用序列 050（`050-project-delete-files-cleanup` vs `050-token-rotate-atomicity`），需要用户确认哪个任务改用后续序列；不阻塞本任务交付 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001/P-002/P-003 全部落地：rotate 写入已事务化并带 `revoked_at
  IS NULL` + 旧公钥快照 CAS 条件，0 行复查精确区分 404/409，新增 Conflict
  映射与契约说明一致；已撤销 rotate 拒绝、并发 rotate 单一获胜者、CAS 拒绝
  过期快照等回归用例通过，全量 84 项测试与 clippy 无回归；独立缺陷发现覆盖
  行为/边界/回归三向反例搜索，F-1~F-4 均为非阻塞低危记录（F-1 与沙箱代理
  相关，F-4 为共享工作树并发会话基线噪声，均与本任务改动无关）。
