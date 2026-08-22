# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: trivial
- Change record: not-applicable

## Delivery Summary
- Outcome: `.github/workflows/build.yml` 的下载端已与 `upload-artifact@v7`
  `archive: false` 的真实产物名对齐：`build-image` 与 `release` 的 server
  下载 `name` 改为 `filehub-server`；`release` 的 CLI 下载 `pattern` 改为
  `filehub-cli_*.tar.gz`。上传侧 `archive: false`、产物内容以及镜像构建和
  Release 发布逻辑保持不变。
- Handoff: 静态验证已通过（YAML 解析、残留引用扫描、下载名与上传文件
  basename 映射核对）。托管 runner 的实际下载成功需由下一次 master push
  触发镜像构建、以及 v 标签触发 Release 发布的运行记录确认；本地沙箱无法
  复现 GitHub Actions runner 的 artifact 服务端行为。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-artifact-download-names | 下载端按 archive:false 产物真实名匹配：server 下载 name 为 filehub-server，CLI 下载 pattern 为 filehub-cli_*.tar.gz，仅改三个下载步骤 | proposal.md P-001 与 Scope 的 In scope | build.yml 中 build-image/release 两处 server 下载 name 已改为 filehub-server，release 的 CLI 下载 pattern 已改为 filehub-cli_*.tar.gz；上传步骤 archive 与 name、镜像/Release 逻辑未改动 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | run 32565108750 的产物 API 与上传日志、upload-artifact v7 README 文档、build.yml 全部上传/下载步骤 | 核对每个由 archive:false 上传的路径 basename 必须等于下载端名称：target/release/filehub-server 对应 filehub-server，dist/filehub-cli_版本号_平台名.tar.gz 对应 filehub-cli_*.tar.gz 模式；同时确认 web-dist 走目录归档、名称不受 basename 改写影响 | 三个下载步骤与实际落库产物名一致，无其他仍按旧名匹配的下载步骤 | pass |
| boundaries-and-failure-paths | build-cli 三平台矩阵命名、download-artifact v8 的 pattern 与 merge-multiple 语义、release 的 publish 与 tag 门控 | 推演三平台 CLI 文件名 filehub-cli_0.1.0_linux-x86_64.tar.gz/macos-aarch64/windows-x86_64 是否互异且都被 filehub-cli_*.tar.gz 命中；核对 server 下载后 ctx/server 与 dist/raw/server 下的文件名与后续 chmod、cp 引用一致；确认 master push 不触发 release，避免无 CLI 产物场景 | 三平台文件名唯一且可被新模式命中，server 文件后续步骤引用无漂移，边界场景无遗漏 | pass |
| regression-and-side-effects | rg 全仓库扫描 server-binary 与 cli-* 残留、python YAML 解析结果、git diff 改动范围、工作流其他触发分支 | 检查是否还有其他工作流或文档按旧名下载（无）；确认改动未触及上传 name/archive、镜像构建命令、Release 汇编与 gh release 逻辑；确认 admin-web 的 web-dist 下载维持不变 | 唯一残留 server-binary 是上传步骤被忽略的 name 值（按提案非目标保留）；无行为回归，改动仅落在 build.yml 三个下载步骤 | pass |

## Verification
- Targeted check: 用 python 解析 `.github/workflows/build.yml` 确认 jobs 结构完整；`rg` 扫描仓库确认无按旧名下载的残留；人工对照下载名与 archive:false 上传路径 basename（server 为 filehub-server，CLI 为 filehub-cli_*.tar.gz，web-dist 保持 web-dist）
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本地沙箱无法触发托管 runner | 修复后需要下一次 master push 的镜像构建下载和 v 标签的 Release 下载运行记录做最终确认；若 GitHub 后续改变 artifact 命名或匹配行为需按托管日志复检 | no |
| F-2 | low | build.yml 上传步骤保留 name: server-binary 与 name: cli-平台名 | upload-artifact v7 在 archive:false 单文件上传下忽略这些 name，保留值仅为冗余说明，不产生行为影响；按提案非目标范围未删除 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 三个下载端名称与实际产物名完全对齐，定向静态验证全部通过；独立缺陷发现覆盖行为逻辑、边界失败路径与回归副作用，剩余仅为托管运行确认与冗余 name 值两项非阻塞说明。
