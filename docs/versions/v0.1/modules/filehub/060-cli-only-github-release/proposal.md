---
task_manifest: task.yaml
status: approved
---

# GitHub Release 仅发布 CLI 文件

Risk profile: not-created (replace with ./risk-profile.yaml only after high-risk confirmation)

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: trivial
- Tier rationale / triggered boundaries:
  - 本任务直接改变正式 GitHub Release 的 produced artifact 与 release/deployment 交付面：移除 server+admin-web 压缩包，只保留三平台 CLI 压缩包；
  - server 的交付入口将明确收敛为 GHCR Docker 镜像，需要保证 server 构建、admin-web 构建、镜像组装、容器冒烟和 GHCR 发布链路不被误删；
  - 本地静态检查可以验证 workflow 契约，但真实 GitHub Release/GHCR 写入仍只能由后续受控 hosted Actions 运行证明，因此建议 `high-risk`。
- Proposal and tier confirmation: 2026-08-28 用户回复“确认，trivial”，批准本提案并明确选择 `trivial` 层级。该选择低于建议的 `high-risk`；执行将保留定向发布契约回归检查，并在完成报告中记录真实 GitHub Release/GHCR 写入无法本地验证的残余风险。

## Approval Record

- approver: 用户
- approval_date: 2026-08-28
- user_statement: “确认，trivial”
- selected_tier: trivial

## Background and Goal

当前 `.github/workflows/build.yml` 的正式发布会向 GitHub Release 上传一个包含
`filehub-server` 与 admin-web 的 Linux 压缩包，以及 Linux、macOS、Windows 三个
CLI 压缩包。server 实际通过 Docker 镜像交付，因此 GitHub Release 不再需要重复
提供 server 相关文件。

目标是让 GitHub Release 只发布三个 CLI 压缩包，同时保留 server/admin-web 作为
Docker 镜像构建输入，并继续通过现有容器冒烟与 GHCR 发布门禁交付 server。

## Scope

### In scope

- 从 `release` job 删除 server binary 与 admin-web dist 的下载步骤；
- 删除 server+admin-web GitHub Release 压缩包的组装步骤；
- 将 Release 资产校验收敛为 Linux、macOS、Windows 三个 CLI `.tar.gz`；
- 将 GitHub Release 上传数量门禁由四个改为三个，并从 Release notes 中移除 server 压缩包条目，保留 Docker 镜像地址；
- 同步根 `README.md` 的交付说明，删除已经失效的 server 手工归档下载与部署路径，明确 GitHub Release 只提供 CLI、server 通过 Docker 镜像交付；
- 更新 `tests/github_actions_build_contract.py`，明确验证 Release 只消费 CLI artifacts，且 Docker job 仍消费 server/admin-web artifacts；
- 通过 YAML 解析、所有 workflow Bash 脚本语法、定向工作流契约测试与 diff 检查验证。

### Out of scope

- 不移除 Linux build job 中的 server release 编译或 `filehub-server` 内部 artifact；
- 不移除 admin-web 测试、构建或 `web-dist` 内部 artifact；
- 不改变 Docker 镜像组装、容器冒烟、GHCR `vX.Y.Z`/`latest` 推送或其授权门禁；
- 不改变三平台 CLI 的构建、压缩格式、文件名或 GitHub Release 发布条件；
- 不修改产品源码、Dockerfile、版本规则、Cargo 依赖解析或 tag/SHA 校验；
- 不触碰工作树中已有的 `Cargo.lock`、`harness/scripts/edit-guard.py`、`filehub-server.yaml`、`filehub.db` 等无关改动。

### Boundary with neighboring modules

本任务只改变仓库级 GitHub Release 的对外文件集合、对应根 README 使用说明及静态
契约测试。server 与 admin-web 仍由同一 workflow 构建并进入 Docker 镜像；产品
API、CLI 行为和镜像内容均不变。

## Requirement Review

需求合理。server 已有独立且更贴近部署形态的 Docker 镜像交付入口，继续在
GitHub Release 发布 server+admin-web 压缩包会形成重复且可能让用户误选的交付方式。
选择只收窄 `release` job，而不收窄上游 build/test 或 `build-image` job，可以满足
“Release 只发布 CLI”并保持 Docker server 的构建、测试和发布完整性。

主要权衡是现有依赖 GitHub Release server 压缩包的消费者将无法从新 Release 获取
该文件；本需求明确指定 server 改由 Docker 镜像发布，因此不保留兼容附件。历史
Release 不在本任务中回写或删除。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-cli-only-github-release | 正式 GitHub Release 只上传 Linux、macOS、Windows 三个 CLI 压缩包；server 继续只通过既有 Docker 镜像发布；根 README 与该交付契约一致 | `.github/workflows/build.yml`、`README.md` 与对应静态契约测试 | 不再为新 Release 提供 server+admin-web 独立压缩包，现有附件消费者需改用镜像 | 契约测试确认 Release job 不下载/打包 server 或 web、只校验并上传 3 个 CLI 包；README 不再宣称存在 server Release 归档；Docker job 仍下载 server/web、完成镜像冒烟及受控 GHCR 发布；YAML/Bash/diff 检查通过 | 不改变镜像内容、CLI 包格式、历史 Release 或发布授权门禁 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - 后续由该 workflow 新建的正式 GitHub Release 只包含 `filehub-cli_<version>_linux-x86_64.tar.gz`、`filehub-cli_<version>_macos-aarch64.tar.gz`、`filehub-cli_<version>_windows-x86_64.tar.gz`；更新已有 Release 时只上传或覆盖这三个 CLI 包，但不主动清理历史附件；
  - Release notes 继续列出 `ghcr.io/{owner}/filehub:v{version}`，不再列出 server 压缩包；
  - 根 README 只把 GitHub Release 描述为 CLI 下载入口，并把 server 使用入口指向 Docker 镜像；
  - server/admin-web 仍被构建、测试并组装到通过冒烟检查后发布的 Docker 镜像中。
- Required evidence:
  - `tests/github_actions_build_contract.py` 的 unit、dv、integration 套件通过；
  - workflow YAML 可解析，所有 `run` 脚本通过 `bash -n`，`git diff --check` 通过；
  - 静态检查证明 GitHub Release 资产恰为三个 CLI 包、README 与新交付契约一致，且 Docker job 的 server/web artifact 消费链路保持不变；
  - 本地验证不宣称真实 GitHub Release 或 GHCR 发布成功，线上结果由后续受控 hosted Actions 运行确认。
- Explicit non-goals:
  - 不直接触发发布，不删除历史 Release 资产，不改变产品代码、Docker 镜像内容或 CLI 归档格式。

## Risks

- 新 Release 不再提供 server+admin-web 独立压缩包，这是有意的交付兼容性变化；已有消费者需切换到 Docker 镜像。
- 若误删上游 server/web artifact 或 Docker job 消费步骤，会破坏镜像发布；契约测试必须同时验证 Release 收窄与 Docker 链路保留。
- 本地无法完整复现 GitHub artifact 传递、GitHub Release API 与 GHCR 写入；通过静态契约和脚本检查降低风险，但真实发布仍需 hosted Actions 证据。
