---
task_manifest: task.yaml
status: approved
---

# GitHub Actions 单文件产物下载名对齐（archive:false 按文件 basename 匹配）

Risk profile: not-created（trivial 层级不创建 risk-profile）

## Approval Record

- approver: user
- approval_date: 2026-08-22
- user_statement: 确认（2026-08-22 用户回复「确认」，采纳 trivial 层级并按
  方案 2 执行：保留 archive:false，下载端按文件 basename 匹配）

## Workflow Tier Judgment

- Proposed tier: trivial
- Final tier: trivial
- Tier rationale / triggered boundaries:
  - 明确的 CI bugfix，根因与修复方向已由用户确认（方案 2：保留
    `archive: false`，下载端按文件 basename 匹配）；
  - 影响局限于 `.github/workflows/build.yml` 的产物名匹配，不改产物内容、
    工作流结构、构建命令、镜像/Release 发布面、公共 API、数据 schema、
    安全边界、依赖或运行时代码，属于配置级修复；
  - 可定向验证：YAML 解析通过 + 下载名与上传文件 basename 映射核对。
- Proposal and tier confirmation:
  - 用户于 2026-08-22 回复「按2方案修改」，确认保留单文件不打包
    （`archive: false`）并改为下载端按 basename 匹配；同日用户回复「确认」，
    采纳 trivial 层级，本提案置为 `status: approved`。

## Background and Goal

- 现象：master push 构建 run 32565108750 在 `Build and publish Docker image`
  的 Download server binary 步骤失败：
  `Unable to download artifact(s): Artifact not found for name: server-binary`。
- 根因（已定位）：
  - `upload-artifact@v7` 在 `archive: false`（单文件不打包）时忽略 `name`
    参数，产物名直接使用文件 basename：`target/release/filehub-server`
    实际上传名为 `filehub-server`，导致下载端按 `server-binary` 找不到；
  - CLI 三个单文件产物同样受该行为影响，实际名为
    `filehub-cli_<version>_<platform>.tar.gz`，release 下载的
    `pattern: cli-*` 同样匹配不到，会在 v 标签发布时复现同类失败。
- 目标：保留 `archive: false` 的「直接展示文件」行为（方案 2），下载端与
  真实产物名对齐，使镜像构建下载与 v 标签 Release 下载均能命中。

## Scope

### In scope

- 修改 `.github/workflows/build.yml` 下载端：
  - `build-image` 与 `release` 的 server 下载步骤 `name` 从
    `server-binary` 改为 `filehub-server`；
  - `release` 的 CLI 下载步骤 `pattern` 从 `cli-*` 改为
    `filehub-cli_*.tar.gz`；
  - 保留上传侧 `archive: false` 与产物内容不变（名称由文件 basename 决定）。

### Out of scope

- 不修改上传侧 `name` 值（在 `archive: false` 单文件上传下被忽略）；
- 不改为默认打包方案（方案 1 已排除）；
- 不修改镜像构建、Release 发布语义与命令，不改 `admin-web`/Rust 源码。

### Boundary with neighboring modules

- 归属 020-github-actions-build-release 引入的唯一构建/发布工作流；本任务只
  调整同一工作流内部的产物名匹配，不影响 server/admin-web/cli 源码与
  容器镜像内容。

## Requirement Review

- 需求合理：下载名与实际产物名不一致是确定性配置错误，uploads 与 downloads
  使用不同命名约定必然失败；对齐下载端是最小、低风险修复。
- 权衡：产物名与文件 basename 绑定（CLI 产物名含版本号），
  `filehub-cli_*.tar.gz` 模式可匹配三个平台；`merge-multiple: true` 下三个
  文件名互不相同，不会冲突。
- 注意：本地只能做静态核对（YAML 解析 + 名称映射）；托管 runner 的实际
  下载成功需在下次 push/tag 触发的工作流运行中确认。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-artifact-download-names | 下载端按 archive:false 产物真实名匹配：server 下载 `name=filehub-server`；CLI 下载 `pattern=filehub-cli_*.tar.gz` | 仅改 `.github/workflows/build.yml` 三个下载步骤 | 产物名跟随文件 basename（CLI 含版本号），保留直传文件行为 | YAML 可解析；下载名与上传路径 basename 对照一致；rg 无残留 `server-binary`/`cli-*` 下载引用 | 不改上传 name 与 archive 语义，不改镜像/Release 逻辑 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - master push 后 `build-image` 能下载 server 二进制并继续镜像构建；
  - v 标签发布时 `release` 能下载三个 CLI 归档并完成发布。
- Required evidence:
  - `python` YAML 解析通过；
  - 下载名与 `archive: false` 上传文件 basename 的映射核对（server：
    `filehub-server`；CLI：`filehub-cli_*.tar.gz`）；
  - 修复后托管运行结果（下一次 push/tag 触发，由后续运行或用户确认）。
- Explicit non-goals:
  - 不改为默认打包（方案 1），不修改上传产物或发布逻辑。

## Risks

- 本地无法执行托管 runner；若 GitHub 后续调整 artifact 命名/匹配行为，需以
  托管运行日志为准复检。
- CLI 产物名包含 workspace 版本；若版本命名约定变化，
  `filehub-cli_*.tar.gz` 模式可能需要同步调整（当前版本格式固定为
  X.Y.Z 三段数字）。
