---
task_manifest: task.yaml
status: approved
---

# 从 GitHub Release 安装 CLI

Risk profile: ./risk-profile.yaml

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: high-risk
- Tier rationale / triggered boundaries:
  - 脚本会从 GitHub Release 下载可执行文件并写入系统级目录，直接涉及供应链信任、主机权限和安装/升级行为；
  - Windows 安装还会修改机器级 `Path`，错误实现可能污染系统环境或覆盖非本产品文件；
  - 当前 Release 只提供 Linux x86_64、macOS aarch64、Windows x86_64 三种资产，且没有发布方 checksum；脚本必须拒绝不支持的平台组合，并明确保留仅依赖 GitHub HTTPS 与固定仓库/版本 URL 的完整性风险；
  - 因此建议使用 `high-risk`，通过独立设计、测试和验收约束下载、解包、提权、覆盖与清理边界。
- Proposal and tier confirmation: 2026-08-28 用户回复“确认，自动完成”，批准本提案与
  `high-risk` 层级，并明确授权从 Design 开始自动完成后续 Implementation、Testing
  和 Acceptance 阶段，无需逐阶段再次确认。

## Approval Record

- approver: 用户
- approval_date: 2026-08-28
- user_statement: “确认，自动完成”
- selected_tier: high-risk
- pipeline_mode: auto-pipeline
- first_auto_stage: design

## Background and Goal

当前 README 只说明手工下载并解压 CLI。目标是在仓库根目录提供可直接使用的安装
脚本：脚本自动识别操作系统和 CPU 架构；用户可传入指定版本，也可省略版本以安装
GitHub 标记的最新正式 Release，然后下载 `wugren/sfo-filehub` 对应 CLI 归档，并
安装为系统范围内可调用的 `filehub` 命令。

## Scope

### In scope

- 新增 `install-cli.sh`，支持 Linux 与 macOS；可选版本参数接受 `0.1.0` 或
  `v0.1.0`，省略时查询 GitHub `releases/latest`，读取并严格校验其 `tag_name`；默认
  安装到 `/usr/local/bin/filehub`，非 root 用户需要时通过 `sudo` 执行最终写入；
- 新增 `install-cli.ps1`，支持 Windows；可选 `-Version` 接受带或不带 `v` 的版本，
  省略时通过同一 GitHub API 获取并校验最新正式 Release tag；默认安装到
  `%ProgramFiles%\filehub\bin\filehub.exe`，要求管理员权限，并在该目录尚未存在时
  幂等加入机器级 `Path`；
- 两个脚本都严格校验版本格式、识别当前平台/架构、拼装固定仓库的 Release URL、
  解析最新版本或指定版本、下载到随机临时目录、只从归档中取出预期的单个二进制、
  完成原子式替换，并在成功或失败时清理临时文件；
- 与当前 Release 产物矩阵保持一致：Linux 仅 x86_64、macOS 仅 aarch64、Windows
  仅 x86_64；其余组合明确报错，不静默下载不兼容二进制；
- 两个脚本都允许显式覆盖安装目录，便于无管理员权限的测试或受管环境使用；显式
  自定义目录时不自动修改系统 `Path`；
- 更新根 `README.md` 的 GitHub Release CLI 安装章节，把脚本安装作为首选方式：分别
  给出 Linux/macOS 与 Windows 的“安装最新正式版”和“安装指定版本”可复制命令，
  说明默认目录、sudo/管理员权限、自定义目录、支持矩阵和卸载方式；保留现有手工下载
  归档说明作为脚本不可用时的备用方式；
- 新增定向契约测试，以本地假 Release/归档验证版本归一化、平台映射、URL、解包、
  安装目录、重复安装、失败清理和不支持架构拒绝；同时做 Shell 语法、PowerShell
  解析（环境可用时）与 diff 检查。

### Out of scope

- 不新增或修改 GitHub Release 资产，不增加 Linux aarch64、macOS x86_64 或 Windows
  arm64 构建；
- 不修改 `.github/workflows/build.yml`、CLI 产品行为、服务端、管理后台或 Cargo 依赖；
- 不把预发布版或草稿 Release 视为省略参数时的“最新版本”；最新版本语义遵循 GitHub
  `releases/latest` 返回的最新正式 Release；
- 不绕过操作系统权限模型，不把二进制写入 `C:\Windows\System32`，也不修改 Unix
  用户的 shell profile；
- 不直接执行真实系统目录安装或真实 GitHub 发布写入作为本地测试；
- 不触碰工作树中已有的 `Cargo.lock`、`harness/scripts/edit-guard.py`、
  `filehub-server.yaml`、`filehub.db` 等无关改动。

### Boundary with neighboring modules

本任务只新增 CLI 分发安装入口和对应文档/契约测试。安装脚本消费现有 GitHub
Release 契约，但不改变 Release 生产链路、CLI 命令面或 v1 API 契约。

## Requirement Review

需求合理，但跨平台“一个脚本”不适合同时覆盖 POSIX Shell 与 Windows PowerShell。
采用 `install-cli.sh` 和 `install-cli.ps1` 两个原生入口，可以减少运行时依赖并保持
各平台权限和 `Path` 操作清晰。

系统范围安装不应直接写入 Windows `System32`；Windows 使用标准的
`%ProgramFiles%\filehub\bin` 并幂等注册机器级 `Path`，Unix 使用已有惯例
`/usr/local/bin`。为便于 CI 和受限环境，显式安装目录可绕过默认系统目录，但该模式
不会隐式修改 `Path`。

当前 Release 没有发布 checksum，脚本无法对发布方签名或摘要做离线验证。选择固定
官方仓库、严格版本格式、HTTPS、精确资产名和安全解包来缩小风险；生成并校验签名或
checksum 需要另一个会改变发布资产契约的任务，不在本次范围内。

省略版本会额外依赖 GitHub 公共 REST API；API 失败、限流、返回 404 或缺少合法
`tag_name` 时安装必须失败，不能回退到猜测版本。GitHub 将该端点定义为最新的非
prerelease、非 draft Release，符合这里“最新正式版”的语义。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-install-cli-from-github | 提供 POSIX Shell 与 PowerShell 安装脚本；版本可选，省略时安装 GitHub 最新正式 Release，指定时安装对应版本；按当前受支持平台下载 CLI 并安全安装到系统级默认目录或显式自定义目录 | 仅消费 `wugren/sfo-filehub` 现有三平台 Release 归档与公开 `releases/latest` 元数据；默认 Unix `/usr/local/bin`，默认 Windows `%ProgramFiles%\filehub\bin`；Windows 默认安装幂等维护机器级 `Path` | 省略版本时额外依赖 GitHub API 可用性；系统级安装需要 sudo/管理员权限；当前无独立发布方 checksum；不支持 Release 尚未生产的 CPU 架构 | 本地假 Release/API 契约测试覆盖 latest 解析、指定版本、URL/平台/安装/覆盖/失败边界；Shell 语法与可用时 PowerShell 解析通过；README 与行为一致 | 不增加发布资产或架构，不把预发布/草稿视为 latest，不修改 CLI/服务端/API/CI 发布流程，不在测试中写真实系统目录 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - Linux x86_64 和 macOS aarch64 用户可用 `install-cli.sh` 安装最新正式版，或用
    `install-cli.sh <version>` 安装指定版本；Windows x86_64 用户可省略或通过
    `-Version <version>` 指定版本安装 `filehub.exe`；
  - 带 `v` 与不带 `v` 的同一版本解析为同一 tag/资产；省略版本时使用 GitHub
    `releases/latest` 的 `tag_name`；API/HTTP 失败、最新 tag 缺失或非法、显式版本非法、
    非预期归档、权限不足和不支持架构均失败且不给出伪成功；
  - 重复安装同一或另一指定版本不会重复污染 Windows 机器级 `Path`，临时目录始终清理；
  - 根 README 将脚本安装作为 GitHub Release CLI 的首选安装方式，提供 Linux/macOS
    与 Windows 的最新版本、指定版本可复制命令，并说明默认路径、权限、自定义目录、
    支持矩阵和卸载；手工下载归档方式继续作为备用入口。
- Required evidence:
  - 新增安装脚本契约测试通过，覆盖最新版本解析、指定版本以及主要失败/清理路径；
  - README 契约检查确认两个脚本入口、最新/指定版本示例、默认目录、权限、支持矩阵、
    自定义目录和卸载说明均存在，且手工下载备用方式仍与当前 Release 资产名一致；
  - `bash -n install-cli.sh` 通过，PowerShell 可用时脚本解析检查通过；
  - 测试只使用临时目录和本地假 Release，不写真实 `/usr/local/bin`、Program Files、
    机器级 `Path`，也不依赖实际 GitHub 网络状态；
  - 独立验收反查版本注入、路径穿越、错误架构、半安装、重复 `Path` 和临时文件泄漏。
- Explicit non-goals:
  - 不声明发布附件具有当前不存在的签名/checksum 保证；不真实执行系统安装或 GitHub
    Release 发布；不扩展现有 Release 平台架构矩阵。

## Risks

- 下载后安装的可执行文件具有供应链风险；当前只能依赖固定 GitHub 仓库的 HTTPS
  Release 下载，不能替代发布方 checksum 或代码签名。
- 默认目录写入需要提权。脚本必须把提权限制在最终目录创建/替换，临时下载与解包不以
  root/管理员身份运行；Windows 机器级 `Path` 只能添加精确安装目录且必须幂等。
- 归档来自外部输入。脚本不能将整个归档直接解压到系统目录，必须验证并只提取预期
  根级文件 `filehub` 或 `filehub.exe`。
- 本地测试可以证明脚本逻辑和模拟安装，但无法证明未来 GitHub Release 资产存在、网络
  可用或所有目标主机权限策略兼容；真实安装仍需在各目标平台受控验证。
- 省略版本时受 GitHub 公共 API 可用性和匿名限流影响；解析失败必须保持系统目录与
  `Path` 不变，并给出可通过显式版本重试的错误提示。
