---
task_manifest: task.yaml
status: approved
---

## Approval Record

- approver: user
- approval_date: 2026-08-23
- user_statement: 确认（2026-08-23 用户回复「确认」，采纳 standard 层级与
  本提案口径：触发改为 workflow_dispatch + v 标签推送，同时移除 main push
  与 pull_request 自动触发）

# GitHub Actions 触发策略：人工触发 + v 标签推送

Risk profile: not-created（确认层级为 trivial/standard 时不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 不满足 trivial：本任务改变 GitHub Actions 的触发策略（移除 main 提交与
    pull_request 自动触发，新增 workflow_dispatch），属于 CI 门禁与
    release/deployment 执行面变化，不符合 trivial 的「无材料影响」条件；
  - 未升级 high-risk：只改 `.github/workflows/build.yml` 的 `on` 触发定义与
    个别 job/步骤条件，不改源码、测试、依赖、数据 schema、API 契约，不引入
    外部凭据，发布门控（v 标签 == workspace 版本、canonical 仓库）保持不变，
    与 020/022 同属构建/发布编排面，按 standard 处理。
- Proposal and tier confirmation: 用户于 2026-08-23 回复「确认」，采纳
  standard 层级与提案口径（含移除 pull_request 自动触发）；本提案置为
  `status: approved`，任务按 standard 层级执行。

## Background and Goal

当前工作流在 main 分支每次 push（以及向 main 的 pull_request）都会自动触发
完整 CI（编译、测试、镜像构建），用户希望改为只在以下两种情况下运行：
1. 人工触发（Actions 手动运行 `workflow_dispatch`）；
2. 推送版本标签（`v*` tag）时自动触发编译 + 发布。

## Scope

### In scope

1. `on` 触发改为：
   - `workflow_dispatch:`（人工触发，无 inputs）；
   - `push: tags: ["v*"]`（版本标签推送）；
   - 移除 `push: branches: [main]` 与
     `pull_request: branches: [main]`；
2. `build-image` job 条件从 `github.event_name == 'push'` 改为
   `github.event_name == 'push' || github.event_name == 'workflow_dispatch'`，
   保证人工触发时也能生成 Docker 镜像；镜像推送仍由
   `needs.version.outputs.publish == 'true'` 门控（只有 canonical 仓库推
   v* 标签才为 true），人工运行只构建不推送；
3. 清理因移除 PR 触发而失效的步骤条件：`Store server binary` 的
   `github.event_name != 'pull_request'` 与 `Store admin-web dist` 的
   `github.event_name != 'pull_request'`（PR 已不是触发源）；
4. 工作流头部注释同步为新的触发语义。

### Out of scope

- 不修改编译/测试命令、镜像内容、GHCR 与 Release 发布逻辑；
- 不修改版本门控（v 标签 == workspace 版本）与 canonical 仓库门控；
- 不新增 workflow_dispatch 输入参数（分支选择用 Actions 默认的 run-branch，
  发布仍只走 v 标签）；
- 不引入分支保护/必检状态等仓库设置（GitHub 仓库设置不在本文件内）；
- 不修改 022 已合并的 `build` 矩阵 job 结构。

### Boundary with neighboring modules

只改 `.github/workflows/build.yml`。发布动作仍然只发生在 push `v*` 标签时；
人工触发用于完整构建/测试/镜像验证。main 提交与 PR 不再自动触发 CI——
这是用户明确选择的取舍，回归检查改为人工触发执行。

## Requirement Review

- 需求合理：开发期频繁 push 时每次全量触发（三平台 Rust 编译 + 前端测试 +
  镜像构建）成本高且噪音大；只在真正需要验证/发布时运行更符合该仓库当前
  的交付节奏。
- 权衡/需要用户知悉的副作用：
  - 移除 pull_request 触发后，PR 合入前面没有自动测试门禁，代码回归需要
    人工触发 workflow_dispatch 验证（本提案按此口径，若仍需保留 PR 检查请
    在确认时说明）；
  - 人工触发运行不发布 GHCR/Release（publish 仅在 v 标签 + canonical 仓库
    为 true）。
- 选择的方向：按用户字面要求，触发面收敛为 workflow_dispatch + v 标签。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-ci-trigger-manual-tag | 触发改为 workflow_dispatch + push v* 标签；build-image 条件覆盖人工触发；清理 PR 相关死条件并同步注释 | 只改 .github/workflows/build.yml | 每次 main push/PR 不再自动跑 CI，需要时人工触发 | YAML 可解析；on 只剩 workflow_dispatch 与 push.tags；build-image 在人工与 tag 触发均可构建；无 pull_request 残留条件 | 不新增 inputs；不改发布门控；不引入仓库策略设置 |

## Success Criteria

- 用户可见结果：
  - push main（任意提交）不再产生 Actions 运行；向 main 的 PR 也不自动触发；
  - Actions 页面可手动触发 `Build, test, image and release`，运行完整编译、
    测试与镜像构建（不推送 GHCR、不创建 Release）；
  - 推送 `v*` 标签（与 Cargo workspace 版本一致）时自动触发并完成
    GHCR 镜像发布 + GitHub Release（server 与三平台 CLI 一次性发布）。
- Required evidence:
  - `python` YAML 解析通过；actionlint 1.7.12 对 build.yml 零告警；
  - `on` 触发源核对、build-image/release/publish 门控核对；
  - 托管 runner 记录：人工触发与下一次 v 标签推送各运行一次确认（与
    020/021/022 相同证据边界）。
- Explicit non-goals:
  - 人工触发不发布；PR/main 不自动触发；
  - 不改发布门控与构建内容。

## Risks

- 失去自动回归门禁：main push/PR 均不再自动检查，合入前后需要人工触发
  workflow_dispatch 验证；已在本提案 Requirement Review 与 Scope 中声明，
  确认即接受该取舍。
- 人工触发语义：若用户后续希望 workflow_dispatch 也能发布，需要另立任务
  增加发布开关，本提案不包含。
- 托管运行确认：本环境无法真实触发 hosted runner，最终以上传仓库后的运行
  记录为准。
