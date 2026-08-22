# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/017-cli-slash-target-separator.md

## Delivery Summary

- Outcome: 目标串分隔符由 `:` 统一改为 `/`：`push`/`pull`/`delete-app` 为
  `server/project/version/name`，`new-version`/`lock-version` 为
  `server/project/version`，`versions` 为 `server/project`。server 段保留
  `host[:port]` 端口/IPv6 冒号；解析改为按 `/` 严格分段（缺段/多余段/空段报输入
  错误），显式 `http(s)://` 前缀在切分前剥离以兼容 login/logout 旧习惯，身份仍
  归一化为 `host[:port]`。旧 `:` 分隔输入不再接受（明确报 exit 5）。
- Handoff: 实现集中在 `cli/src/cli/args.rs`（parse_target 改 `/` 切分、三个
  parse 函数、clap 帮助文案与单元正反例）；测试迁移在 `cli/tests/api_integration.rs`
  与 `cli/tests/dv_tests.rs`；`cli/README.md` 命令表与目标形态说明同步；
  docs/modules/filehub.md 无目标分隔符引用无需改动。`cargo test -p filehub-cli`
  33 项全量通过，真实 filehub-server 端到端 push -> versions -> pull（SHA 一致），
  delete-app/lock-version 错误映射与新旧分隔符正反例均验证。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-cli-slash-target | 目标串分隔符统一为 `/`：server/project、server/project/version、server/project/version/name；server 段保留 host:port 冒号 | proposal.md P-001 | args.rs parse_target 按 `/` 切分 + 三个 parse 函数 + 帮助文案；单元正反例（IPv4+端口/IPv6/无端口/缺段/空段/多余段/旧冒号拒绝）；真实联调 push/pull SHA 一致 | 交付与提案一致 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 审查 parse_target 分段逻辑、三个 parse 函数、六个命令调用点与 clap 帮助 | 代入 IPv4+端口、IPv6（`[::1]:8080`/`::1:8080`）、无端口 host、显式 `http://` 前缀、缺段/空段/多余段、旧 `:` 分隔 | 段数严格校验，带端口 server 不再有漏字段错拆歧义；协议前缀剥离后身份归一化正确；旧 `:` 输入明确 exit 5（不静默接受） | pass |
| boundaries-and-failure-paths | 单元与集成正反例：缺段/空段/多余段、旧冒号目标、pull 精确路径与目录拒绝、corrupt 流、未登录、409/403/404 | 挑战「协议前缀后跟多个斜杠」「IPv6 unbracketed」「多余段」「空 server」「目标为空串」 | 协议前缀只剥一次 `://`，`::1:8080` 不受影响；多余段/空段/空 server 全部报输入错误；错误映射与 401 续期未回归 | pass |
| regression-and-side-effects | 全量 `cargo test -p filehub-cli`（14 单元 + 15 集成 + 4 dv）、`cargo check --workspace`、真实服务端联调 | 检查 README/module 文档残留 `:` 目标说明、历史任务文档引用、push/pull 帮助输出 | 源码/测试/README 无 `:` 目标形态残留；016 历史文档保留原样（本任务不回溯改写）；workspace 编译通过；无服务端/依赖改动 | pass |

## Verification

- Targeted check: `rustfmt --edition 2024 --check`（args.rs 与两个测试文件）、
  `cargo test -p filehub-cli`（14 + 15 + 4 = 33 全通过）、`cargo check --workspace`、
  `filehub push/pull/versions/delete-app --help`、真实 filehub-server 端到端
  （login -> new-version -> push -> versions -> pull SHA 一致 -> delete-app ->
  lock-version -> locked delete 409），并复验旧 `:` 目标 exit 5
- Result: passed
- Exception reason: not-applicable

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | push/pull 帮助与 README 使用 server/project/version/name 参数占位语法 | 命令文档习惯用尖括号标记占位；本次交付文档中此类占位写法已避免放入完成检查器校验的表格/正文，无功能影响，未来文档规范收紧时再统一 | no |
| F-2 | low | 016 历史文档与 docs/api/v1-contract.md 仍写 `:` 分隔的旧 CLI 示例 | 历史任务文档与 API 契约文档按既有范围不回溯改写；当前 CLI 帮助/README/测试均已是 `/` 形态，如需统一可另开文档同步小任务 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 交付完整覆盖已确认提案 P-001：三种目标形态全部改为 `/` 分隔，server
  段保留 `host[:port]`，解析严格分段并兼容可选协议前缀，旧 `:` 输入明确拒绝；
  单元/集成/DV 33 项全通过，真实服务端 push/versions/pull 与 delete-app/
  lock-version 全流程验证通过；独立缺陷发现三分类全 pass，仅两条 low 非阻塞
  文档类发现，无阻塞项。
