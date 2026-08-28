# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: trivial
- Change record: not-applicable
- Approved proposal: proposal.md

## Delivery Summary

- Outcome: GitHub Actions 的既有授权 GHCR 发布步骤现在会把已构建并完成容器冒烟的 `vX.Y.Z` 镜像重标为 `latest`，依次推送版本标签和 `latest`，再读取两个远端 manifest digest 并要求完全相等。普通 `workflow_dispatch` 构建仍受原有 `publish == true` 条件保护，不会产生 registry 写入。
- Handoff: 该变更只使后续正式发布具备 `latest`；当前线上仅有的 `v0.1.0` 不会被本地修改自动回填。真实 `latest` 需要合入后运行一次受控正式发布才能出现。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-publish-latest-image | 通过现有授权门禁的 GHCR 正式发布将同一个已构建镜像同时推送为 `vX.Y.Z` 与 `latest`；普通构建不推送，现有版本/tag/SHA 门禁不变 | proposal.md P-001 | `.github/workflows/build.yml` 在唯一 `Publish image to GHCR` 步骤中从 `version_image` 重标 `latest_image`、显式推送两个标签并比较远端 digest；`tests/github_actions_build_contract.py` 新增同源标签及唯一 push 步骤契约检查 | 与批准提案一致 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `build-image` 的 build、smoke、publish 三个步骤及新增 DV 契约测试 | 反查 `latest` 是否被第二次独立构建、是否错误使用不存在的本地标签、是否只推送 `latest` 而丢失版本标签、两个远端标签是否可能静默指向不同 digest | `latest_image` 由已存在的 `version_image` 执行 `docker tag` 得到；两个标签均显式 push；非空版本 digest 与 latest digest 必须相等，否则 `set -euo pipefail` 使 job 失败 | pass |
| boundaries-and-failure-paths | 发布 step 的 `if`、`needs.authorize-publication` 输出、tag/SHA 二次校验以及 workflow 全部 `docker push` 脚本 | 枚举全部 job/step 中包含 `docker push` 的脚本，尝试寻找 build-only 或未授权写入；核对 tag、登录、两个 push、digest 检查的执行顺序与失败传播 | 全工作流只有 `build-image/Publish image to GHCR` 一个 push 脚本，且条件仍严格为授权输出 `publish == true`；tag/SHA 漂移检查仍在登录和 push 前；任一命令失败均终止发布 | pass |
| regression-and-side-effects | 任务 pre-edit 基线、工作流完整 unit/DV/integration 契约、当前 git diff/status | 检查手工输入默认值、Cargo lock 分发、发布依赖图、Release 资产、checkout SHA、Bash 语法是否漂移；核对任务是否夹带已有 `Cargo.lock`、Harness 脚本、配置或数据库改动 | unit 5/5、DV 5/5、integration 5/5 全通过；任务交付差异仅 `.github/workflows/build.yml` 与 `tests/github_actions_build_contract.py`，已有无关脏文件由基线隔离且未修改 | pass |

## Verification

- Targeted check: `python3 tests/github_actions_build_contract.py --suite unit`（5/5）；`--suite dv`（5/5）；`--suite integration`（5/5，包含全部 `run` Bash 语法检查）；额外 YAML 审计确认全工作流只有一个 `docker push` step、条件为授权发布输出、重标与两个 push 顺序正确、远端 digest 相等断言存在；任务范围 `git diff --check -- .github/workflows/build.yml tests/github_actions_build_contract.py docs/versions/v0.1/modules/filehub/058-publish-latest-image` 通过
- Result: pass
- Exception reason: 当前环境没有 `actionlint` 和 Docker daemon，且本任务不直接触发 hosted Actions/GHCR 写入；YAML 由 PyYAML 成功解析、所有 run 脚本由 `bash -n` 检查，真实线上标签需下一次受控正式发布验证。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 当前 GHCR tags API 仅返回 `v0.1.0`；提案明确不直接回填线上 registry | 代码完成后 `latest` 仍需一次正式发布才会在线上出现，不能把本地静态验证表述为已可下载 | no |
| F-2 | low | 本机 `actionlint` 不可用且无 Docker daemon；PyYAML、Bash 语法、工作流静态契约全部通过 | 无法在本地复现 GHCR 的真实双标签 push 与远端 manifest 可见性 | no |
| F-3 | low | registry 的两个 tag push 是两个顺序命令，workflow 使用严格失败传播和最终 digest 比较 | registry 不提供跨两个标签的原子事务；若第二次 push 失败，版本标签可能已发布而 `latest` 未更新，但 Actions 会明确失败 | no |
| F-4 | low | 全仓 `git diff --check` 命中任务开始前已有的 `harness/scripts/edit-guard.py` CRLF/尾随空白；任务范围检查通过 | 无关脏文件使全仓 diff check 非零，不能归因于本任务，也未获授权修复 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: `fh-publish-latest-image` 已按批准范围完成；版本标签与 `latest` 从同一本地镜像派生、在唯一授权步骤中推送，并通过远端 digest 相等断言防止静默漂移。15 个工作流契约测试及额外对抗审计均通过，未发现阻塞缺陷；线上尚未回填、缺少本地 Docker/actionlint、双 push 非原子性与无关文件的既有空白问题均已记录为非阻塞残余风险。
