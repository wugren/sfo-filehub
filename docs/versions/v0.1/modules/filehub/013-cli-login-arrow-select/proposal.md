---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-21
---

## Approval Record

- approver: user
- approval_date: 2026-08-21
- user_statement: 确认（按提案显示内容、standard 层级执行，含 dialoguer 依赖引入）

# filehub-cli 登录方式上下键选择提案

Risk profile: not-created（standard 层级不创建风险档案）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
  - 确认记录：2026-08-21 当前用户回复「确认」，按提案显示内容以 standard 层级
    执行，接受引入 `dialoguer` 依赖；进入 lower-tier 交付流（pre-edit 基线 ->
    变更记录 -> 实现与验证 -> 独立缺陷发现 -> 完成报告）。
- Tier rationale / triggered boundaries: 本任务把 011 任务已交付的「输入 1 或 2」
  登录方式选择改为终端上下键（↑/↓）加回车确认的选择列表，影响仍仅限 `cli` 模块
  的登录交互；不改命令行参数、环境变量、退出码、凭据存储或服务端调用。变化点：
  (1) 用户可见交互工作流变更（与 011、006/007/008 同类，按仓库惯例 standard）；
  (2) 引入跨平台终端交互依赖 `dialoguer`（含 crossterm/console 传递依赖）并更新
  Cargo.lock，属于新增依赖面；该库为主流终端选择组件、版本由 Cargo.lock 锁定、
  无运行时网络行为，供应链影响评估为低、不构成 material consequence；若用户
  认为新增依赖属于实质性供应链风险，可选择 high-risk 全流程确认。无其它
  high-risk 触发边界。
- Proposal and tier confirmation: 本提案需获得当前用户明确确认后才能执行；用户
  可选择按本提案确认、以替换层级（trivial/standard/high-risk）确认，或要求修订提案。

## Background and Goal

011 任务交付了交互终端下的登录方式提示「请选择登录方式（1=账号密码，
2=Token）：请输入 1 或 2」。当前用户希望把该交互改为更贴合终端习惯的上下键
选择：用 ↑/↓ 在「账号密码」与「Token」两个候选项间移动高亮，回车确认，不再
要求用户输入数字。

目标：`filehub login` 在交互终端且未指定登录方式时，展示可上下键选择的登录
方式列表并回车确认，随后按所选方式收集（不回显的）凭据输入。

## Scope

### In scope

1. 交互终端下未指定登录方式时，用上下键（↑/↓）高亮选择「账号密码」/「Token」，
   回车确认；默认高亮「账号密码」；
2. 删除数字输入提示「（1=账号密码，2=Token）」与对应的数字解析路径；选择的映
   射保持稳定（账号密码 -> 密码登录，Token -> token 登录）；
3. 选择确认后，后续密码/token 收集逻辑不变（Username/Password/Token 提示与
   隐藏输入）；
4. 非交互模式（stdin 非终端）行为不变：未显式指定登录方式仍报用法错误
   （退出码 1）；显式选项与环境变量通道不变；
5. 为跨平台（Windows/macOS/Linux）终端键盘事件引入 `dialoguer` 依赖并更新
   `cli/Cargo.toml` 与 `Cargo.lock`；相应单元测试改为覆盖选择结果映射，交互
   冒烟验证改为 PTY 键盘事件回放。

### Out of scope

- 不新增、删除或修改命令行参数、环境变量、退出码；
- 不修改账号密码/token 的服务端调用、凭据保存与复用逻辑；
- 不改 Username/Password/Token 以外的交互提示（不做 TUI 化改造）；
- 不修改其它模块或服务端。

### Boundary with neighboring modules

仅修改 `cli/src/cli/login_handler.rs`、`cli/Cargo.toml`、
`Cargo.lock` 及对应测试；`apiclient`、`credential_store`、`archive`
与服务端契约不涉及。

## Requirement Review

需求合理：上下键选择是终端登录/配置类 CLI 的常见交互（如 docker login 后的
选择器、各种 wizard），比数字输入更直观，且不改变任何外部契约。采用
`dialoguer::Select` 直接获得跨平台原始键盘事件处理与高亮渲染，避免手工维护
termios/Windows 控制台的差异；代价是新增受锁依赖，已在层级判断中记录。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-cli-login-arrow-select | 交互未指定登录方式时用 ↑/↓ + 回车选择账号密码或 Token；删除数字选择路径；非交互与显式模式不变 | 仅限 `cli` 模块登录方式选择交互与依赖清单 | 新增 dialoguer 依赖换取跨平台方向键支持 | 单元测试覆盖选择映射；PTY 键盘事件回放选择两条路径；全量 `cargo test -p filehub-cli` 通过 | 不改命令面、不做 TUI 化、不改服务端 |

## Success Criteria

- 用户可见结果：交互终端裸跑 `filehub login` 时显示可上下键选择的登录方式
  列表（默认高亮「账号密码」），↑/↓ 移动高亮，回车确认后继续对应模式的隐藏
  输入；不再出现「1=账号密码，2=Token」的数字提示。
- 必需证据：`cargo test -p filehub-cli` 全量通过；PTY 冒烟覆盖 ↑ 键选 Token、
  默认回车选账号密码两条路径；非交互/显式模式回归用例通过；变更记录与完成
  报告记录依赖新增与验证。
- 明确非目标：不改命令面参数定义与退出码表；不改造其余交互为 TUI；不修改服务端。

## Risks

- 新增依赖（低）：`dialoguer` 及其传递依赖进入构建图，Cargo.lock 锁定版本；
  库无运行时网络/外部命令行为，跨平台键盘读取由库承担，CI 与三平台构建验证覆盖。
- 键盘事件在真实终端/伪终端的差异：以 PTY 回放验证 ↑/↓/回车，Windows 侧由
  dialoguer 的 WinApi 实现覆盖，构建矩阵保持不变。
- 交互回归：显式模式、非交互模式与互斥校验保持既有测试覆盖，不回退 011 已交付
  的隐藏输入安全约束。
