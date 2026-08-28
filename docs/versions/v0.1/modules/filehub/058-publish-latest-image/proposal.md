---
task_manifest: task.yaml
status: approved
---

# GHCR 正式发布同时维护 latest 镜像标签

Risk profile: not-created (replace with ./risk-profile.yaml only after high-risk confirmation)

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: trivial
- Tier rationale / triggered boundaries:
  - 本任务直接改变 GHCR produced artifact 与 release/deployment 发布面：每次正式发布除不可变版本标签外，还会移动可变的 `latest` 标签；
  - 需要保证 `latest` 与同一次发布的 `vX.Y.Z` 指向完全相同的镜像，并保持普通手工构建不产生外部写入；
  - 改动局限于一个工作流发布步骤和其静态契约测试，但任务大小不消除发布面风险，因此建议 `high-risk`。
- Proposal and tier confirmation: 2026-08-28 用户回复「确认，按简单任务完成就好」，批准本提案并明确选择 `trivial` 层级。该选择低于建议的 `high-risk`；执行仍保留最小发布门禁回归检查，并在完成报告中记录线上 GHCR 写入无法本地验证的残余风险。

## Approval Record

- approver: 用户
- approval_date: 2026-08-28
- user_statement: 「确认，按简单任务完成就好」
- selected_tier: trivial

## Background and Goal

当前 GitHub Actions 只构建并推送 `ghcr.io/wugren/filehub:vX.Y.Z`。GHCR 不会自动生成 `latest` 标签，因此 `docker pull ghcr.io/wugren/filehub:latest` 失败。

目标是在既有授权发布条件成立时，同时推送版本标签与 `latest` 标签，使用户可以固定版本下载，也可以显式选择最新正式发布版本。

## Scope

### In scope

- 在 `.github/workflows/build.yml` 的 GHCR 正式发布步骤中，为已经完成构建和容器冒烟的同一镜像增加 `latest` 标签；
- 同一次授权发布同时推送 `ghcr.io/{owner}/filehub:v{version}` 与 `ghcr.io/{owner}/filehub:latest`；
- 保持版本标签为发布来源，`latest` 仅作为同一镜像的附加别名，不进行第二次独立构建；
- 扩展 `tests/github_actions_build_contract.py`，检查两个标签均只在授权发布步骤写入，且 `latest` 从已构建的版本镜像派生；
- 通过 YAML 解析、Bash 语法和工作流契约测试验证发布逻辑。

### Out of scope

- 不改变 `workflow_dispatch` 的 `publish=true`、`release_tag`、canonical 仓库、版本一致性或 tag commit/SHA 门禁；
- 不让普通手工构建推送任何 GHCR 标签；
- 不改变 `push v*` 触发策略、Docker 镜像内容、GitHub Release 资产或 Cargo 依赖解析；
- 不增加多架构镜像、`major`/`minor` 浮动标签、签名或 provenance；
- 不在本任务中直接触发线上发布或手工回填现有 `v0.1.0` 的 `latest`；工作流合入后需由下一次受控正式发布产生线上标签；
- 不触碰工作树中已有的 `Cargo.lock`、`harness/scripts/edit-guard.py`、`filehub-server.yaml`、`filehub.db` 等无关改动。

### Boundary with neighboring modules

本任务只影响仓库级 GitHub Actions/GHCR 发布契约，不修改 filehub-server、admin-web、CLI、Dockerfile 或产品 API 行为。

## Requirement Review

需求合理。保留 `vX.Y.Z` 让部署可以固定到不可变版本，同时增加 `latest` 可以简化希望跟随最新正式版本的用户拉取命令。主要权衡是 `latest` 属于可变标签，不适合作为要求可复现或严格回滚的生产部署唯一依据；因此实现必须保留版本标签，并让两个标签来自同一份已经测试的本地镜像。

选择在现有受控发布步骤内执行一次本地重标记并推送两个标签，而不是为 `latest` 再次构建镜像，以避免标签内容漂移。若第二个 push 失败，workflow 必须失败并暴露版本标签与 `latest` 可能短暂不一致的结果；不增加隐藏重试或吞掉错误。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-publish-latest-image | 每次通过现有授权门禁的 GHCR 正式发布，将同一个已构建镜像同时推送为 `vX.Y.Z` 与 `latest`；普通构建不推送 | `.github/workflows/build.yml` 与对应静态契约测试 | `latest` 便于消费但会随正式发布移动；固定部署仍应使用版本标签或 digest | 定向契约测试确认 `latest` 由版本镜像重标记、两个标签均被推送、发布条件未改变；YAML/Bash 检查通过 | 不直接触发/回填线上发布，不增加其他浮动标签或改变镜像内容 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - 下一次通过现有授权门禁的正式发布后，`docker pull ghcr.io/wugren/filehub:latest` 与对应的 `docker pull ghcr.io/wugren/filehub:vX.Y.Z` 获取同一镜像内容；
  - 普通 `workflow_dispatch` 构建仍不推送 `latest` 或版本标签。
- Required evidence:
  - `tests/github_actions_build_contract.py` 的 unit、dv、integration 套件通过；
  - 工作流 YAML 可解析，所有 `run` 脚本通过 `bash -n`，`git diff --check` 通过；
  - 静态检查证明 `latest` 只在既有授权发布步骤中从版本镜像派生并推送；
  - GHCR 线上标签只有在后续 hosted Actions 正式发布成功后才能确认，本地验证不宣称线上已更新。
- Explicit non-goals:
  - 不更改应用代码、产物内容、版本规则或现有发布授权边界。

## Risks

- `latest` 是可变引用，消费者若不固定版本或 digest，重新部署时可能获得不同镜像；版本标签继续保留作为稳定入口。
- 两次 registry push 不是原子事务；如果版本标签 push 成功而 `latest` push 失败，会出现短暂不一致，但 workflow 会失败并明确暴露该状态。
- 本地无法执行真实 GHCR 写入；线上成功仍需后续受控 hosted Actions 发布结果证明。
