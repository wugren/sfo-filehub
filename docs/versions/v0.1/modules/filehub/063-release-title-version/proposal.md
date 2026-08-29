---
task_manifest: task.yaml
status: approved
---

# GitHub Release 使用版本号标题并确保正式发布

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 改动只涉及单一 GitHub Actions 发布步骤及其契约测试，不改变 tag、版本校验、发布
    授权、构建产物、附件名称或 GHCR 镜像标签；
  - 但 Release 标题和 draft 状态属于公开发布面，不能满足 `trivial` 的“无发布影响”
    条件，因此建议使用 `standard`；
  - 改动不引入新的发布入口、权限、凭据、兼容性或回滚协调，也不直接执行线上发布，
    不构成需要完整分阶段流程的 `high-risk`。
- Proposal and tier confirmation: 2026-08-29 用户回复“确认”，批准本提案与
  `standard` 层级，并授权在不扩大范围的前提下完成实施、验证和验收。

## Approval Record

- approver: 用户
- approval_date: 2026-08-29
- user_statement: “确认”
- selected_tier: standard

## Background and Goal

当前 GitHub Release 创建分支使用 `--title "filehub ${VERSION}"`，已有 Release 分支只
执行 `gh release upload --clobber`，不会重新对齐标题或把已有 draft 转为正式发布。
目标是让 Release 的显示名称只使用 Cargo workspace 版本号，例如 `0.1.0`；tag 仍为
`v0.1.0`，并让首次创建与已有 Release 更新两条路径最终都处于非 draft 状态。

## Scope

### In scope

- 调整 `.github/workflows/build.yml` 的 Release 创建标题为精确的 `${VERSION}`，例如
  `0.1.0`，不添加 `filehub` 前缀，也不使用 `${RELEASE_TAG}`；
- 对已有 Release 的更新路径显式执行 `gh release edit`，将标题对齐为 `${VERSION}` 并
  设置 `--draft=false`，避免仅上传附件后继续保留旧标题或 draft 状态；
- 新建 Release 继续不使用 `--draft`，并通过契约测试锁定精确标题、非 draft 收敛以及
  create/update 两条路径；
- 运行 GitHub Actions 契约测试、workflow 内嵌 Shell 语法检查和差异检查。

### Out of scope

- 不改变发布 tag 格式或触发条件；tag 仍使用 `vMAJOR.MINOR.PATCH`；
- 不改变 workspace 版本来源、tag/SHA 校验、canonical repository 门禁或发布权限；
- 不改变三个 CLI Release 附件、Release notes、Docker 镜像、GHCR 版本标签或 `latest`；
- 不直接调用 GitHub API 修改当前线上 `v0.1.0` Release，也不触发 hosted Actions；
- 不清理已有 Release 中遗留的 server 附件；
- 不触碰工作树中已有的 `Cargo.lock`、`harness/scripts/edit-guard.py`、
  `filehub-server.yaml`、`filehub.db` 等无关改动。

### Boundary with neighboring modules

本任务只调整 filehub 的 GitHub Release 展示元数据收敛逻辑。CLI、server、admin-web、
安装脚本及其产品行为均不变化。

## Requirement Review

需求合理。Release 标题与 tag 承担不同职责：标题用于用户可见的版本展示，tag 用于
源码定位和发布触发。将标题固定为无 `v` 前缀的 workspace 版本号，可以避免把 tag
误当作显示名称；保留 tag 为 `v0.1.0` 则不破坏现有触发和校验契约。

只在创建分支修改 `--title` 不足以修复当前现象，因为已有 Release 会进入 upload 分支，
其标题和 draft 状态都不会改变。因此更新分支需要在附件上传成功后显式执行
`gh release edit --title "${VERSION}" --draft=false`。该操作仍受现有 publication gate、
tag/SHA 复核和 `contents: write` 权限约束，不新增发布通路。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-release-version-title | GitHub Release 的标题精确显示 workspace 版本号；首次创建和已有 Release 更新后均为非 draft | 只修改 workflow 发布步骤和对应契约测试；tag 仍为带 `v` 前缀的发布标识 | 已有 draft 在下一次授权发布运行时会被公开；不直接修改线上 Release | 契约测试确认 create 使用 `${VERSION}` 标题且无 draft，update 在上传成功后以 `${VERSION}` 标题和 `--draft=false` 收敛；Shell 语法与 diff 检查通过 | 不改发布门禁、tag、附件、notes、GHCR 或当前线上状态 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - 下一次授权发布运行创建或更新 Release 后，Release 标题显示为 `0.1.0` 形式；
  - Release 关联 tag 仍显示为 `v0.1.0` 形式；
  - 已有 draft Release 经授权更新路径后变为正式发布。
- Required evidence:
  - GitHub Actions 契约测试覆盖 `${VERSION}` 与 `${RELEASE_TAG}` 的标题边界；
  - 契约测试覆盖 create 和 upload/update 两条路径的非 draft 结果；
  - workflow 内嵌 Shell 脚本通过语法检查，`git diff --check` 通过。
- Explicit non-goals:
  - 本地验证不声称已经修改当前 GitHub Release，也不代替下一次 hosted 发布运行。

## Risks

- `--draft=false` 会在下一次已授权发布运行中公开现存 draft；这是修复前述 draft 问题的
  预期行为，但真实线上结果仍需 hosted Actions 验证。
- GitHub CLI 参数或远端 API 行为可能随版本变化；契约测试只能验证仓库内命令结构与
  Shell 语法，不能证明真实 GitHub 写入。
