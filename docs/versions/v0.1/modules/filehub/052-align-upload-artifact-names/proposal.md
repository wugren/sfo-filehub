---
task_manifest: task.yaml
status: approved
---

# GitHub Actions 上传 artifact 名称对齐：服务端 filehub-server、客户端 filehub-cli

Risk profile: not-created（待确认层级后再决定；standard/trivial 不创建）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 明确的 CI bugfix，根因与修复方向已由用户溯源并指定：上传端服务端 artifact
    必须是 `filehub-server`，客户端必须是 `filehub-cli`（三平台唯一命名采用
    `filehub-cli-{platform}`），影响局限在 `.github/workflows/build.yml`；
  - 与 021 方向相反：021 保留 `archive: false`（name 被忽略、artifact 名取文件
    basename）并让下载端贴 basename；本次建议移除 `archive: false` 使 `name`
    成为真实 artifact 名，并同步 release 下载匹配面。这直接改动「上传产物存储
    形态 + Release 下载匹配」这个发布面，不满足 trivial 对「无 produced
    artifact / release / deployment 实质影响」的要求；
  - 不触发 high-risk：无 schema/迁移、无安全/隐私边界、无依赖/构建图与供应链
    变更、无运行时/CLI/API 契约、无跨项目/架构边界；回滚等于撤销单一工作流
    改动，无需协调；
  - 与 020/023 这类 CI 构建/发布面任务的 standard 先例一致。
- Proposal and tier confirmation: 用户 2026-08-26 回复「确认」，确认采纳
  本提案推荐的方案 A（移除 `archive: false`、CLI 命名
  `filehub-cli-{platform}`、release 下载 `pattern: filehub-cli-*`）与
  standard 层级。

## Approval Record

- approver: 用户
- approval_date: 2026-08-26
- user_statement: 用户 2026-08-26 回复「确认」，采纳方案 A 与 standard 层级；
  两个未决问题按推荐项确认：CLI artifact 采用 `filehub-cli-{platform}`
  唯一命名；移除 `archive: false` 使 `name` 成为真实 artifact 名。

## Background and Goal

- 现象（用户评审项，标注高危）：`build.yml` 上传端与下载端的 artifact 名称
  完全对不上——server 上传 `name: server-binary`，但 `build-image` 与 `release`
  下载 `name: filehub-server`；CLI 上传 `name: cli-{platform}`，release 却用
  `pattern: filehub-cli_*.tar.gz` 去匹配 artifact 名。无论按静态声明还是按
  真实产物名口径，两者都不在同一命名约定内，image/release 流程的下载阶段
  存在确定性失败风险。
- 技术背景：pinned `upload-artifact@v7` 在 `archive: false`（单文件直传）时
  `name` 参数被忽略，实际 artifact 名取文件 basename；021 据此选择了「下载端
  贴 basename」的方向并已归档。用户现在明确要求反过来：服务端必须是
  `filehub-server`，客户端必须是 `filehub-cli`。
- 目标：让上传 artifact 的真实名称成为 `filehub-server` 与
  `filehub-cli-{platform}`，下载端（镜像 job 与 release job）按这些名称命中；
  Release 最终四件资产的文件名与发布逻辑保持不变。

## Scope

### In scope（推荐方案 A）

- `.github/workflows/build.yml` server 上传步骤：`name` 从 `server-binary`
  改为 `filehub-server`，移除 `archive: false` 使 `name` 权威；
- CLI 上传步骤：`name` 从 `cli-${{ matrix.artifact }}` 改为
  `filehub-cli-${{ matrix.artifact }}`，移除 `archive: false`；
- `release` CLI 下载步骤：`pattern` 从 `filehub-cli_*.tar.gz` 改为
  `filehub-cli-*`（保留 `merge-multiple: true`）；
- `build-image`/`release` 的 server 下载保持 `name: filehub-server`
  （已与目标名称一致，不改）。

### Out of scope

- 不修改 CLI 打包命令、Release 资产文件名（`filehub-cli_${VERSION}_${ARTIFACT}`
  `.tar.gz`）、`gh release` 上传与 notes；
- 不修改 `web-dist` 上传/下载、镜像构建与 GHCR 发布、触发策略、矩阵与 action
  版本；
- 不改写 021 已完成的提案/报告（历史基线保留；本次由用户新指令覆盖其方向）；
- 不触碰其他在制任务的未提交改动，不做仓库级格式化。

### Boundary with neighboring modules

- 全仓库仅 `.github/workflows/build.yml` 引用这些 artifact 名（`rg` 已确认）；
  server/CLI/web-dist 三组命名互不影响，改动不影响 server/admin-web/cli
  源码与最终 Release 资产内容。

## Requirement Review

- 需求合理：上传名与下载名必须使用同一命名约定，这是确定性的配置一致性
  要求；用户指定的 `filehub-server` / `filehub-cli` 与现有下载端语义一致，
  需要把上传端对齐过去。
- 关键权衡 1（CLI 唯一性）：`upload-artifact@v4+` 明文禁止多个 job 上传同名
  artifact（README「Uploading to the same artifact via multiple jobs is not
  supported with v4」），三个平台矩阵不能都叫 `filehub-cli`；最小唯一命名是
  `filehub-cli-{platform}`，前缀即用户要求的 `filehub-cli`。若要求聚合为
  单一的 `filehub-cli` artifact，需要新增聚合 job，属更大工作流改动，本提案
  不默认包含。
- 关键权衡 2（archive 语义）：当前 `archive: false` 下 `name` 被官方文档明确
  忽略，仅改 `name` 字段在托管 runner 上不产生行为变化；要让
  「artifact 名必须是 filehub-server / filehub-cli」真实成立，需要移除
  `archive: false`（方案 A）。单文件 artifact 下载后布局不变
  （`ctx/server/filehub-server`、`dist/raw/cli/*.tar.gz`），Release 资产名
  不变。
- 结论：推荐方案 A；若不接受移除 `archive: false`，则退化为方案 B（只改
  `name` 声明、保留官方 archive:false 语义、行为不变），需用户在确认时二选一。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-upload-artifact-names | 上传 artifact 名必须为 `filehub-server` 与 `filehub-cli-{platform}`，镜像/release 下载端按这些名称命中 | 仅 `.github/workflows/build.yml` 的 server/CLI 上传步骤与 release CLI 下载步骤 | CLI 因 v4+ 同名互斥采用 `filehub-cli-{platform}`；移除 `archive: false` 使 `name` 权威 | python YAML 解析通过；`rg` 无 `server-binary`/`cli-${{ matrix.artifact }}` 残留；上传名↔下载名↔后续文件引用映射核对一致 | 不聚合为单一 `filehub-cli` artifact；不改 Release 资产文件名、镜像逻辑与触发器 |

## Success Criteria

- 可见结果：人工触发（workflow_dispatch）时镜像 job 的 Download server binary
  按 `name: filehub-server` 命中并继续构建镜像；v 标签发布时 release job 的
  server 与 CLI 下载全部命中，产出并校验四件 Release 资产。
- 所需证据：YAML 解析通过；`rg` 名称扫描无旧名残留；上传名→下载名→后续
  `chmod`/`cp`/`verify` 文件引用映射核对；本地路径/命令模拟
  （`ctx/server/filehub-server`、`dist/raw/cli/*.tar.gz`、四件资产校验）。
- 显式非目标：不改变最终 Release 资产文件名与内容格式；不做聚合 artifact；
  不以本地验证替代托管 runner 的真实 artifact 行为确认。

## Risks

- 本地沙箱无法运行 GitHub artifact 服务，托管 runner 的最终命中结果需由
  下一次 `workflow_dispatch`/v 标签运行确认；若 GitHub 后续调整
  upload/download artifact 语义，以托管日志为准复检。
- 若用户期望「单一 artifact 名为 filehub-cli 且包含三平台内容」，需要新增
  聚合 job，超出当前提案范围，需另行确认后再执行。
- 方案 B 残余风险：`archive: false` 下 `name` 按官方文档无效，改动仅为声明层
  对齐，托管行为与现状相同；该取舍已在本提案 Requirement Review 中显式记录。

## Confirmed Decisions

1. CLI artifact 名称：采用 `filehub-cli-linux-x86_64` /
   `filehub-cli-macos-aarch64` / `filehub-cli-windows-x86_64`，release 下载
   `pattern: filehub-cli-*`（保留 `merge-multiple: true`）。
2. 移除 server 与 CLI 上传步骤的 `archive: false`（方案 A），使 `name`
   参数成为真实 artifact 名；单文件 artifact 下载后布局与 Release 资产名
   不变。
