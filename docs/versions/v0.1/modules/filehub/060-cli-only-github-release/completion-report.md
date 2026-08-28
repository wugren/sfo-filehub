# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: trivial
- Change record: not-applicable
- Approved proposal: proposal.md

## Delivery Summary

- Outcome: GitHub Release job 现在只下载、校验并上传 Linux、macOS、Windows 三个平台的 CLI `.tar.gz`，不再下载或打包 server/admin-web，也不再在 Release notes 中列出 server 包。server 二进制与 admin-web 仍作为内部 Actions artifacts 进入 Docker 镜像构建、容器冒烟和受控 GHCR 发布。根 README 已同步为“Release 只提供 CLI，server 通过 Docker 镜像交付”。
- Handoff: 新建 Release 将只包含三个 CLI 包；更新已有 Release 时 workflow 只上传或覆盖 CLI 包，但不会主动删除历史上已经存在的 server 附件。真实 GitHub Release/GHCR 写入需由后续受控 hosted Actions 运行确认。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-cli-only-github-release | 正式 GitHub Release 只上传三平台 CLI 包；server 只通过既有 Docker 镜像发布；README 与交付契约一致；不改变构建、镜像内容及发布授权门禁 | proposal.md P-001 | `.github/workflows/build.yml:459-509` 仅下载、校验并发布三个 CLI 包；`.github/workflows/build.yml:197-230,334-428` 保留 server/CLI 构建、server/web artifact 消费、镜像冒烟与 GHCR 发布；`README.md:41-59` 只列出 CLI Release 包并指向 Docker server；`tests/github_actions_build_contract.py:230-281` 覆盖 Release、Docker 和 README 契约 | 与批准提案一致 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `.github/workflows/build.yml` 的 `build`、`build-image`、`release` jobs 与 Release notes；对应契约测试 | 反查 Release job 是否仍下载 server/web、生成 server 包、接受非三个资产或在 notes 中列出 server；同时检查是否误删 Docker 所需的 server/web artifacts | Release job 只匹配 `filehub-cli-*`，逐一验证三个预期包并强制资产总数为 3；server/web 只在 `build-image` 中继续消费，镜像构建、冒烟和 GHCR push 保持原条件 | pass |
| boundaries-and-failure-paths | `Verify release assets`、`Create or update GitHub Release`、已有 Release 的 `gh release upload --clobber` 分支、提案历史资产非目标 | 检查缺包、额外包、损坏 tar、首次创建与已有 Release 更新路径；尝试确认未重新上传的旧 server 附件是否会被自动删除 | 缺包、额外包或不可读取 tar 均使 job 失败；首次创建只带三个 CLI 包。更新已有 Release 只上传 CLI，`--clobber` 不删除未列出的历史附件；这与“不删除历史 Release 资产”的明确非目标一致并作为残余边界保留 | pass |
| regression-and-side-effects | `README.md` 当前下载说明、全仓旧 server Release 字符串搜索、Docker artifact 链路、unit/DV/integration 契约、任务 pre-edit 基线与 git 状态 | 搜索仍指导用户下载 server Release 包的当前消费者；检查 server 编译、admin-web 构建、Docker artifact 路径、tag/SHA 与授权写入门禁是否漂移；核对是否夹带既有脏文件 | 根 README 的陈旧手工 server 部署路径已删除并由测试锁定；历史任务记录保持不改；5 个 unit、6 个 DV、7 个 integration 用例全部通过；现有 `Cargo.lock`、Harness 脚本、本地 YAML/数据库改动未纳入本任务 | pass |

## Verification

- Targeted check: `python3 tests/github_actions_build_contract.py --suite unit`（5/5）；`python3 tests/github_actions_build_contract.py --suite dv`（6/6）；`python3 tests/github_actions_build_contract.py --suite integration`（7/7，包含 workflow YAML 解析与全部 `run` 脚本 `bash -n` 检查）；`git diff --check -- .github/workflows/build.yml README.md tests/github_actions_build_contract.py docs/versions/v0.1/modules/filehub/060-cli-only-github-release` 通过；定向 `rg` 未发现当前 workflow/README 残留 server Release 包或四资产门禁
- Result: pass
- Exception reason: 当前任务不直接触发 hosted Actions、GitHub Release API 或 GHCR 写入；本地证据验证 workflow 契约，真实外部发布结果留给后续受控发布运行。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `.github/workflows/build.yml:497-500` 的已有 Release 分支仅执行 `gh release upload --clobber`；提案明确不删除历史 Release 资产 | 旧 Release 若已经存在 server 附件，本次 workflow 更新不会主动删除它；新建 Release 与后续上传均只产生 CLI 附件 | no |
| F-2 | low | 本地仅执行 PyYAML、Bash 语法与静态契约测试，未持有或使用 GitHub/GHCR 发布权限 | 无法在本地证明 hosted artifact 传递及真实 GitHub Release/GHCR 写入成功 | no |
| F-3 | low | 全仓 `git diff --check` 命中任务开始前已有的 `harness/scripts/edit-guard.py` CRLF/尾随空白；任务范围检查通过且 pre-edit 基线已隔离该文件 | 无关既有脏文件使全仓 diff check 非零，不能归因于本任务，也未获授权修复 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: `fh-cli-only-github-release` 已按确认范围完成；Release job 对外只生产三个 CLI 附件，README 与新交付方式一致，Docker server 构建和发布链路保持完整。18 个工作流契约测试及独立反向检查均通过，未发现阻塞缺陷；历史附件不主动清理、真实 hosted 发布尚未执行及无关脏文件问题均已明确记录为非阻塞残余边界。
