# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/052-align-upload-artifact-names.md

## Delivery Summary

- Outcome: `.github/workflows/build.yml` 上传 artifact 名已按要求对齐——server
  上传 `name: filehub-server`、CLI 上传 `name: filehub-cli-{platform}`
  （linux-x86_64/macos-aarch64/windows-x86_64），并移除 `archive: false`
  使 `name` 成为真实 artifact 名；`build-image`/`release` 的 server 下载
  `name: filehub-server` 生效，release CLI 下载 `pattern` 改为
  `filehub-cli-*` 并保留 `merge-multiple: true`。CLI 打包命令、Release 四件
  资产文件名、镜像/GHCR 与发布逻辑均未改动。
- Handoff: 静态定向验证全部通过（YAML 解析、旧名残留扫描、上传名↔下载名↔
  后续文件引用映射核对、多触发分支布局推演）；托管 runner 的真实 artifact
  命中需由下一次 workflow_dispatch 与 v 标签运行确认。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-upload-artifact-names | 上传 artifact 名必须为 filehub-server 与 filehub-cli-{platform}，镜像/release 下载端按这些名称命中；仅改 upload 与 release CLI 下载步骤 | proposal.md P-001 | build.yml 中 server 上传 name=filehub-server、CLI 上传 name=filehub-cli-${{ matrix.artifact }}、两处 archive:false 已移除；release CLI pattern=filehub-cli-*；server 下载 name=filehub-server 保持 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|--------------------|-------------------|-----------------------------------|--------|
| behavior-and-logic | build.yml 全部上传/下载步骤、pinned upload-artifact@v7 README（archive:false 时 name 被忽略、v4+ 禁止多 job 同名）、download-artifact@v8 pattern 与 merge-multiple 语义、release verify/gh release 段落 | 反向推演：若任一 job 仍用旧名上传 server-binary/cli-{平台}，下载会否漏配（已确认无）；按官方语义重放「三平台 upload → release pattern 命中 → dist/raw/cli/*.tar.gz → 四件资产校验」全链路，确认上传名与下载 pattern 不再分离 | 三处改动的行为链路一致，无仍按旧名匹配的下载点或遗漏 job | pass |
| boundaries-and-failure-paths | 三平台矩阵命名、workflow_dispatch 与 v 标签两种触发下 CLI 上传条件、v4+ 同名互斥约束、merge-multiple 单目录收敛、publish 门控与四件资产校验 | 检查三平台 artifact 名是否唯一（filehub-cli-linux-x86_64 / macos-aarch64 / windows-x86_64 互异）；检查 workflow_dispatch 无 tag 时 release 不运行、build-image 只依赖 filehub-server 与 web-dist；检查 pattern=filehub-cli-* 不会误匹配其他产物（不存在其他 filehub-cli 前缀 artifact） | 边界路径全部成立：三平台名称唯一合法，release 门控与镜像下载不依赖 CLI 产物，pattern 无多余命中 | pass |
| regression-and-side-effects | git 改动范围（build.yml + 本任务交付文档）、全仓 rg 扫描、download-artifact 对单文件 zip artifact 的解压布局、verify 步骤期望文件名 | 检查移除 archive:false 后下载布局是否变化（单文件 artifact 解压后仍以原文件名落在目标目录）；检查 Release 资产名与上传打包名是否漂移（仍为 filehub-cli_${VERSION}_${ARTIFACT}.tar.gz）；检查 web-dist、镜像构建、GHCR、触发器与 action 版本未触碰 | 无布局/资产名漂移，web-dist 与其他逻辑零改动，全仓无其他工作流引用这些 artifact 名 | pass |

## Verification

- Targeted check: python YAML 解析（jobs 完整）；`rg` 无 server-binary /
  cli-${{ matrix.artifact }} / filehub-cli_*.tar.gz 残留；上传名→下载名→
  chmod/cp/verify 引用映射核对；两触发分支下载布局推演
- Result: pass
- Exception reason: not-applicable

## Findings

| ID | severity | evidence | problem | blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本地沙箱与官方 README/action 源码 | 托管 runner 的 artifact 服务端行为未实跑，最终命中需由下一次 workflow_dispatch 与 v 标签运行确认 | no |
| F-2 | low | 环境中未安装 actionlint | 未运行 actionlint，已用完整 YAML 解析与名称映射静态校验替代，不改变结论 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 交付与已确认提案逐项一致，定向验证与三类别缺陷发现全部通过；CLI
  artifact 采用用户确认的 filehub-cli-{platform} 唯一命名，剩余仅为托管运行
  确认与 actionlint 替代两项非阻塞说明。
