# filehub login 交互登录方式选择

- Status: complete
- Owner module: filehub（cli 模块）
- Task manifest: docs/versions/v0.1/modules/filehub/011-cli-login-mode-prompt/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/011-cli-login-mode-prompt/proposal.md
- Affected paths: `cli/src/cli/login_handler.rs`（含单元测试）
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

补齐已批准 003 契约（`design/cli.md` 登录交互流程第 2 步）的实现缺口：当
`filehub login` 在交互终端（stdin 为终端）运行且未通过命令行选项或环境变量
指定登录方式时，`collect_login_inputs` 先提示用户选择"1=账号密码 / 2=Token"，
再按所选模式收集输入；密码/token 继续回显隐藏（`rpassword`）。选择解析独立为
`parse_login_mode_choice`，对空输入、非 1/2、EOF 返回用法错误（退出码 1）。
显式选项、环境变量和非交互模式路径保持原判断顺序不变，互斥校验与退出码表不回退。

## Risk Screen

- Public contract, protocol, or CLI change: no（参数面、环境变量与退出码不变）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no（密码/token 仍不回显、不进参数与日志）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: yes（新增交互终端下的登录方式选择提示与中文文案；属于本次已确认的提案范围，不触发回提案或升级）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-cli`（单元 + 集成，8 + 11 + 4 全通过）；`rustfmt --edition 2024 --check cli/src/cli/login_handler.rs` 通过；PTY 冒烟 `script -qec './target/debug/filehub login http://127.0.0.1:9'` 分别回放选择 1（密码）、选择 2（token）、无效选择 3 与 EOF
- Result: passed
- Residual risk or follow-up: 验证阶段曾误触全仓 `cargo fmt` 格式化 57 个无关文件，已从任务基线快照恢复，最终变更清单仅含目标文件；后续格式检查限定目标文件。交互选择的真实终端路径依赖本地 PTY 验证；非交互/环境变量回归由既有测试用例覆盖
