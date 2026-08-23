# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: trivial
- Change record: not-applicable

## Delivery Summary
- Outcome: `.github/workflows/build.yml` 中 `Package CLI archive` 与
  `Store CLI archive` 两步骤的触发条件由仅 v 标签扩展为
  `startsWith(github.ref, 'refs/tags/v') || github.event_name == 'workflow_dispatch'`，
  使 workflow_dispatch 手动触发同样生成并上传三平台 CLI 归档；工作流头部
  注释同步为「+ CLI 三平台归档」。v 标签推送路径、Release 发布门控
  （publish 仅 v 标签 canonical 仓库）、产物内容与下载端
  `pattern: filehub-cli_*.tar.gz` 均保持不变。
- Handoff: 静态验证已通过（YAML 解析、条件真值表、改动范围与发布门控
  核对）。托管 runner 的实际 artifact 输出需由下一次 workflow_dispatch
  运行的 Actions 页面确认；本地沙箱无法触发 GitHub Actions 托管环境（与
  020-023 相同的证据边界）。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-cli-artifact-manual-trigger | workflow_dispatch 时三平台同样打包并上传 CLI 归档；v 标签路径不变；仅改 build.yml 两处 CLI 步骤 if 与头部注释 | proposal.md P-001 与 Scope 的 In scope | build.yml 第 137/149 行两处 if 均为 tag 或 workflow_dispatch，第 4 行注释同步更新；`rg` 确认 Release 下载 pattern 与 publish 门控（67/241/256 行）未改 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | run 32585239301 的步骤结论（三平台 CLI 编译 success、归档步骤 skipped）、artifact API 返回的 `filehub-server`/`web-dist`、build.yml 全部 build job 步骤 | 逐步骤推演 workflow_dispatch 下 version job 是否产出 version 输出（无 if，正常执行）、三平台 `Package CLI archive` 是否在条件变为 true 后先打包后上传、`archive: false` 下产物 basename 是否与下载 pattern 匹配（CLI 名长期为 filehub-cli_版本号_平台名.tar.gz，匹配 filehub-cli_*.tar.gz 通配） | 手动触发时三平台均可完成 打包 -> 上传，产物可被 release 下载 pattern 命中；server/web-dist 上传路径不受影响 | pass |
| boundaries-and-failure-paths | build-image job 第 198 行已有的 push 或 workflow_dispatch 同款表达式、release job 的 publish 门控、包与上传两步骤的条件一致性 | 检查新条件最坏边界：手动运行未推 v 标签（publish=false）不会触发 release/镜像推送；两步骤 if 相同避免打包后不传或传无文件；未来若新增 push main 触发源，条件仍只覆盖 tag 与手动 dispatch，不会意外发布；Windows tar 命令沿用 tag 路径既有用法 | 无边界漏项：手动运行只增产 CLI 归档，不触发发布；包/传条件一致，无孤档状态 | pass |
| regression-and-side-effects | git diff 全量、`rg` 扫描 `refs/tags/v`/`workflow_dispatch`/publish 引用、YAML 解析结果、git status 工作树差异 | 核对改动是否波及下载 pattern、gh release 汇编、GHCR 推送、server/web-dist 上传（均无改动）；确认 `filehub-server.json` 与 `harness/scripts/edit-guard.py` 两处既有未提交修改未被本次改动触碰（基线对比才会排除，diff 仅含 build.yml） | 唯一变更文件为 build.yml 共 6 行（注释 1 行 + 两处 if）；发布面与下载面零回归，既有脏文件保持原样 | pass |

## Verification
- Targeted check: 用 python `yaml.safe_load` 解析 build.yml 并断言两个 CLI
  步骤 if 的精确值；对 (event, ref) 三组组合做条件真值表（v 标签 push /
  workflow_dispatch 均运行，push main 不运行）；`rg` 核对下载 pattern 与
  publish 门控未变；`git diff --name-only` 确认改动仅落在 build.yml
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本地沙箱无法触发 GitHub Actions 托管 runner | 最终 artifact 输出需由下一次 workflow_dispatch 运行确认；若 GitHub 改变 event/ref 或 artifact 语义需按托管日志复检 | no |
| F-2 | low | 环境未安装 actionlint | 未能本地跑 actionlint；新表达式与同文件 198 行已由 023 用 actionlint 1.7.12 验证过的写法一致，风险很低 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 两处 CLI 步骤条件已按 proposal 精确扩展，YAML 解析与条件真值表
  全部通过；独立缺陷发现覆盖行为逻辑、边界失败路径与回归副作用，发布门控
  与下载端未受影响，剩余仅为托管运行确认与 actionlint 两项非阻塞说明。
