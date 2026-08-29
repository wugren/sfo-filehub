# 将命令提示和运行时错误信息统一为英文

- Status: complete
- Owner module: filehub
- Task manifest: `docs/versions/v0.1/modules/filehub/064-english-user-facing-copy/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/064-english-user-facing-copy/proposal.md`
- Affected paths: `cli/src/**`、`cli/tests/**`、`server/src/**`、`server/tests/**`、`build-docker.sh`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

只翻译会作为 CLI 帮助、交互提示、状态/错误信息、server API/日志或构建脚本终端输出的
固定中文文本。保持命令、参数、状态码、错误分类、格式化占位符、结构化 JSON 和退出码
不变；同步更新直接断言这些文本的测试。注释、Rustdoc、测试说明、测试数据以及
`admin-web/**` 不修改。

## Risk Screen

- Public contract, protocol, or CLI change: yes — CLI 人类可读文案变化，但命令、参数、结构化输出和稳定退出码不变，不构成 material 高风险边界
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test --manifest-path cli/Cargo.toml --all-targets`（14 unit、16 API integration、14 command integration、4 DV、1 real-server E2E）；修正默认错误正文后再次运行 `cargo test --manifest-path cli/Cargo.toml --test cmd_integration`（14/14）；`bash -n build-docker.sh`；运行时中文字符串扫描；任务范围 `git diff --check`；`admin-web/**` 零差异断言
- Result: pass
- Residual risk or follow-up: 依赖精确中文句子的非正式脚本需改用退出码或 JSON；`cargo fmt --check` 仍报告任务前已存在于 `cli/src/cli/mod.rs`、`cli/tests/cmd_integration.rs` 和未修改的 `cli/tests/e2e_cli_server.rs` 的格式差异，本任务未为消除无关差异执行全文件格式化；管理后台明确未修改
