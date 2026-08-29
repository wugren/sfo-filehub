# GitHub Release 使用版本号标题并确保正式发布

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/063-release-title-version/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/063-release-title-version/proposal.md
- Affected paths: `.github/workflows/build.yml`、`tests/github_actions_build_contract.py`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

首次创建 Release 时将 `--title` 从带产品前缀的文本改为精确的 `${VERSION}`。已有
Release 先上传或覆盖三个 CLI 附件，成功后再执行 `gh release edit`，将标题对齐为
`${VERSION}` 并设置 `--draft=false`。tag、发布授权、tag/SHA 校验、附件和 notes 均保持
不变。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: yes；公开 Release 的显示标题会变化，已有 draft 在下一次已授权发布运行中会变为正式发布；该影响正是批准提案的既定范围，且未增加发布入口、权限或产物
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

已确认的发布元数据影响是选择 `standard` 的原因；实施中未发现需要修改需求、范围或
验收边界的新风险。

## Verification

- Targeted check: `python3 tests/github_actions_build_contract.py --suite unit`（5/5）；`--suite dv`（7/7）；`--suite integration`（7/7）；actionlint v1.7.12；Release 步骤和全部 workflow `run` 脚本的 Bash 语法检查；任务范围 `git diff --check`
- Result: pass
- Residual risk or follow-up: 本地契约测试不能证明真实 GitHub Release 写入；最终线上标题和发布状态需要下一次受控 hosted Actions 运行确认
