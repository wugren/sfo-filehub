---
task_manifest: task.yaml
status: approved
---

# GitHub Actions：workflow_dispatch 手动触发时也生成并上传 CLI 归档

Risk profile: not-created（trivial 层级不创建 risk-profile）

## Approval Record

- approver: user
- approval_date: 2026-08-23
- user_statement: 用户 2026-08-23 回复「改掉吧」，确认采纳修复方向
  （手动触发同样打包上传 CLI 归档）并采纳 trivial 层级。

## Workflow Tier Judgment

- Proposed tier: trivial
- Final tier: trivial
- Tier rationale / triggered boundaries:
  - 明确的 CI 行为修正，根因已在上一轮用托管 run 32585239301 的步骤结论
    与 artifact API 数据定位（CLI 编译成功，但打包/上传步骤因
    `startsWith(github.ref, 'refs/tags/v')` 条件在 workflow_dispatch 下
    skipped）；
  - 影响局限于 `.github/workflows/build.yml` 两处步骤条件与一行头部注释，
    不改产物内容、打包命令、镜像/Release 发布门控、公共 API、数据 schema、
    安全边界、依赖或运行时代码，属于配置级修复；
  - 可定向验证：YAML 解析通过 + 条件真值表核对（v 标签 push 与
    workflow_dispatch 均打包上传）。

## Background and Goal

- 现象：workflow_dispatch 手动运行（run 32585239301）整体 success，但
  Actions 页面没有 `cli-*` artifact，只有 `filehub-server` 与 `web-dist`。
- 根因（已用托管运行证据确认）：
  - 三个平台的 `Build filehub-cli (release)` 均 success；
  - 但 `Package CLI archive` 与 `Store CLI archive` 两步骤的
    `if` 为 `startsWith(github.ref, 'refs/tags/v')`，手动触发时
    `github.ref` 为 `refs/heads/main`，两步骤被 skipped；
  - 因此 CLI 只在 v 标签推送时打包上传，手动运行不产出 CLI artifact。
- 目标：workflow_dispatch 手动触发也生成并上传三平台 CLI 归档；v 标签
  推送路径行为保持不变；Release 发布门控（publish 仅 v 标签 canonical
  仓库）不变。

## Scope

### In scope

- 修改 `.github/workflows/build.yml`：
  - `Package CLI archive` 的 `if` 扩展为
    `startsWith(github.ref, 'refs/tags/v') || github.event_name == 'workflow_dispatch'`；
  - `Store CLI archive` 的 `if` 同样扩展；
  - 工作流头部注释中 workflow_dispatch 行补充「+ CLI 三平台归档」，
    与新的触发行为一致。
- 保留 `archive: false`、打包命令、产物命名、下载端
  `pattern: filehub-cli_*.tar.gz`（该模式匹配 archive:false 下按文件
  basename 命名的实际产物，已在 021 验证）与镜像/Release 逻辑不变。

### Out of scope

- 不改 Release 发布门控（v 标签 + canonical 仓库才 publish）；手动触发
  仍只构建、不推送、不发布；
- 不改三平台矩阵、构建命令、CLI 产物内容与命名；
- 不改 server binary / web-dist 上传逻辑与 release 下载逻辑。

### Boundary with neighboring modules

- 归属 022/023 引入的唯一构建/发布工作流；本任务只调整同一工作流内部
  CLI 打包/上传的触发条件，不影响 server/admin-web/cli 源码、镜像内容
  与发布面。

## Requirement Review

- 需求合理：CLI 已编译却因条件跳过后处理，导致手动验证拿不到 CLI 产物；
  把 workflow_dispatch 加入打包/上传条件是最小、低风险修正。
- 权衡：当前工作流只有 workflow_dispatch 与 v 标签 push 两个触发源，
  显式保留 tag 判断可避免未来新增触发源时默认打包；两步骤条件保持一致，
  避免「打包了却没上传」或「上传了却没文件」的中间状态。
- 注意：本地只能静态验证（YAML 解析 + 条件真值表 + 与 021 的产物名映射
  对照）；托管 runner 的手动运行记录需在下次 workflow_dispatch 中确认
  （与 020-023 相同证据边界）。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-cli-artifact-manual-trigger | workflow_dispatch 时三平台同样打包并上传 CLI 归档；v 标签路径不变 | 仅改 build.yml 两处 CLI 步骤 if 与头部注释 | 显式追加 workflow_dispatch 判断，保留 tag 门控语义，读起来明确未来新增触发源不会默认打包 | YAML 可解析；条件真值表两行均命中；发布门控与下载 pattern 未变 | 不改 Release 门控、产物内容与下载端匹配 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - workflow_dispatch 手动运行完成后 Actions artifact 列表出现
    `filehub-cli_<version>_linux-x86_64.tar.gz`、
    `macos-aarch64`、`windows-x86_64` 三个 CLI 归档；
  - v 标签推送的既有行为（构建 + 打包上传 + Release 发布）不受影响。
- Required evidence:
  - python YAML 解析通过；
  - 条件真值表核对（tag push / workflow_dispatch 均为打包上传）；
  - 托管运行结果显示（下一次 workflow_dispatch 的 artifact 列表，由后续
    运行或用户确认，本地无法真实触发 GitHub Actions runner）。
- Explicit non-goals:
  - 不改变 Release 发布语义，不改变 CLI 二进制与归档内容。

## Risks

- 本地无法执行托管 runner；GitHub 若后续调整 event/ref 语义需以托管日志
  复检。
- 手动触发会产生 3 个 CLI 归档 artifact（保留 14 天），占用普通 run 的
  artifact 配额，规模很小，不构成阻塞。
