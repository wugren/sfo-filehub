# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/011-cli-login-mode-prompt.md

## Delivery Summary
- Outcome: `filehub login` 在交互终端运行且未通过命令行选项或环境变量指定
  登录方式时，现在会先提示"请选择登录方式（1=账号密码，2=Token）"，再按所选
  模式收集用户名/密码或 token；密码与 token 保持隐藏输入，无效选择与 EOF 报
  用法错误并退出码 1。显式选项、环境变量和非交互模式行为不变。
- Handoff: 实现集中在 `cli/src/cli/login_handler.rs`（新增
  `prompt_login_mode`/`parse_login_mode_choice`，密码收集拆出 `password_inputs`
  ）并补充 2 个单元用例；全量 `cargo test -p filehub-cli` 通过，PTY 冒烟回放
  两种登录方式与失败路径。无遗留阻塞问题。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-cli-login-mode-prompt | 交互模式下未指定登录方式时提示用户选择账号密码或 token，再按所选方式收集不回显输入；无效/EOF 选择报用法错误（退出码 1）；命令面与非交互行为不变 | proposal.md P-001（Scope 与 Proposal Items） | `login_handler.rs` 的 `collect_login_inputs` 接入交互选择并复用原凭证收集；`cargo test -p filehub-cli` 与 PTY 冒烟均通过 | 交付与提案一致 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 审查 `collect_login_inputs` 全部旧分支与新增 `prompt_login_mode`/`parse_login_mode_choice` 的实现 | 代入互斥选项、环境变量+终端、空行、带空白选择、'12' 等输入对照预期 | 模式判断保持「显式选项 > 环境变量 > 交互提示」顺序，互斥校验未回退；'1'/'2' 正确映射，其余输入与 EOF 均落入用法错误 | pass |
| boundaries-and-failure-paths | PTY 回放选择 1（密码）、2（token）、3（无效）与 EOF；既有 `non_terminal_without_mode_is_usage_error` 用例 | 挑战终端与管道混用、空输入、无效选择、token 校验失败路径 | 选择 3 与 EOF 均以退出码 1 报用法错误；非终端未显式选模式仍报用法错误；token 校验失败保持认证/网络退出码（2/6） | pass |
| regression-and-side-effects | 全量 `cargo test -p filehub-cli`（8 单元 + 11 集成 + 4 流程）；基线快照比对 126 个未跟踪文件 | 检查环境变量通道、互斥选项、401/403/409 与 logout 流程；比对全仓是否出现任务外改动 | 全部用例通过；验证期间误触全仓 `cargo fmt` 曾格式化 57 个无关文件，已从基线快照恢复，最终变更清单仅含目标文件 | pass |

## Verification
- Targeted check: `cargo test -p filehub-cli`（单元 + 集成全量）；`rustfmt --edition 2024 --check cli/src/cli/login_handler.rs`；PTY 冒烟 `script -qec './target/debug/filehub login http://127.0.0.1:9'`（选择 1/2/3/EOF 四条路径）
- Result: passed
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `cargo fmt` 全仓回退与基线快照比对（57 个无关文件被格式化） | 验证阶段误格式化无关源码；已从 .harness 基线恢复，最终变更清单仅含目标文件，对交付无影响 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 交付完整覆盖已确认提案 P-001 的全部要求：交互选择、隐藏输入、无效/EOF
  用法错误与非交互/显式模式不变均已实现并验证；独立缺陷发现三分类全部通过，
  无阻塞发现，targeted verification 通过。
