# 061 GitHub CLI 安装脚本验收报告

Risk profile: ./risk-profile.yaml

## Findings
| ID | Severity | Owning Stage | Correctness Category | Evidence | Problem | Blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-061-0 | none | none | overall | 最终两个脚本、README、测试源码、最新任务 artifact 与第一轮返修路径 | 第二轮反证未发现剩余实现或测试缺陷；第一轮 F-061-1 至 F-061-4 已修复并由 runtime return record 保留历史 | no |

## Object and Scope
- Task manifest: task.yaml
- Review date: 2026-08-28
- In-scope implementation: `install-cli.sh`、`install-cli.ps1`、`README.md`、`tests/install_cli_scripts_contract.py`、`testplan.yaml` 与最新任务运行证据
- Review mode: independent falsification；第二轮从第一轮缺陷、链接归档、回滚失败、信号中断、latest 404、平台/资产契约和测试漏检重新审查后再选择结论

## Requirement Coverage
| change_id | Requirement or Boundary | Source | Implementation Evidence | Finding | Status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-install-cli-from-github | 版本可缺省并解析 GitHub 最新正式 Release，也可指定带/不带 v 的版本；按现有 Linux x86_64/macOS aarch64/Windows x86_64 资产安全安装；默认系统目录、有限提权、Windows Path 幂等、自定义目录、失败回滚/清理及 README 安装说明 | `proposal.md` P-001 | `install-cli.sh:35-203`、`install-cli.ps1:18-221`、`README.md:43-113`、`tests/install_cli_scripts_contract.py` 与 `20260828T042106Z-filehub+061-install-cli-from-github-all.json` | 批准行为均有实现和定向证据；真实 Release 当前不存在及 Windows pwsh/系统安装未在本机执行，均按提案作为明确外部/目标平台边界保留 | pass |

## Independent Defect Discovery
| Category | Applicable Scope | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|------------------|--------------------|-------------------|----------------------------------|--------|
| requirement-and-behavior | P-001 全部行为、非目标与新增 README 要求 | `proposal.md:31-139`、最终两脚本、README、testplan 与最新 artifact | 逐项反查缺省/指定版本、支持矩阵、系统/自定义目录、权限、Path、归档、替换、清理、卸载和手工备用入口 | 全部批准项已实现；不创建 Release/checksum/新架构且不宣称当前不存在的真实安装成功 | pass |
| logic-and-control-flow | 参数/latest/平台分支、Unix direct/sudo、Windows replace/Path/rollback、信号 | `install-cli.sh:35-203`、`install-cli.ps1:18-221`、7 unit 与 5 DV | 枚举带/不带 v、非法/重复参数、非法 API tag、OS/arch、已有/新安装、Path 已存在、信号与回滚失败 | 分支失败关闭且无猜测版本；第一轮信号/回滚控制流缺陷已关闭 | pass |
| boundary-and-input | 外部版本/API JSON、OS/arch、安装路径、tar 名称/类型/大小 | 版本正则、固定 URL、两次 tar 类型/内容验证、symlink 实跑反例 | 输入空/畸形/多版本、额外条目、唯一同名 symlink、非法 latest tag、不支持架构 | 非法输入均在系统写入前失败；symlink 外部目标内容和权限保持不变 | pass |
| state-and-data-integrity | 既有 CLI、同目录 staging/backup、Windows machine Path | Unix mktemp/install/mv；Windows File.Replace/backup/rollback/Path 规范化 | 重复版本安装、替换失败、Path 更新失败、回滚再次失败及 Path 重复项 | 正常替换为同目录原子动作；失败尽量恢复旧版本，自动恢复失败时保留备份；默认 Path 仅缺失时添加，自定义目录不修改 | pass |
| error-handling-and-recovery | API/下载/tar/权限/写入/Path/回滚错误 | `fail`、try/catch/finally、DV API/download failure 与 unit recovery tokens | 在每个边界返回非零并检查旧文件、staging、backup 与错误提示 | API/下载/归档错误保留旧安装并清理；Windows 二次回滚失败不再删除恢复材料 | pass |
| resource-lifetime-and-cleanup | Unix/Windows 临时目录、同目录 staging、旧版本 backup、信号 | EXIT/signal trap、PowerShell finally/PreserveBackup、清理断言 | 成功、普通失败、恶意归档、INT/TERM 设计和回滚失败资源状态 | 普通路径清理临时/staging/backup；信号明确 exit；仅回滚失败时有意保留 backup 并警告 | pass |
| concurrency-and-ordering | 单安装进程中的下载→校验→stage→replace→Path 顺序 | pipeline plan 状态所有权、两个脚本顺序 | 搜索后台任务、共享锁、异步写和跨线程状态 | 未引入后台/异步工作；同目录替换保证单进程顺序。多安装器并发互斥不在批准要求中，且不是当前行为退化 | not-applicable |
| interface-and-compatibility | 两个新脚本接口、现有 Release 生产者、README 用户入口 | `.github/workflows/build.yml:132-230,459-509`、脚本参数、README、integration tests | 对照三资产名、带/不带 v、最新/指定、自定义路径、手工 fallback 与当前 API 404 | 新接口与生产者命名一致且不移除旧手工路径；当前无 Release 被如实记录，不伪装成功 | pass |
| security-and-capacity | HTTPS/固定仓库、版本注入、链接/穿越、提权、机器 Path、临时空间 | curl protocol 限制、正则、tar regular type、sudo 边界、Windows admin/Path/rollback | 注入 URL 字符、唯一 symlink/hardlink/reparse、非管理员默认安装、恶意归档条目 | URL 不接受非语义版本；非普通文件在解包/安装前拒绝；提权限于最终 Unix 写入，Windows 默认安装需管理员，自定义目录不改机器 Path | pass |
| test-adequacy | unit/DV/integration 和目标平台缺口 | 测试源码、testplan、最新 artifact、PowerShell 条件 skip | 确认第一轮四项发现都会触发新断言；检查 macOS/Windows/真实 Release 未证实范围 | 原缺陷均有回归；15 项通过。当前无 pwsh，PowerShell 解析/真实 Windows 系统安装未执行，但提案明确“环境可用时”，静态分支契约和安全反例映射已覆盖，作为非阻塞目标平台验证缺口记录 | pass |

## Document Consistency
| Document | Source | Implementation Consistency | Finding | Status |
|----------|--------|----------------------------|---------|--------|
| design | `pipeline/plan.md` | 最终实现符合两个原生入口、单一临时/安装状态所有者、普通文件校验、失败流、同目录替换、Path 规则和文件级顺序 | 第一轮返回没有改变批准接口；修复补足原设计安全/恢复不变量 | pass |
| testing | `testplan.yaml` | unit/DV/integration 均由任务入口执行，覆盖唯一 change_id 与七类 case type | 最新 artifact 绑定当前 testplan/实现；PowerShell 条件 skip 与 precondition note 一致 | pass |

## Result Summary
- Overall result: accepted
- Outcome: 两个安装脚本、最新版/指定版语义、三平台映射、系统/自定义安装、README 说明和第一轮安全返修均已完成并通过任务级验证。
- Blocking issues: none；F-061-1 至 F-061-4 已关闭。
- Next action: 合并并推送脚本后，首次正式 GitHub Release 发布会解除当前 API 404；届时在 Windows x86_64 管理员 PowerShell 和目标 macOS/Linux 主机各执行一次真实安装冒烟。

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 批准的 change_id 已有对应实现、失败边界和 15 项通过证据；第一轮发现的链接跟随、备份删除、信号和漏检问题均已修复并受回归约束。当前无真实 Release/pwsh 的外部验证缺口已明确，不掩盖本地未执行事实且不构成批准范围内的实现缺陷。
