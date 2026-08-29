# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/063-release-title-version.md

## Delivery Summary

- Outcome: GitHub Release 首次创建时使用精确的 workspace 版本号作为标题，例如
  `0.1.0`，不再显示 `filehub 0.1.0` 或把 `v0.1.0` tag 当作标题。已有 Release 在三个
  CLI 附件上传成功后会再次校验 tag/SHA，再用相同版本标题执行
  `gh release edit --draft=false`，从而同步修正旧标题和 draft 状态。
- Handoff: 改动集中在 `.github/workflows/build.yml` 与
  `tests/github_actions_build_contract.py`；5 个 unit、7 个 DV、7 个 integration 契约
  测试、actionlint v1.7.12、Bash 语法与差异检查均通过。未直接修改当前线上 Release，
  新标题和正式发布状态将在下一次受控 hosted Actions 发布运行后生效。

## Proposal Consistency

| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-release-version-title | Release 标题精确使用 workspace 版本，tag 保留带 v 前缀；create 和 update 两条路径最终均为非 draft；不改变发布门禁、附件、notes 或 GHCR | proposal.md P-001（Scope、Proposal Items、Success Criteria） | `.github/workflows/build.yml` 的 Create or update GitHub Release 步骤；`test_release_title_uses_version_and_existing_draft_is_published`；全部 workflow 契约套件 | 交付覆盖批准要求，线上状态保持未直接写入 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 逐行核对 Release 的 view、upload、edit、create 分支以及 VERSION、RELEASE_TAG、SOURCE_SHA 数据来源 | 分别模拟不存在 Release、已有正式 Release、已有 draft Release，检查标题是否误用 tag 或产品前缀，以及 draft 是否只上传附件而未发布 | create 使用 `${VERSION}` 标题且默认正式发布；existing 分支在 upload 后以同一版本标题和 `--draft=false` 收敛；契约测试分别约束两条路径 | pass |
| boundaries-and-failure-paths | 检查资产数量门禁、已有 Release 上传失败、tag 漂移、edit 失败、create 的 `--verify-tag` 与 gh v1.7.12 帮助契约 | 反查上传失败后是否仍编辑元数据、两次 GitHub 写入之间 tag 是否可能漂移、draft 发布参数是否有效 | `set -e` 使 upload 或 edit 失败时任务失败；第二次写入前重新 fetch 并比较 tag/SHA；本机 gh 帮助明确支持 `gh release edit --draft=false` 发布既有 draft | pass |
| regression-and-side-effects | 对照任务开始基线检查 workflow 全部发布门禁、CLI 附件、Release notes、GHCR 和无关脏文件；执行全部 workflow 契约套件 | 搜索是否改动 tag 格式、canonical repository、三个 CLI 文件名、Docker 镜像标签或夹带既有 Cargo.lock、Harness、本地 YAML 和数据库改动 | 仅 workflow 元数据分支和对应测试进入交付范围；5 unit、7 DV、7 integration 与 actionlint 全部通过，无关脏文件保持任务前状态 | pass |

## Verification

- Targeted check: `python3 tests/github_actions_build_contract.py --suite unit`；`--suite dv`；`--suite integration`；actionlint v1.7.12；Release 步骤与全部 workflow `run` 脚本 Bash 语法检查；任务范围 `git diff --check`
- Result: pass
- Exception reason: 未触发 hosted Actions，也未调用写 API 修改当前 `v0.1.0` Release；本地证据验证 workflow 契约，真实 GitHub 标题和发布状态需由下一次受控发布运行确认。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 当前 GitHub API 仍返回名称 `filehub 0.1.0` 且 `draft=true`；本任务明确不直接执行线上写入 | 仓库改动不会在提交前立即改变现有 Release；需要下一次受控 hosted Actions 发布运行应用新逻辑 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: Release 标题与 tag 职责已经分离，首次创建和已有 Release 更新路径都会使用
  精确版本标题，已有 draft 也会在授权发布中转为正式状态；发布门禁、tag/SHA 复核、
  附件、notes 与 GHCR 未发生非预期变化。三类独立反查和全部定向验证通过，无阻塞发现。
