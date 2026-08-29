# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/062-document-cli-installer.md

## Delivery Summary
- Outcome: 根 `README.md` 的 Linux/macOS 和 Windows 安装示例现在都在一条命令中
  从 GitHub raw 地址获取安装脚本并立即执行；最新版、指定版本和自定义安装目录均有
  可复制命令，不再要求先保存脚本文件。POSIX 示例使用 `pipefail` 传播下载失败，文档
  同时提示 `main` 分支远程执行风险并保留手工 Release 归档备用方式。
- Handoff: 修改集中在 `README.md` 与 `tests/install_cli_scripts_contract.py`；Shell
  标准输入模拟覆盖 latest、指定版本和自定义目录，unit/dv/integration 定向套件通过。
  当前环境没有 `pwsh`，PowerShell 解析用例跳过，未执行真实 GitHub 或系统目录安装。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-online-cli-installer-docs | 根 README 通过 GitHub 脚本地址提供单行在线安装命令，覆盖 Linux/macOS、Windows、最新版、指定版本、自定义目录及远程执行提示；不修改安装脚本和发布行为 | proposal.md P-001（Scope、Proposal Items、Success Criteria） | `README.md` 的 GitHub Release CLI 章节；`test_stdin_execution_accepts_latest_explicit_version_and_custom_dir`；README integration 契约断言 | 交付覆盖批准要求，且未修改两个安装脚本 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 逐行核对 `README.md` 四组 POSIX/PowerShell 命令与 `install-cli.sh` 参数循环、`install-cli.ps1` `param` 块 | 分别代入省略版本、`0.1.0`、仅自定义目录、版本加自定义目录，检查参数是否传给解释器后的安装脚本而非下载工具 | POSIX 使用 `sh -s --`，PowerShell 在 ScriptBlock 调用末尾传命名参数；四种调用与现有脚本接口一致 | pass |
| boundaries-and-failure-paths | 检查 POSIX 管道退出语义、固定 raw URL、远程执行说明、手工归档备用入口和 PowerShell 无本地文件调用 | 反查 `curl` 下载失败、空脚本、错误参数归属、用户不愿立即执行 `main` 内容等失败/信任边界 | 初查发现裸 curl-to-sh 管道可能掩盖下载失败，已改为 `bash -o pipefail -c` 并验证上游退出码 22 可传播；审阅链接和手工归档入口仍存在 | pass |
| regression-and-side-effects | 对照任务开始基线、根 README 原有默认目录/卸载/支持矩阵、两个安装脚本以及全部安装契约测试 | 搜索旧的 `Invoke-WebRequest -OutFile`、`chmod +x install-cli.sh` 和 `./install-cli.sh` 文档调用；核对是否误改脚本、CLI、Release 或无关工作树文件 | 旧两步调用已从根 README 移除；安装脚本未被本任务修改；7 unit、6 dv、4 integration（1 skip）通过；无任务范围外新增交付改动 | pass |

## Verification
- Targeted check: `python3 tests/install_cli_scripts_contract.py --suite unit`；`--suite dv`；`--suite integration`；`bash -n install-cli.sh`；POSIX `pipefail` 上游失败传播检查；任务范围 `git diff --check`
- Result: pass
- Exception reason: 当前 Linux 环境未安装 `pwsh`，因此未执行 PowerShell 解析和真实 Windows 安装；README 契约与脚本静态接口已复核。

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 初版 POSIX 在线命令的管道退出状态反查 | 裸 curl-to-sh 管道在下载失败时可能因空 `sh` 成功而返回 0；实施中已使用 `bash -o pipefail -c` 修复并补充契约断言 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 根 README 已按确认要求提供 GitHub 脚本地址的一行在线下载安装命令，所有
  参数形态与现有脚本一致；独立反查发现的管道失败传播问题已修复，三类缺陷发现均
  通过，无阻塞发现，定向验证通过。
