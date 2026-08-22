# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/022-ci-merge-cli-job.md

## Delivery Summary
- Outcome: `.github/workflows/build.yml` 中 `test-rust` 与 `build-cli` 已合并为
  单一 `build` 矩阵 job：ubuntu-24.04 条目测试并编译 server+CLI，
  macos-14/windows-2022 条目编译 CLI；server 二进制与三平台 CLI 归档上传
  条件沿用 020/021 约定；`build-image`/`release` 的依赖改为 `build`。
  `release` job 保持唯一发布入口，一次 `gh release create/upload` 同时发布
  server+web 归档与三平台 CLI 归档（4 件 tar.gz），实现用户要求的
  「CLI 三平台 + server 与 CLI 一次性发布」。
- Handoff: 静态与本地验证已通过（YAML 解析、actionlint 1.7.12、needs 图、
  产物名映射、本地两包 release 构建）。托管 runner 的矩阵与发布运行记录需在
  仓库内下一次 push/v 标签触发后确认（与 020/021 相同证据边界）。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-consolidate-rust-build | test-rust 与 build-cli 合并为单一 build 矩阵 job；linux 条目测试+编译 server+CLI，macos/windows 条目编译 CLI；上传逻辑不变 | proposal.md P-001 | build.yml 中仅剩 build（三平台矩阵，linux 条目带 server: true），不再存在 test-rust/build-cli job；Test/Build server 步骤按 matrix.server 门控；CLI 构建对所有平台执行；上传步骤与 021 一致 | 匹配 | pass |
| fh-one-shot-release | release 依赖与产物校验对齐 build；保留一次 gh release 发布 server+web 与三平台 CLI 共四件归档 | proposal.md P-002 | release needs=[version, build, test-web, build-image]；仍下载 filehub-server、web-dist、filehub-cli_*.tar.gz（merge-multiple），四件产物校验与 gh release 引用不变 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 合并前后 build.yml 全量 diff、build job 的矩阵行与步骤顺序、build-image/release 的 needs 与下载步骤 | 逐行比对原 test-rust/build-cli 步骤是否全部保留：测试 (linux)、server 构建 (linux)、CLI 构建 (三平台)、server 上传 (非 PR)、CLI 打包/上传 (tag)；matrix.server 在 macos/windows 行为空 → 对应步骤为 false，语义与需求一致 | 步骤无丢失；job 名与依赖引用全部更新；release 一次性发布链路完整 | pass |
| boundaries-and-failure-paths | PR / main push / v tag / fork 非 canonical / 错误 tag 五种触发形态、archive:false 产物名、download-artifact pattern | 推演 PR 无上传且不触发 image/release；main push 由 linux 条目上传 filehub-server 供 build-image 下载；v tag 时三平台 CLI 归档文件名互异且被 filehub-cli_*.tar.gz 命中；fork 时 version 输出 publish=false，release 跳过；tag 与版本不符时 version job 先失败 | 各边界行为与原工作流一致，无空下载/缺产物路径 | pass |
| regression-and-side-effects | rg 全仓库残留引用、git diff 改动范围、actionlint 输出、本地 cargo build 结果、既有未提交文件 | 确认 build.yml 无 test-rust/build-cli 残留引用；改动仅限 build.yml 与 022 任务文档；本地 `cargo build --release -p filehub-server -p filehub-cli` 成功且 filehub --version=0.1.0；未触碰预存在的 filehub-server.json 与 harness/scripts/edit-guard.py 改动 | 无行为回归：改动只落在 build.yml 与 022 任务文档，预存在的 filehub-server.json 与 harness/scripts/edit-guard.py 修改未被触碰；本地双包 release 构建与 CLI 版本输出正常 | pass |

## Verification
- Targeted check: python YAML 解析确认 jobs 仅含 version、build、test-web、build-image、release；actionlint 1.7.12 对 build.yml 零告警；needs 图（build-image/release 依赖 build）与实际上传文件 basename（server 为 filehub-server，CLI 为 filehub-cli_版本号_平台名.tar.gz）映射核对通过；本地 cargo build --release 同时成功构建 filehub-server 与 filehub-cli
- Result: pass
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本地沙箱与 actionlint 无法执行托管 runner | 合并后的矩阵运行与 v 标签一次性发布需下一次 push/tag 的 Actions 记录做最终确认 | no |
| F-2 | low | GitHub Actions 矩阵语义 | 运行列表仍按三平台显示三个 build 子 job（原生三平台编译的固有形态），与提案 Requirement Review 已声明一致，非缺陷 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 方案乙按确认范围完整落地：CLI 三平台保留、编译收敛为单一 build 矩阵
  job、release 单 job 一次性发布 server 与 CLI；定向验证 + 独立缺陷发现全部
  通过，剩余仅为托管运行确认与矩阵展示形态两项非阻塞说明。
