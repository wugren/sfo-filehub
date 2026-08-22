# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/016-cli-push-pull-commands.md

## Delivery Summary

- Outcome: filehub-cli 的命令面改为 push/pull 并统一 server:... 目标形态：
  `publish` 更名为 `push server:project:version:name + 本地路径`，新增
  `pull server:project:version:name + 精确输出文件路径`（SHA-256 校验后原子
  落盘），`versions`/`new-version`/`lock-version`/`delete-app` 分别改为
  `server:project` / `server:project:version` / `server:project:version:name`
  形态；旧命令 `publish`/`download` 直接移除不留别名。
- Handoff: 实现位于 `cli/src/cli/args.rs`（统一目标解析与 clap 命令面）、
  `cli/src/cli/push_handler.rs`、`cli/src/cli/pull_handler.rs`、其余五个 handler
  的目标形态迁移与 `cli/src/cli/mod.rs` 分发；测试位于 `cli/tests/api_integration.rs`
  与 `cli/tests/dv_tests.rs`，新增目标解析正反例、pull 精确路径/目录拒绝/缺版本
  拒绝等用例。本机 `cargo test -p filehub-cli` 33 项全量通过；真实 filehub-server
  联调完成 push -> versions -> pull（SHA 一致）与 new-version/lock/delete-app 全流程。

## Proposal Consistency

| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-cli-push | `publish` 改名为 `push server:project:version:name + 本地路径`，旧 `publish` 移除 | proposal.md P-001 | `Command::Push`、`push_handler.rs`、真实联调 `push succeeded: e2e:1.1.0:logs`；help 不再含 publish | 交付与提案一致 | pass |
| fh-cli-pull | `pull server:project:version:name` 到精确文件路径、SHA-256 校验、原子落盘；旧 `download` 移除 | proposal.md P-002 | `pull_handler.rs`、pull_verifies_sha_and_rejects_corrupt_body、pull_rejects_directory_target、真实联调拉回 logs.tar.gz 且 sha256 一致 | 交付与提案一致 | pass |
| fh-cli-unified-target | server:... 目标解析族与校验（右侧固定切分、空段拒绝） | proposal.md P-003 | args.rs 三组 parse 函数 + 单元正反例（IPv4/IPv6/无端口/缺段/空段） | 交付与提案一致 | pass |
| fh-cli-management-targets | versions/new-version/lock-version/delete-app 统一 server:... 形态 | proposal.md P-004 | 五个 handler 迁移 + api_integration 生命周期用例 + 真实联调 delete-app 404/locked 409 错误映射不变 | 交付与提案一致 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 审查 args.rs 解析函数、push/pull handler、五个迁移 handler、mod.rs 分发 | 代入 IPv4+端口、IPv6（`[::1]:8080`/`::1:8080`）、无端口 host、缺段/空段与冒号输入；审查 pull 临时文件/父目录/覆盖与目录拒绝路径 | 解析正反例符合右侧切分契约；pull 精确路径原子落盘、校验失败与中途失败均清理临时文件；push/pull 续期、409/403/404 错误映射未回归 | pass |
| boundaries-and-failure-paths | 单元与集成正反例：目录目标、corrupt 流、缺版本/空 name、未登录、token 只读、locked 删除 | 挑战「pull 到不存在父目录」「覆盖既有文件」「download 旧 latest 快捷方式」「带端口 server 缺版本段的歧义」 | 不存在父目录自动创建；覆盖文件经 remove+rename 成功；latest 快捷方式随 download 移除（已确认变更，非缺陷）；带端口 server 下缺版本段与合法形态存在右切分歧义（F-1，文档化） | pass |
| regression-and-side-effects | 全量 `cargo test -p filehub-cli`（14 单元 + 15 集成 + 4 dv）与真实服务端联调；README/模块文档 | 检查旧 publish/download 引用残留、命令帮助表、003 设计/测试计划对命令面依赖、`cargo check --workspace` | 源码/测试/README/module 文档无 publish/download 残留；help 表与目标形态一致；003 testplan 仅跑 cargo test 不受影响；workspace 编译通过 | pass |

## Verification

- Targeted check: `rustfmt --edition 2024 --check`（本次实际改动 10 个源/测试文件）、
  `cargo test -p filehub-cli`（14 + 15 + 4 = 33 全通过）、`cargo check --workspace`、
  `filehub --help` 与 push/pull/versions/delete-app 子命令帮助、真实 filehub-server
  端到端（login -> new-version -> push -> versions -> pull SHA 一致 -> delete-app
  -> lock-version -> locked delete 409）
- Result: passed
- Exception reason: not-applicable

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | args.rs 右侧切分对 `host:port:project:name`（缺 version）会按 server=host、project=port 解析 | `server:project:version:name` 若服务器带端口且漏写 version，右切分无法与合法形态区分，属于格式固有歧义；已文档化，测试只对无歧义非法输入断言 exit 5 | no |
| F-2 | low | docs/api/v1-contract.md 的 003-cli 说明仍写 `publish --app`/download 示例 | 提案明确将 `docs/api/v1-contract.md` 划为 out of scope（API 契约未变），CLI 命令示例行留在后续文档同步项；如需要可单独小任务更新 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 交付完整覆盖已确认提案 P-001/P-002/P-003/P-004：push/pull 与六条命令的
  `<server:...>` 目标形态已实现，旧命令移除，解析正反例、精确路径语义、错误映射
  与 401 续期均有自动化与真实联调证据；独立缺陷发现三分类全 pass，仅两条 low 级
  非阻塞发现（格式固有歧义文档化、契约文档 CLI 示例行留作后续同步），无阻塞项。
