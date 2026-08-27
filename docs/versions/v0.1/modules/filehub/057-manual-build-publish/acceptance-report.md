# 057 手工构建发布与 cargo update 验收报告

Risk profile: ./risk-profile.yaml

## Findings
| ID | Severity | Owning Stage | Correctness Category | Evidence | Problem | Blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-057-0 | none | none | none | `.github/workflows/build.yml:22-24,47-129,153-202,274-518`；`actionlint v1.7.12` 无输出通过；任务运行证据 `20260827T103822Z-filehub+057-manual-build-publish-all.json` | 第二轮独立反证未发现剩余缺陷；真实 GHCR/GitHub Release 写入按提案保留给合并后的 hosted 手工运行验证 | no |

## Object and Scope
- Task manifest: task.yaml
- Review date: 2026-08-27
- In-scope implementation: `.github/workflows/build.yml`、`tests/github_actions_build_contract.py`、`testplan.yaml` 与任务运行证据
- Review mode: independent falsification；忽略实现阶段自评，从重复发布取消、共享 lock 漂移、错误输入、tag 移动、权限与测试漏检重新审查

## Requirement Coverage
| change_id | Requirement or Boundary | Source | Implementation Evidence | Finding | Status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-manual-build-publish | 默认人工只构建；显式 publish=true 且 tag/version/repository/SHA 合法时才发布 GHCR 与 Release | `proposal.md` P-001 | `build.yml:7-24` 定义安全默认输入和不可取消串行组；`build.yml:274-317` 集中授权；`build.yml:391-518` 的两个写入点只消费授权输出并复检 tag/SHA | 普通人工、人工发布和既有 tag push 路径均满足门控；四件资产名称保持不变 | pass |
| fh-ci-cargo-update | Rust 编译前集中 cargo update，一次运行三平台共享同一 Cargo.lock | `proposal.md` P-002 | `build.yml:47-129` 安装 Rust、唯一执行 cargo update 并上传 lock；`build.yml:153-202` 三平台先下载同一 artifact，再以 `--locked` test/build | 更新顺序、失败关闭和跨平台单次运行一致性已机械约束 | pass |

## Independent Defect Discovery
| Category | Applicable Scope | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|------------------|--------------------|-------------------|----------------------------------|--------|
| requirement-and-behavior | 两个 change_id、既有 tag 发布和明确非目标 | `proposal.md` P-001/P-002、完整 workflow diff、14 项契约测试 | 逐项寻找缺失入口、默认误发布、tag 自动发布退化和仓库 Cargo.lock 回写 | 默认人工只构建，显式人工发布和 tag 发布均存在；临时 lock 只上传 artifact，没有回写步骤 | pass |
| logic-and-control-flow | version、authorize、build-image、release 条件与 needs 图 | `build.yml:35-129,274-322,391-518` 与 DAG 契约测试 | 枚举 dispatch publish false/true、tag push、fork、错误 tag 和 skipped job 传播 | 显式错误均非零失败；build-only 仍构建镜像但不 push，release 只在授权输出为 true 时运行 | pass |
| boundary-and-input | boolean 输入、空/畸形/错误版本 tag、canonical repository、annotated/lightweight tag | 两段 tag 正则/版本/SHA shell 与 unit/DV 测试 | 注入空 tag、shell 元字符、错误版本、非 canonical repo 和 tag 指向不同 commit | 正则和引用阻断注入；显式发布缺少任一条件均失败关闭，annotated tag 通过 `^{commit}` 解析 | pass |
| state-and-data-integrity | Cargo.lock artifact、授权 outputs、GHCR 与 Release 外部状态 | `build.yml:61-64,123-202,297-317,391-518` | 假设平台 cargo 尝试重解析，或同 tag 新 run 在首次外部写入后启动 | `--locked` 禁止平台修改解析；统一 concurrency group 且 cancel=false 使重复 run 排队，不会中断前一发布 | pass |
| error-handling-and-recovery | update/upload/download/tag fetch/build/push/release 失败 | `set -euo pipefail`、artifact error 策略、四资产计数、tag 三次校验 | 在每个依赖或外部写边界假设非零退出，并检查后续 job 是否还能发布 | 依赖失败通过 needs 阻断发布；写入前 tag 漂移失败；重复发布不会通过 concurrency 取消制造部分结果 | pass |
| resource-lifetime-and-cleanup | Cargo.lock artifact、Docker smoke 容器和 hosted workspace | 14 天 retention、smoke trap、job 临时目录 | 检查成功/失败时容器与临时依赖状态是否泄漏 | smoke 容器通过 EXIT trap 删除，artifact 明确 retention；任务未引入永久进程或本地产品状态 | pass |
| concurrency-and-ordering | 同 tag 手工/自动发布、同 ref 重复构建、cargo update 到矩阵构建 | `build.yml:22-24` concurrency 与 version/build needs；并发和顺序契约测试 | 在第一 run 已推送 GHCR、尚未创建 Release 时启动相同 tag 的第二 run | 相同 release_tag/ref_name 映射到同组且 cancel=false，第二 run 排队；全部 Rust job 依赖唯一 update job | pass |
| interface-and-compatibility | workflow_dispatch 新输入、push v*、job outputs、四资产名称 | workflow event/outputs/needs 与 integration 资产断言 | 省略新 inputs、运行既有 tag push、检查下游是否读取未经授权的原始值 | 新输入有兼容默认值；写入 job 只读取 authorize 输出；四件 tar.gz 契约未改变 | pass |
| security-and-capacity | GITHUB_TOKEN 权限、tag 输入、canonical gate、artifact 数量 | job permissions、quoted expansions、regex、artifact retention | 从 fork 请求发布、输入 shell 元字符、移动 tag、重复 run 和缺失 artifact | fork 发布失败，写权限仅在 GHCR/Release job；tag 在授权及每次写入前校验；额外 lock artifact 体积有限 | pass |
| test-adequacy | Rust 编译闭包、YAML/action shell、并发/lock/发布契约 | testplan.yaml、actionlint、`20260827T103822Z...json`、14 项 Python 测试 | 确认首轮三个缺陷分别能被现有断言捕获，并验证 Bash 脚本语法 | 测试现已断言 cancel=false、统一 concurrency、全部 cargo 命令 `--locked`，并对每个 run 脚本执行 `bash -n` | pass |

## Document Consistency
| Document | Source | Implementation Consistency | Finding | Status |
|----------|--------|----------------------------|---------|--------|
| design | `pipeline/plan.md` | 实现符合共享 lock 状态所有权、同 tag/ref 串行不可取消、集中授权与失败流设计 | 首轮返回后的设计与最终 workflow 一致 | pass |
| testing | `testplan.yaml` | 统一入口执行 Rust 编译闭包及 unit/DV/integration 契约测试，覆盖两个 change_id | 最新成功 artifact 与当前测试和实现匹配 | pass |

## Result Summary
- Overall result: accepted
- Outcome: 人工构建打包与受控发布入口、编译前 cargo update、共享 locked 构建以及不可取消发布并发均已实现并通过本地自动验收。
- Blocking issues: none；首轮 F-057-1、F-057-2、F-057-3 已通过设计、实现和测试返修关闭。
- Next action: 合并后从 GitHub Actions 手工运行默认 build-only；需要真实发布时选择目标 tag 对应源码并显式设置 publish=true 与 release_tag。

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 两个 change_id 的需求、失败边界和兼容路径均有对应实现及测试证据，首轮阻塞缺陷已被回归断言覆盖，第二轮反证未发现剩余阻塞问题。
