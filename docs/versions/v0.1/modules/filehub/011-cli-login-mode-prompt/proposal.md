---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-21
---

## Approval Record

- approver: user
- approval_date: 2026-08-21
- user_statement: 确认（按提案显示内容、standard 层级执行）

# filehub-cli 交互登录方式选择提案

Risk profile: not-created（standard 层级不创建风险档案）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
  - 确认记录：2026-08-21 当前用户回复「确认」，按提案显示内容以 standard 层级执行；
    进入 lower-tier 交付流（pre-edit 基线 -> 变更记录 -> 实现与验证 ->
    独立缺陷发现 -> 完成报告）。
- Tier rationale / triggered boundaries: 本次修改补齐已批准契约（003 提案与
  `design/cli.md`）中"stdin 为终端且未指定登录方式时，先提示选择账号密码或
  token"的交互行为；影响范围仅限 `cli` 模块的登录交互流程，不改变命令行参数
  契约、凭据存储、网络协议或退出码，也不涉及数据迁移、依赖或发布面。由于该
  变更属于用户可见的交互工作流行为，按仓库惯例（006/007/008 同为用户可见行为
  变更采用 standard）建议 standard，不做 trivial；无 high-risk 触发边界。
- Proposal and tier confirmation: 本提案需获得当前用户明确确认后才能执行；用户
  可选择按本提案确认、以替换层级（trivial/standard/high-risk）确认，或要求修订提案。

## Background and Goal

`filehub login` 在交互终端且未通过命令行选项或环境变量指定登录方式时，当前
实现直接进入账号密码模式。已经批准的 003 提案与 `design/cli.md` 明确要求：
"无凭据选项且 stdin 为终端：提示选择账号密码或 token；密码/token 不回显"。
当前行为与已冻结契约不符：用户无法在交互模式下选择 token 登录，也无法先看到
两种登录方式的明确入口。

目标：`filehub login` 在交互终端且未指定登录方式时，提示用户选择登录方式
（账号密码 / token），再按所选方式收集输入；输入不回显。

## Scope

### In scope

1. 交互终端（stdin 为终端）下 `filehub login` 未指定任何登录方式（无
   `-u/--username`、`--password-stdin`、`--token-stdin`，且
   `FILEHUB_USERNAME`/`FILEHUB_PASSWORD`/`FILEHUB_TOKEN` 均未提供）时，先提示
   选择登录方式：账号密码（1）或 token（2）；
2. 选择后按对应模式收集输入：账号密码模式提示 Username/Password，token 模式
   提示 Token；密码与 token 输入均不回显；
3. 选择输入无效（非 1/2、空输入或 EOF）时给出用法错误并保持稳定退出码 1；
4. 显式指定登录方式（命令行选项或环境变量）时行为不变，直接按原优先级执行；
5. 非交互模式（stdin 非终端）行为不变：未显式选择登录方式时仍报用法错误
   （退出码 1），不读取管道内容。

### Out of scope

- 不新增、删除或修改任何命令行参数、环境变量或退出码；
- 不改变账号密码/token 登录的服务端调用、凭据保存与复用逻辑；
- 不做交互式 TUI/图形界面；
- 不修改 005/006/007/008 等其它任务范围。

### Boundary with neighboring modules

仅修改 `cli/src/cli/login_handler.rs` 的输入收集逻辑及对应测试；`apiclient`、
`credential_store`、`archive` 不涉及；服务端契约不变。

## Requirement Review

需求合理，且是对已批准 003 契约实现缺口的修复，不构成新的对外契约变化。当前
实现已满足"显式选项 > 环境变量 > 交互提示"的优先级，但交互提示阶段缺少登录
方式选择一步，导致交互用户无法选择 token 登录。选择方式提示放在凭据输入收集
之前，属于最小改动，保持密码/token 不回显的既有安全要求。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-cli-login-mode-prompt | 交互模式下未指定登录方式时提示用户选择账号密码或 token，再按所选方式收集不回显输入；无效/EOF 选择报用法错误（退出码 1） | 仅限 `cli` 模块登录输入收集；命令面与非交互行为不变 | 交互多一步选择，脚本/CI 不受影响（显式模式或非终端仍走原路径） | 单元测试覆盖选择分支（密码/token/无效输入/EOF）与既有模式优先级回归；CLI 测试套件通过 | 不改命令参数、不改变实现服务端认证 |

## Success Criteria

- 用户可见结果：交互终端运行裸 `filehub login` 时首先出现登录方式选择提示
  （账号密码 / token），按所选方式继续并成功登录；输入密码/token 不回显；
  无效选择提示明确用法错误并以退出码 1 结束。
- 必需证据：任务变更记录记录实现与验证；`login_handler` 单元测试与 CLI
  Harness 测试通过；非交互/显式模式回归用例通过。
- 明确非目标：不改命令面参数定义与退出码表；不新增 TUI/图形界面；不修改服务端。

## Risks

- 交互提示文案与选择解析错误可能导致用户误选或死循环：通过受限重试与
  无效输入即报错的单次失败策略控制（与现有"用法错误即退出"的行为一致）。
- 凭据安全：保持密码/token 不回显这一既有约束，不引入新输入通道；
  本次不向命令行参数或日志增加任何明文凭据路径。
