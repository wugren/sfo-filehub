# 054-switch-published-sfo-account 验收报告

## Findings

| ID | Severity | Owning Stage | Correctness Category | Evidence | Problem | Blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-000 | none | none | overall | 独立反例搜索覆盖提案/设计/实现/测试与统一运行产物（Cargo.lock registry 0.2.1、短 key 校验、登录错误码/英文消息、refresh-only、旧字符串移除扫描、64 unit + 2 dv + 20 api + 1 cli e2e 全绿） | 未发现本任务范围内缺陷 | no |
| F-001 | none | none | interface-and-compatibility | `third_party/sfo-account/` 为 untracked 本地副本，删除后无 git 历史 | 回滚需从 `/tmp/filehub-sfo-account-shim-backup-20260827` 或 `.harness` 053 基线恢复 shim；design.md「Risks and Rollback」已记录，不阻塞交付 | no |

## Object and Scope

- Task manifest: task.yaml
- Review date: 2026-08-27
- Review mode: independent falsification（单智能体环境：验收 owner 未采信实现/测试自评，从提案与设计原文出发直接复核交付代码、消费方与运行产物，并构造反例逐一验证）
- In-scope implementation:
  - `Cargo.toml`：移除 `[patch.crates-io] sfo-account`；`Cargo.lock`：
    `sfo-account 0.2.1` registry 来源 + 校验和；
  - `third_party/sfo-account/` 本地 shim 移除（备份至 /tmp）；
  - `server/src/model/config.rs`：`UsersConfig::validate`（>=32 字节）；
  - `server/src/main.rs`：解析后调用校验；
  - `server/src/account/mod.rs`：改用返回 `AccountResult` 的
    `new_with_login_verifier_and_session_config`；
  - `server/tests/*` 与 `cli/tests/e2e_cli_server.rs`：fixture 密钥 32 字节、
    登录失败/限流断言收敛；
  - `docs/api/v1-contract.md`、`docs/modules/filehub.md`、`README.md` 文档同步。

## Requirement Coverage

| change_id | Requirement or Boundary | Source | Implementation Evidence | Finding | Status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-sfo-account-published-source | 删除 path patch 与本地 shim，`sfo-account` 由 crates.io registry 0.2.1 解析 | proposal.md P-001 + design.md Directly Mapped Change Items | `Cargo.toml` 无 `[patch.crates-io]`；`Cargo.lock` `sfo-account` v0.2.1 + `source = "registry+https://github.com/rust-lang/crates.io-index"` + checksum `8130c5...`；`third_party/sfo-account/` 目录不存在；`docs/modules/filehub.md` 记录 registry 来源 | 实现与提案/设计一致；workspace 全目标编译闭环通过 | pass |
| fh-sfo-account-conformance | 0.2.1 登录失败语义（未知 err=9、密码 err=10、限流 err=11 与英文消息）；session_key 最短 32 字节启动校验与非 panic 组装；测试/契约/模块/README 同步 | proposal.md P-002 + design/account-dependency.md | `UsersConfig::validate` 与 `main.rs` 调用；`account/mod.rs` `SessionConfig::default()` 结果传播；`server/tests/unit/config.rs` 31B/32B 边界；`api_integration.rs` err=9/10/11 与英文消息断言；v1-contract 同步；README 密钥说明 | 实现与提案/设计一致；旧中文消息与旧 fixture 密钥已无引用 | pass |
| fh-sfo-account-regression | 全量 workspace 回归 + 登录失败/限流/refresh 边界定向验证 | proposal.md P-003 + design.md Key Flows/Invariants | `test-run.py filehub/054-switch-published-sfo-account all` 产物 `.harness/test-results/test-runs/20260827T052905Z-filehub+054-switch-published-sfo-account-all.json`：contract-compile-closure、contract-removed-symbol-scan、unit（64）、dv（2）、api integration（20）、cli e2e（1）全绿 | 覆盖与运行产物一致，退出码 0 | pass |

## Independent Defect Discovery

| Category | Applicable Scope | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|------------------|--------------------|-------------------|----------------------------------|--------|
| requirement-and-behavior | 提案 P-001/P-002/P-003 + 用户「按2方案实现，不管verify_dummy的问题」 | proposal.md 三个 Proposal Items、design.md Scope/Non-goals、实际 diff 与统一运行产物 | 逐项核对：patch 移除、registry 0.2.1、短 key 校验、未知/密码/限流三码与英文消息、历史文档不改写、verify_dummy 不补 | 需求全部落地且未越界；用户确认的语义取舍（err 区分、无伪校验）如实交付 | pass |
| logic-and-control-flow | `UsersConfig::validate` 分支、`AccountModule::init` 错误传播、错误码断言 | server/src/model/config.rs、main.rs、account/mod.rs、unit/config.rs、api_integration.rs | 构造分支假想：31B 拒绝且不回显密钥、32B 放行；构造器失败走 `map_err` 不回退 panic；账号不存在/密码错误/超限三个分支逐一断言 | 分支与错误传播正确，无漏放行/误伤 | pass |
| boundary-and-input | session_key 长度边界、错误码边界、64 KiB 请求体边界 | unit/config.rs（validate_accepts_32_byte_session_key / validate_rejects_short_session_key）、api_integration.rs（err=9/10/11 断言与既有 body-limit 用例）、testplan | 验证 31B 拒绝且 32B 通过；未知/密码/限流三码与消息逐一断言；64 KiB body-limit 用例随 20/20 api integration 通过 | 未发现越界放行或误伤；边界断言直接命中分支两侧 | pass |
| state-and-data-integrity | 配置校验生命周期、Cargo.lock 依赖图 | main.rs（解析->校验->DB）、Cargo.lock、risk-profile.yaml | 检查：短 key 配置在 DB 初始化前失败；锁文件来源/校验和唯一；无 schema/迁移改动 | 无非法迁移或持久化状态变更 | pass |
| error-handling-and-recovery | 启动失败路径、登录错误分类 | main.rs、account/mod.rs、unit/config.rs、api_integration.rs | 验证短 key 错误消息不含密钥明文且不 panic；未知/密码/限流错误码与消息各自正确；既有 200 信封不回退 | 失败路径 fail closed，无吞错 | pass |
| resource-lifetime-and-cleanup | 删除的 untracked shim、新增资源生命周期 | `third_party/` 状态、/tmp 备份、交付 diff | 检查句柄/事务/任务/连接：本任务未新增资源；删除的 shim 有 /tmp 与 053 基线可恢复路径 | 删除可恢复；无新资源生命周期引入 | pass |
| concurrency-and-ordering | 并行测试干扰、运行时并发语义 | testplan 串行集成步骤、运行产物、design.md | 构造并发假想：添加的校验为启动期同步逻辑，不参与请求路径并发；测试按串行入口运行避免已知 502/EOF 干扰 | not-applicable: 运行时无新增并发/顺序/共享状态声明，验证以串行入口规避既有并行干扰 | not-applicable |
| interface-and-compatibility | v1 HTTP 契约、CLI/admin-web 消费面、fixture | docs/api/v1-contract.md、admin-web/src/api/client.ts、cli/src/apiclient、design.md Consumer Migration Closure | 验证消费方只按 err!=0/透传消息处理（无精确中文匹配）；`consumer-closure-check` 扫描旧中文消息、旧 patch、旧 fixture 密钥全部通过；cli e2e 全流程通过 | 迁移分类与消费方实际依赖一致；无隐藏调用方依赖 | pass |
| security-and-capacity | 登录枚举信息面、refresh-only、密钥最小长度、密钥泄露 | proposal Risks、risk-profile.yaml、account/mod.rs、api_integration.rs | 构造攻击假想：短 key 配置启动被拒；回复哈希不含 key；refresh token 走用户接口仍被拒；限流窗口不放大；账号枚举信息面为已确认接受项并记录 | 已接受的信息面回归被文档化；其余安全边界保持 fail closed | pass |
| test-adequacy | 正常/边界/负例/错误/兼容/生命周期/跨模块覆盖 | testing.md 各表、testplan.yaml、运行产物 | 评估缺失可见性：err 回退（断言 9/10/11）、消息回退（英文断言）、短 key 回退（31B 拒绝）、patch 回退（removed-symbol-scan）、fixture 回退（cli e2e）均可被抓；gap：sfo-account crate inline 测试非 workspace 成员不可经 canonical 入口运行，由 server/集成层断言覆盖且风险已记 | 测试足以暴露本任务失败模式；记录 gap 不隐蔽缺陷 | pass |

## Document Consistency

| Document | Source | Implementation Consistency | Finding | Status |
|----------|--------|----------------------------|---------|--------|
| proposal | proposal.md | 三个 Proposal Items 的 requirement/boundary/tradeoff/success_evidence 与交付代码、测试一一对应；非目标未越界 | 无 mismatch | pass |
| design | design.md + design/account-dependency.md | File-Level Interfaces、Implementation Order、Consumer Migration Closure、Scope Paths 与已交付路径一致（published-source 以 docs/modules/filehub.md 记录来源切换） | 无 mismatch | pass |
| testing | testing.md + testplan.yaml | Direct Change Coverage/Case-Type/Design Element 与 testplan 步骤、统一运行产物一致 | 无 mismatch | pass |

## Result Summary

- Overall result: accepted
- Outcome: `sfo-account 0.2.1`（crates.io registry）已替换本地 vendored shim；
  登录失败语义按用户确认的 0.2.1 现状适配；`session_key` 最短 32 字节启动
  校验与测试 fixture 同步到位；workspace 全目标编译、64 unit + 2 dv +
  20 api integration + 1 cli e2e 与旧符号移除扫描全部通过。
- Blocking issues: none as functional/defect-discovery；Harness 生命周期收据的
  proposal/design 绑定因本任务中途按 schema 补 P-003 与收敛 scope 而滞后
  （进程记录问题，见 Next action）。
- Next action:
  1) 修复 lifecycle 收据绑定（proposal/design 与最终 task.yaml 不一致）；
  2) 完成 `task-transition complete` 与任务索引移除；
  3) 部署侧提示：已有短于 32 字节的 `session_key` 配置需先加长，否则启动失败。

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 实现、契约与验证均满足已确认提案和设计方案；独立反例搜索未发现
  阻塞性功能缺陷。Harness lifecycle 收据绑定需要按进程工具修复后完成收尾，
  属于记录同步问题而非交付功能问题。
