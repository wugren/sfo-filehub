# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/023-ci-trigger-policy.md

## Delivery Summary
- Outcome: `.github/workflows/build.yml` 的触发策略已改为「人工触发
  (workflow_dispatch) + v 标签推送（push.tags["v*"]）」，移除 main 分支 push
  与 pull_request 自动触发；`build-image` 条件同步覆盖人工触发（不推送），
  发布仍只在 v 标签 + canonical 仓库发生；清理了两个不再生效的
  `!= 'pull_request'` 步骤条件并更新文件头注释。
- Handoff: 静态验证已通过（YAML 解析、actionlint 1.7.12、触发源与门控核对）。
  托管 runner 的人工触发与 v 标签运行记录需推送后确认（与 020/021/022 相同
  证据边界）。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-ci-trigger-manual-tag | 触发改为 workflow_dispatch + push v* 标签；build-image 条件覆盖人工触发；清理 PR 相关死条件并同步注释；只改 build.yml | proposal.md P-001 | on 仅含 workflow_dispatch 与 push.tags；无 branches/pull_request 残留；build-image if 为 push 或 workflow_dispatch；两处 != pull_request 条件已移除；文件头注释已更新 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 触发块完整 diff、on 的 YAML 解析值、build-image/release/version 的 if 与 publish 门控、文件头注释 | 逐一推演三种触发形态：main push 不生成运行；manual 运行完成编译/测试/镜像构建但不推送 GHCR 且 release 跳过；v 标签 canonical 仓库运行完成镜像推送与四件 Release 发布 | 触发矩阵符合用户确认口径，无残留旧触发源，发布语义未变 | pass |
| boundaries-and-failure-paths | workflow_dispatch 语义、tag push 的 GITHUB_REF_TYPE、canonical 门控、artifact 上传条件 | 验证 version job 只在 push+tag 时输出 publish=true；manual 运行下 server/web 产物上传但 CLI 归档不打包（tag 条件为 false），release 被 publish 门控跳过；fork 推 tag 只构建不发布；错误 tag 在 version 阶段失败 | 各边界行为一致，人工触发不会误发布，tag 发布链路完整 | pass |
| regression-and-side-effects | rg 全仓库残留引用、actionlint 输出、git diff 范围、预存在的未提交文件 | 确认 build.yml 无 branches/pull_request/!= pull_request 残留；改动仅落在 build.yml 与 023 任务文档；未触碰 filehub-server.json 与 harness/scripts/edit-guard.py 的既有修改 | 无回归；改动范围与提案一致 | pass |

## Verification
- Targeted check: python YAML 解析确认 on 只含 workflow_dispatch 与 push.tags 的 v 星号模式；actionlint 1.7.12 对 build.yml 零告警；rg 扫描无 pull_request 或 branches 触发残留；build-image 的 job if 同时覆盖 push 与 workflow_dispatch，发布沿用 publish 门控
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本地沙箱无法触发 hosted runner | 人工触发与 v 标签发布的最终运行记录需在推送后确认 | no |
| F-2 | low | 触发策略变更本身 | main/PR 不再自动跑 CI，回归验证依赖人工触发；属用户已确认的取舍，非缺陷 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 触发策略按确认口径落地：仅人工触发与 v 标签触发，发布门控和构建
  内容不变；定向验证与独立缺陷发现全部通过，剩余仅为托管运行确认与已声明
  的人工触发取舍两项非阻塞说明。
