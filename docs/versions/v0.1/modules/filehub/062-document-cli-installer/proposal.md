---
task_manifest: task.yaml
status: approved
---

# 根 README 使用在线脚本安装 CLI

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 当前根 `README.md` 的命令先把脚本下载到工作目录，再由用户单独执行；用户要求
    改成一条命令在线获取脚本并立即安装；
  - 该变更不修改安装脚本、CLI、Release 产物或安装目录行为，但会改变公开安装入口，
    且远程脚本立即执行涉及供应链信任与安全提示，不满足 `trivial` 的无公共安装契约、
    无供应链信任影响条件；
  - 影响仍局限于根 README 与对应文档契约测试，固定脚本来源和安装行为均不变化，
    无新增权限、发布、兼容或运行时后果，因此建议使用 `standard`，不升级为
    `high-risk`。
- Proposal and tier confirmation: 2026-08-29 用户回复“确认，自动完成”，批准本提案与
  `standard` 层级，并授权在不扩大范围的前提下自动完成实施、验证和验收。

## Approval Record

- approver: 用户
- approval_date: 2026-08-29
- user_statement: “确认，自动完成”
- selected_tier: standard

## Background and Goal

根 `README.md` 已提供 `install-cli.sh` 和 `install-cli.ps1`，但示例需要先下载脚本、
再执行本地文件。目标是改为可直接复制的一行在线安装命令：命令从
`raw.githubusercontent.com/wugren/sfo-filehub/main` 获取脚本并立即执行；省略版本
安装 GitHub 最新正式 Release，传入版本则安装指定版本。

## Scope

### In scope

- 根 `README.md` 的 Linux/macOS 最新版示例明确使用 GitHub 原始脚本地址：
  `curl -fsSL https://raw.githubusercontent.com/wugren/sfo-filehub/main/install-cli.sh | sh`；
- Linux/macOS 指定版本示例为
  `curl -fsSL https://raw.githubusercontent.com/wugren/sfo-filehub/main/install-cli.sh | sh -s -- 0.1.0`；
  自定义目录同样通过 `sh -s --` 传入参数；
- Windows 最新版示例为一条命令：
  `& ([scriptblock]::Create((Invoke-RestMethod -Uri 'https://raw.githubusercontent.com/wugren/sfo-filehub/main/install-cli.ps1')))`；
  指定版本在同一条命令末尾追加 `-Version 0.1.0`，自定义目录追加
  `-InstallDir "$HOME\bin"`；
- 每个示例均在同一条命令中完成在线下载与安装，不生成需要用户再次执行的本地脚本；
- 文档明确命令会在线获取并立即执行固定仓库 `main` 分支的安装脚本，提示用户可先
  审阅脚本内容；保留手工下载 CLI Release 归档作为不执行远程脚本的备用方式；
- 更新 `tests/install_cli_scripts_contract.py` 的 README 契约断言，并验证 Shell
  脚本从标准输入执行时仍能接收省略版本、指定版本和自定义目录参数；
- 运行安装脚本定向契约测试、Shell 语法检查和差异检查。

### Out of scope

- 不修改 `install-cli.sh`、`install-cli.ps1`、`cli/README.md` 或任何 CLI 产品代码；
- 不改变版本解析、默认安装目录、提权、Windows 机器级 `Path`、支持平台或 Release
  资产契约；
- 不新增 checksum、签名或脚本固定 commit URL；脚本来源仍为用户指定仓库的
  `main` 分支；
- 不执行真实 GitHub 下载、真实系统目录安装、提交或推送；
- 不触碰工作树中已有的 `Cargo.lock`、`harness/scripts/edit-guard.py`、
  `filehub-server.yaml`、`filehub.db` 等无关改动。

### Boundary with neighboring modules

本任务只调整根 README 的安装调用方式及其本地契约测试。安装脚本仍消费任务 061
已有的 GitHub Release 契约，CLI 行为、发布流水线、服务端和管理后台不变。

## Requirement Review

需求合理。一行在线安装命令比“下载、授权、执行”三步更容易复制，也能自然支持省略
版本安装最新正式版。Shell 使用 `sh -s --` 显式分隔解释器参数和安装脚本参数；
PowerShell 使用 `[ScriptBlock]::Create((Invoke-RestMethod ...))`，从而可以在不落地
脚本文件的情况下继续传入 `-Version` 和 `-InstallDir`。

主要权衡是在线获取后立即执行不再给用户默认的本地审阅步骤。README 将明确来源为
固定仓库 URL、提示可先审阅脚本，并保留手工下载 CLI 归档作为备用方式；本任务不虚构
当前不存在的 checksum 或签名保证。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-online-cli-installer-docs | 根 `README.md` 为 Linux/macOS 与 Windows 提供在线获取并立即执行安装脚本的一行命令，覆盖最新版本、指定版本和自定义目录 | 只修改根 README 与对应安装文档契约测试；脚本 URL 固定为 `wugren/sfo-filehub` 的 `main` 分支 | 一行执行更方便，但减少默认审阅步骤；增加来源说明、审阅提示并保留手工归档备用方式 | README 契约检查确认 POSIX 管道、PowerShell ScriptBlock、latest/指定版本/自定义目录及安全提示；Shell 标准输入模拟安装测试、原安装契约测试和 diff 检查通过 | 不修改安装脚本、发布资产、权限行为、平台矩阵或真实系统状态 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - Linux/macOS 用户可复制一行 `curl ... | sh` 安装最新版本，或使用
    `sh -s -- 0.1.0` 安装指定版本；
  - Windows 用户可复制一行 PowerShell 命令在线加载并执行脚本，且可省略或指定
    `-Version 0.1.0`；
  - 自定义目录示例同样不要求先在本地保存脚本；文档说明远程执行边界并保留手工
    归档安装入口。
- Required evidence:
  - 安装脚本契约测试的 unit、dv、integration 套件通过；
  - Shell 在线调用形态通过标准输入模拟，证明 latest、指定版本和自定义目录参数可达
    现有脚本；
  - README 定向断言覆盖在线 URL、POSIX 管道、PowerShell ScriptBlock、版本参数、
    自定义目录、安全提示和手工备用方式；
  - `bash -n install-cli.sh` 与 `git diff --check` 通过；PowerShell 环境可用时继续执行
    解析检查。
- Explicit non-goals:
  - 不声称本地模拟证明真实 GitHub、目标系统权限或未来 Release 资产可用；不改变
    安装脚本与发布能力。

## Risks

- `main` 分支上的远程脚本将被立即执行，内容可能随仓库更新；README 必须明确该
  行为并提供审阅与手工归档替代入口。
- POSIX 管道中的参数必须传给 `sh -s --` 而不是误传给 `curl`；PowerShell 必须先
  创建 `ScriptBlock` 才能可靠传入命名参数。契约测试会固定这两种调用形态。
- 本地没有 Windows PowerShell 时只能做静态契约和可选解析检查，不能宣称真实 Windows
  系统安装已验证。
