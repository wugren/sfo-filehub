# filehub login 登录方式上下键选择

- Status: complete
- Owner module: filehub（cli 模块）
- Task manifest: docs/versions/v0.1/modules/filehub/013-cli-login-arrow-select/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/013-cli-login-arrow-select/proposal.md
- Affected paths: `cli/src/cli/login_handler.rs`（含单元测试）、`cli/Cargo.toml`、`Cargo.lock`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

把 011 交付的数字输入（「1=账号密码，2=Token」）改为 `dialoguer::Select` 的
↑/↓ 高亮加回车确认：候选项「账号密码」「Token」，默认高亮账号密码；选择结果
经 `login_mode_from_select_index`（0/1）映射为既有 `LoginMode`，密码/token
后续仍走原隐藏输入流程。非交互模式、显式选项与环境变量路径保持原判断顺序。
为跨平台方向键读取新增 `dialoguer = "0.12.0"`（含 crossterm/console 传递
依赖），版本由 Cargo.lock 锁定。

## Risk Screen

- Public contract, protocol, or CLI change: no（参数面、环境变量与退出码不变）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no（密码/token 仍不回显、不进参数与日志）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: yes（新增 dialoguer v0.12.0 及 console/crossterm 等传递依赖并锁定进 Cargo.lock；属已确认提案范围内、主流终端交互库，无运行时网络行为，不构成升级触发）
- Material UI, accessibility, localization, or navigation workflow change: yes（登录方式选择改为 ↑/↓ + 回车，中文文案；属本次已确认的提案范围，不触发回提案或升级）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-cli`（单元 + 集成全量，8 + 11 + 4 全部通过）；`rustfmt --edition 2024 --check cli/src/cli/login_handler.rs` 通过；PTY 键盘事件回放 `script -qec`：↑ 键选 Token（退出码 2 网络校验路径）与默认回车选账号密码（退出码 6 网络路径，均进入对应登录请求而非用法错误）
- Result: passed
- Residual risk or follow-up: 交互选择依赖真实终端键盘事件，PTY 回放覆盖 ↑/Enter；Windows 键盘读取由 dialoguer 实现，三平台构建矩阵不变；任务窗口内检测到 admin-web 三个文件被外部并行会话修改（17:52-17:53），与本任务无关，已保留未覆盖
