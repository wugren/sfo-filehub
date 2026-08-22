# GitHub Actions 触发策略：人工触发 + v 标签推送

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/023-ci-trigger-policy/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/023-ci-trigger-policy/proposal.md
- Affected paths: `.github/workflows/build.yml`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

按用户确认口径（standard 层级）实施：

- `on` 改为 `workflow_dispatch:` + `push: tags: ["v*"]`，移除
  `push: branches: [main]` 与 `pull_request: branches: [main]`，main 提交与
  PR 不再自动触发；
- `build-image` job 的 `if` 从 `github.event_name == 'push'` 扩展为
  `== 'push' || == 'workflow_dispatch'`，保证人工触发也能完整构建镜像；
  镜像推送仍由 `needs.version.outputs.publish == 'true'` 门控（只有
  canonical 仓库 push v* 标签为 true），人工运行只构建不推送；
- 清理已失去触发源的死条件：`Store server binary` 与 `Store admin-web dist`
  的 `!= 'pull_request'` 判断；server 二进制上传改为仅 `matrix.server`
  门控，web dist 无条件上传；
- 工作流头部注释同步为新的触发语义。

发布语义不变：v 标签 + canonical 仓库才触发 GHCR push 与 GitHub Release
（server + 三平台 CLI 一次性发布，同 022）。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: yes（仅在已确认的
  standard 范围内：移除 main/PR 自动触发，CI 门禁改为人工触发 + v 标签；
  job 结构、构建命令与发布产物面不变，无依赖/供应链/兼容性影响）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: YAML 解析确认 on 只含 workflow_dispatch 与 push.tags[v*]；
  actionlint 1.7.12 零告警；rg 无 pull_request/branches 残留；build-image
  if 覆盖 push + workflow_dispatch；发布门控（publish 仅 v 标签 canonical
  仓库）未变
- Result: pass
- Residual risk or follow-up: main/PR 不再自动跑 CI，代码回归需人工触发
  workflow_dispatch 验证（用户已确认的取舍）；托管 runner 的人工触发与
  v 标签运行记录待推送后确认（与 020/021/022 相同证据边界）
