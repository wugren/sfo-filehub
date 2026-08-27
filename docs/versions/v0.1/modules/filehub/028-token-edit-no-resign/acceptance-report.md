# 028-token-edit-no-resign 验收报告

## Object and Scope

- Task manifest: `task.yaml`
- Review mode: independent defect-discovery review——独立于实现/测试结论，直接
  以当前代码、测试与运行产物做逐类别反例搜索，再核对文档一致性（同一执行
  会话内由同一 reviewer 承担独立搜索职责，未复用实现自评结论）。
- Reviewed sources: proposal.md、design.md + design/{tokens,admin-web-tokens}.md、
  testing.md、testplan.yaml、server/src/tokens/*、admin-web/src（api/contract.ts、
  api/client.ts、pages/TokensPage.tsx、i18n/messages.ts）、server/tests/unit/
  tokens.rs、admin-web/tests（unit/components/TokensPage.test.tsx、unit/client.test.ts、
  integration/contract.test.ts）、test-run 产物
  `.harness/test-results/test-runs/20260823T170853Z-filehub+028-token-edit-no-resign-all.json`

## Findings

| id | severity | owning_stage | correctness_category | evidence | problem | blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-01 | none | none | interface-and-compatibility | `design.md` Consumer Migration Closure 初始把 `admin-web/tests/unit/client.test.ts` 与 `admin-web/tests/integration/contract.test.ts` 写在同一行的「Consumer Path」单元格，`consumer-closure-check.py` 无法按文件级路径闭合 | 记录项（已修正）：design 阶段后由 symbol scan 暴露的表格格式问题，已在 testing 阶段拆分为两行并重跑通过，不构成当前交付缺陷 | no |
| F-02 | none | none | interface-and-compatibility | `server/src/tokens/model.rs` `TokenUpdateRequest` 未启用 serde `deny_unknown_fields` | 记录项（可选加固）：旧客户端仍可携带已移除的 `expires_at` 字段并被静默忽略，不会重签或误签发，符合本需求保护方向；后续可考虑拒绝未知字段 | no |

## Requirement Coverage

| change_id | requirement_or_boundary | source | implementation_evidence | finding | status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-token-update-no-resign | 属性修改（name/project_scope/scopes）只落库返回 TokenSummary，不生成密钥/不签 JWT，旧 JWT exp 不被破坏、权限按数据库生效 | proposal P-001 + design/tokens.md | `server/src/tokens/{model,mod,service,http}.rs` 移除 expires_at/重签分支；`server/tests/unit/tokens.rs` `token_attribute_update_preserves_exp_and_does_not_resign` 断言 public_key_pem 字节级不变、原 JWT 的 exp 前后一致且仍可 resolve，`token_lifecycle...` 断言 update 返回摘要后旧 JWT 继续有效 | 无 | pass |
| fh-token-explicit-resign-action | 管理端显式「重新签发」按钮是唯一重签入口；编辑弹窗无有效期预设/无重签警告、保存不展示 JWT | proposal P-002 + design/admin-web-tokens.md | `admin-web/src/pages/TokensPage.tsx`、`i18n/messages.ts`、`api/client.ts`、`api/contract.ts`；`admin-web/tests/unit/components/TokensPage.test.tsx` 断言编辑保存请求体无 expires_at 且不展示 JWT、仅「重新签发」确认后展示 JWT | 无 | pass |
| fh-token-no-resign-regression-tests | server 与 admin-web 测试更新并覆盖 exp 保持、update 不产出 JWT、rotate 失效、契约形状 | proposal P-003 + design.md Consumer Migration Closure + docs/api/v1-contract.md | `server/tests/unit/tokens.rs`（24 unit）、`admin-web/tests`（42 unit + 7 integration）、`testplan.yaml` 5 个 contract 步骤（negative 夹具/符号扫描/编译闭包/文档闭包）在 `20260823T170853Z...all.json` 全部 exit 0 | 无 | pass |

## Independent Defect Discovery

| category | applicable_scope | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|------------------|--------------------|--------------------|-----------------------------------|--------|
| requirement-and-behavior | 提案 all change_ids 与成功信号 | proposal.md 范围/非目标、docs/api/v1-contract.md 更新、实现 diff | 反向核对：属性修改是否还可能产出 JWT？update 全路径无 generate_keypair/sign 调用；创建/轮换仍正常签发 | 未发现需求边界偏离：非目标（无新端点/无 schema/CLI 不变）均满足 | pass |
| logic-and-control-flow | service.rs update 分支与前端保存/重签流 | update 空 patch / 仅 name / 仅 scopes / only project_scope 分支；TokensPage saveToken/rotateToken 调用链 | 构造“expires_at 传入 update”的旧调用：DTO 已移除字段，编译期拒绝；构造全 None patch：返回当前摘要且不写库 | 分支覆盖与回归测试一致，未发现错误分支或不可达行为 | pass |
| boundary-and-input | TokenUpdateRequest 三字段与 JWT exp | tokens.rs 空操作/空集合/空 scopes 断言；contract.ts/客户端测试 | 空 scopes 集合、空 project_scope 集合（归一化 All）、全部字段缺省、仅 name；revoked token 上 update 只改属性不撤销状态（既有语义，非回归） | 未发现越界或错误处理；空 name 服务端不校验是既有行为（表单层拦截），非本任务引入 | pass |
| state-and-data-integrity | tokens/token_scopes 写入事务与 updated_at | service.rs tx（UPDATE tokens + DELETE/INSERT token_scopes 同一事务）、public_key_pem 前后断言 | 检测属性修改是否替换验签公钥/双表半提交：public_key_pem 不写入、作用域置于同一 tx；rotate 仍只换钥 | 无非法状态迁移或部分更新路径 | pass |
| error-handling-and-recovery | update/rotate/resolve 错误路径 | load_token_row not-found、sqlx 错误映射、rotate 后旧 JWT resolve 失败 | 越权修改他人 token（owner WHERE 条件）、数据库写失败回滚、旧 JWT 验签失败 -> 认证失败 | 错误分类与既有语义一致，未发现吞错/回滚缺陷 | pass |
| resource-lifetime-and-cleanup | update 与 rotate 的密钥对及 SQLite 事务句柄生命周期边界 | update 内不再创建 keypair；tx begin/commit 作用域；rotate 既有 keypair 生命周期未改 | 检查 update 是否遗留未提交事务或密钥缓冲：无 create/销毁新资源；sqlx tx drop 自动回滚 | not-applicable——本变更只删除签发副作用（少一个密钥对生命周期），不新增句柄/文件/内存资源；具体任务原因已上述 | not-applicable |
| concurrency-and-ordering | update 与 rotate/revoke 的写路径 | service.rs update（不写 public_key_pem）与 rotate/revoke（写 public_key_pem/revoked_at） | 并发 update+rotate 是否产生密钥/属性交错：update 不再触碰密钥列，写路径解耦；SQLite 事务串行化 | 未发现新增竞态；属性与密钥写的并发时序风险比旧代码（update 也换钥）更低 | pass |
| interface-and-compatibility | HTTP/TS/Rust 三面契约 | docs/api/v1-contract.md、contract.ts/client.ts 类型、service trait 签名、negative 夹具与 symbol scan 产物 | 旧字段 expires_at 编译失败（tsc 夹具）、removed-symbol scan、workspace compile closure、docs module 边界 | 无缺陷：F-01 为已修正记录项、F-02 为可选加固记录项，均不构成当前交付缺陷 | pass |
| security-and-capacity | 授权/吊销语义与凭据处理 | resolve 数据库权威路径、rotate 换钥、JWT 一次性展示、sign 私钥即弃 | 权限收紧后旧 JWT 是否仍可越权？resolve 每次查库取最新权限（fail-closed）；让旧副本失效的唯一手段是显式重签（用户确认的取舍）；无新增无界工作/存储 | 未发现可利用缺陷；意外转永久 token 的原缺陷已由“属性修改不重签”根除 | pass |
| test-adequacy | 全部 change_id 的测试能否暴露失败模式 | server 24 unit（新增 exp/密钥回归）、admin 42 unit + 7 integration、组件测试 2 例、contract 5 步 | 反向问：若 update 仍重签，哪个测试会红？public_key_pem/exp/无 JWT 断言均会红；若前端仍展示 JWT，组件测试会红；若 rotate 失效，lifecycle 断言会红 | 正常/边界/负向/兼容/生命周期/跨模块均有直接断言；测试文件均为仓库既有目标可达 | pass |

## Document Consistency

| document | source | implementation_consistency | finding | status |
|----------|--------|---------------------------|---------|--------|
| design | design.md、design/tokens.md、design/admin-web-tokens.md | 接口签名/事务/UI 流程与实现一致；Consumer Migration Closure 一行一文件路径（F-01 已修正） | 无残留不一致 | pass |
| testing | testing.md、testplan.yaml | testplan 步骤与 testing.md 层表/validation id 一致；运行产物 exit 0 | 无 | pass |

## Result Summary

- Overall result: accepted
- Outcome: 属性修改不再重签（TokenSummary、无签发副作用、旧 JWT exp 保持），
  显式「重新签发」为唯一重签入口；server 24 unit + 2 dv + 2 integration、
  admin-web 42 unit + 7 integration、5 个契约检查全绿。
- Blocking issues: none
- Next action: 任务收尾并移出未完成索引；F-02（未知字段容忍）留作后续可选
  加固，不阻塞交付。

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 三个 change_id 均有直接的实现证据与回归测试；十类反例搜索未发现
  阻塞缺陷；剩余两项低风险记录（设计表格式已修正、未知字段静默忽略）均已
  显式标注且不改变交付行为；harness 单据链（design/implementation/testing）
  重新推进后全部通过。
