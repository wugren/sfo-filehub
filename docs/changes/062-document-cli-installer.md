# 根 README 在线安装 CLI

- Status: complete
- Owner module: filehub
- Task manifest: `docs/versions/v0.1/modules/filehub/062-document-cli-installer/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/062-document-cli-installer/proposal.md`
- Affected paths: `README.md`, `tests/install_cli_scripts_contract.py`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

把根 README 中先下载再运行的两步示例改为单行在线执行：POSIX 使用 GitHub raw URL
管道到 `sh -s --`，并由 `bash -o pipefail -c` 保证下载失败不会被空脚本成功退出掩盖；
PowerShell 把 `Invoke-RestMethod` 返回内容转换为 `ScriptBlock` 后执行。最新版、指定版本
和自定义目录均保持单行；同步契约断言，并用本地标准输入模拟验证 Shell 参数传递，不
访问真实 GitHub 或系统目录。

## Risk Screen

- Public contract, protocol, or CLI change: yes；公开 CLI 安装入口从本地脚本执行改为在线单行执行，但不改变 CLI 命令或安装脚本参数。
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: yes；GitHub `main` 分支脚本会被立即执行，已在批准提案中明确审阅提示和手工归档备用方式。
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no；脚本来源、Release 资产和安装行为不变。
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

上述 `yes` 是已在批准提案中评估的文档调用与信任提示变化，不新增运行时行为或发布
能力，维持用户确认的 `standard` 层级。

## Verification

- Targeted check: `python3 tests/install_cli_scripts_contract.py --suite unit`（7 项）、`--suite dv`（6 项）、`--suite integration`（4 项，其中 PowerShell 解析因无 `pwsh` 跳过 1 项）；`bash -n install-cli.sh`；`bash -o pipefail -c 'sh -c "exit 22" | sh'; test $? -eq 22`；任务范围 `git diff --check`
- Result: pass
- Residual risk or follow-up: 本地验证不执行真实 GitHub 下载、Windows 系统安装或机器级 `Path` 修改；PowerShell 在线调用只能静态复核，真实目标平台验证仍是后续发布前检查项。
