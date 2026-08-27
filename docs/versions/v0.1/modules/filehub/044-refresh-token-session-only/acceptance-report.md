# 044-refresh-token-session-only 验收报告

## Findings

| ID | Severity | Owning Stage | Correctness Category | Evidence | Problem | Blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-000 | none | none | overall | 独立反例搜索覆盖提案/设计/实现/测试与统一运行产物（decode_session 分支、三个消费方、red-green 复现、49 unit + 2 dv + 17 integration + compile closure 全绿） | 未发现本任务范围内缺陷；两处非阻塞观察（v1-contract Bearer 描述可澄清、vendored 上游同步）记录于 Result Summary 的 Next action | no |

## Object and Scope

- Task manifest: task.yaml
- Review date: 2026-08-26
- In-scope implementation:
  - `third_party/sfo-account/src/account_manager.rs`：`decode_session` 在
    验签与过期检查后新增 `token.sub == refresh_sub` 拒绝分支，返回
    `SessionInvalid`；
  - `docs/modules/filehub.md`：account 行补充 refresh 仅可用于续期的边界
    说明；
  - `server/tests/unit/account.rs`：新增
    `decode_session_rejects_refresh_session`；
  - `server/tests/api_integration.rs`：新增
    `refresh_session_cannot_access_user_apis`。
- Review mode: independent falsification（单智能体环境：验收 owner 未采信
  实现/测试自评，从提案与设计原文出发直接复核交付代码、消费方与运行产物，
  并构造反例逐一验证）

## Requirement Coverage

| change_id | Requirement or Boundary | Source | Implementation Evidence | Finding | Status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-refresh-decoder-reject | `decode_session` 拒绝 `sub == refresh_sub`（SessionInvalid），refresh token 不能映射为用户身份；正常 session 解码、过期语义与续期流程不变 | `proposal.md` prop-044-refresh-decode + `design/account-refresh.md` | `third_party/sfo-account/src/account_manager.rs:340-358`（验签 -> is_expire -> refresh_sub 拒绝 -> Ok(token.data)）；消费方 `server/src/http/auth.rs:19`、`server/src/account/authn.rs:9`、`third_party/sfo-account/src/account_server.rs:126,145` 均复用该收口；`refresh_session`（account_manager.rs:324-338）未改动 | 实现与提案/设计一致；回归断言直击拒绝路径 | pass |
| fh-refresh-regression | 覆盖 refresh 不能用作访问凭据的回归测试；续期端点与换发后 session 行为不变 | `proposal.md` prop-044-refresh-regression + testing.md/testplan.yaml | `server/tests/unit/account.rs` `decode_session_rejects_refresh_session`（decode(refresh) -> SessionInvalid、decode(session) -> alice、refresh(refresh) 轮换成功）；`server/tests/api_integration.rs` `refresh_session_cannot_access_user_apis`（/api/v1/projects 401、/account/get_account_info err!=0、refresh 换发后新 session 200）；统一运行产物 `.harness/test-results/test-runs/20260826T030930Z-filehub+044-refresh-token-session-only-all.json` all 全绿 | 实现与提案/设计一致；red-green 复现已录制 | pass |

## Independent Defect Discovery

| Category | Applicable Scope | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|------------------|--------------------|-------------------|----------------------------------|--------|
| requirement-and-behavior | 提案 P044 两项 + 用户「refresh token 只能用于 refresh」 | proposal.md Proposal Items、design/account-refresh.md、account_manager.rs:340-358、三个消费方与两个新增测试 | 逐项核对：refresh 不能映射用户、正常 session 解码不变、续期仍可用、非目标（不新增 claim/白名单/DB、不改有效期与 CLI/web）全部守界 | 核对后未发现需求遗漏、需求矛盾或越界行为；提案每项均被实现与测试直接覆盖 | pass |
| logic-and-control-flow | decode_session 分支与顺序 | account_manager.rs:340-358、refresh_session:324-338、authn.rs:9、auth.rs:19 | 构造分支假想：验签失败、过期、refresh_sub、无 sub 四路，确认拒绝优先且顺序固定；refresh_session 的 sub 校验保持原状不反向影响 | 顺序固定为 fail closed，无漏放行/误伤分支 | pass |
| boundary-and-input | token.sub 判别域与配置边界 | SessionConfig::validate（account_manager.rs:75-78）、generate_session:277-299、默认/自定义配置测试 | 三态断言：无 sub（放行）、refresh_sub（拒绝）、其他自定义 sub（配置校验封死相同组合；sfo-account 临时目录 inline 套件 6/6 通过含 custom subs）；过期 token 仍按 SessionExpired 分类 | 边界判别无绕过，自定义组合由构造入口静态封死 | pass |
| state-and-data-integrity | SessionConfig claims 判别与轮换语义 | generate_session:277-299、decode_session:340-358、refresh_session:324-338 | 检查非法迁移：refresh -> authenticated 被新增分支拒绝；refresh -> rotated 保持原语义；无持久化状态/缓存/事务参与 | 无非法迁移路径；refresh 轮换状态迁移未受影响 | pass |
| error-handling-and-recovery | 解码失败后的错误分类与认证桥 fallback | errors.rs（AccountErrorCode::SessionInvalid）、authn.rs:9、api_integration.rs 新用例、既有 garbage decode 断言 | 验证错误码断言（SessionInvalid）、HTTP 映射（受保护接口 401、sfo-account 信封 err!=0），且携带无效 Bearer 时认证桥 fail closed（不降级为匿名 200） | 错误分类与失败路径正确；无吞错/错误放行 | pass |
| resource-lifetime-and-cleanup | 本任务交付 diff 中是否引入新资源生命周期（句柄/事务/任务/定时器/连接） | 交付 diff（仅 account_manager.rs 新增一段同步只读比较） | 检查句柄/事务/任务/定时器/连接获取：无新增资源，解码路径仍是纯函数式局部变量（Payload/token.data 由 jsonwebtoken 与调用方管理） | not-applicable: 本任务未新增任何资源获取、释放、任务或连接生命周期管理，无可审查的清理路径 | not-applicable |
| concurrency-and-ordering | 并发认证请求下的解码与轮换 | decode_session:340-358、refresh_session:324-338、SessionConfig 不可变配置 | 构造并发假想：多请求同时 decode refresh/session；新增分支只读比较 `session_config.refresh_sub`（不可变），无共享可变状态/锁/排序要求 | not-applicable: 改动为无副作用的只读判别，无并发原语与顺序语义；既有轮换/撤销并发边界未变更 | not-applicable |
| interface-and-compatibility | trait/HTTP/凭据格式与消费者契约 | account_manager.rs trait 签名（Trait decode_session）、api_integration.rs 登录/refresh/项目流程、cli/tests/common/mod.rs 续期 mock、README 未涉及 | 验证 signature 不变、路由/DTO/信封/有效期不变；CLI 只在 401 后调用 `/account/refresh_session`（mock 233-239 行续期路径）从不把 refresh 当访问凭据；vendored crate 非 workspace 成员（根 Cargo.toml members=[cli, server]），其 inline 套件经独立临时目录 6/6 通过 | 无外部消费者受影响；行为收紧是缺陷能力而非契约能力 | pass |
| security-and-capacity | 认证冒充主攻击面与放大面 | account_manager.rs:340-358、auth.rs:19、authn.rs:9、tokens resolve 路径（server/src/tokens/service.rs）、api_integration.rs 新用例 | 尝试绕过：伪造剥离 sub 的 refresh（需 HMAC 密钥，与伪造 session 等价）；refresh 走 token 验签路径（EdDSA 每 token 公钥，HS256 不可互冒）；未过期 refresh 冒充 session（新分支拒绝）；无资源放大/注入输入面 | 攻击面在解码收口关闭，且无放大路径；修复前已泄露的 refresh 仍可续期属运维处置（提案已列出） | pass |
| test-adequacy | 新测试能否暴露正常/边界/反例/错误/生命周期/跨模块缺陷 | 新增两个测试的断言、testplan.yaml 四步、运行产物（unit 49 / dv 2 / integration 17 全绿）、red-green 录制 | 评估缺失/回退场景可见性：decode(refresh) 放行（红）、session 被误拒（放行侧断言）、401 变 200（红）、错误码漂移（SessionInvalid 断言）、轮换断裂（新 session 401）均可被抓；既有全套守护登录/token/上传回归；gap 记录：vendored inline cfg(test) 不能经本仓库 canonical 入口运行（非 workspace 成员、不改其 manifest 超出任务范围），自定义 session_sub 由配置校验静态封死且临时目录 6/6 通过；/account/get_account_info_of_session 与 get_account_info 同代码路径，解码收口测试已覆盖 | 测试足以暴露本修复的失败模式；记录 gap 不隐蔽缺陷 | pass |

## Document Consistency

| Document | Source | Implementation Consistency | Finding | Status |
|----------|--------|----------------------------|---------|--------|
| proposal | proposal.md | 两项提案的 requirement/boundary/success_evidence 与交付代码、测试一一对应；非目标未被越界修改 | 无 mismatch | pass |
| design | design.md + design/account-refresh.md | File-Level Interfaces 与实现一致（decode_session 新增分支、消费方复用），Scope Paths/Implementation Order 与 changed-path 证据吻合 | 无 mismatch | pass |
| testing | testing.md + testplan.yaml | Direct Change Coverage/Case-Type/Design Element 表格与 testplan 步骤、运行产物一致；red-green 复现记录与实测一致 | 无 mismatch | pass |

## Result Summary

- Overall result: accepted
- Outcome: 高危认证冒充（30 天 refresh token 冒充普通 session 访问用户接口）已在
  `decode_session` 唯一收口修复，并配齐单元/API 回归与 red-green 证据；
  49 unit + 2 dv + 17 integration + workspace compile closure 全部通过。
- Blocking issues: none recorded
- Next action: 完成 lifecycle 收尾并从任务索引移除；后续建议（不阻塞交付）：
  1) `docs/api/v1-contract.md:3` 的 Bearer 描述（`session|refresh|token-jwt`）
     可进一步澄清 refresh 仅用于 `/account/refresh_session`；
  2) 上游 crates.io sfo-account 若后续解除本仓库 patch，需把本修复随源码
     合并到上游并回归验证。

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 解码收口已在验签与过期检查后拒绝 `sub == refresh_sub` 的 token，
  认证桥与 sfo-account 用户信息路由自动获得防线；正常 session、续期轮换与
  换发后访问保持原语义；回归测试覆盖拒绝/放行/错误/生命周期/跨模块五类
  场景并录制 red-green；统一任务级入口全绿，独立反例搜索未发现阻塞缺陷。
