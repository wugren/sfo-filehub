# GitHub Actions：CLI 三平台编译并入单一 build 矩阵 job，server 与 CLI 一次性发布

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/022-ci-merge-cli-job/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/022-ci-merge-cli-job/proposal.md
- Affected paths: `.github/workflows/build.yml`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

按用户确认的方案乙（standard 层级）实施：把原 `test-rust` 与 `build-cli`
两个独立 job 合并为单一 `build` 矩阵 job，三平台矩阵保持不变：

- ubuntu-24.04（linux-x86_64，`server: true`）：workpace 全量测试 +
  `-p filehub-server` + `-p filehub-cli` release 构建；非 PR 上传
  `target/release/filehub-server` 单文件产物；
- macos-14（macos-aarch64）与 windows-2022（windows-x86_64）：只构建
  `-p filehub-cli`；v 标签时三个平台都把
  `dist/filehub-cli_<version>_<平台>.tar.gz` 单文件上传；
- `build-image` 与 `release` 的 `needs` 由 `test-rust`/`build-cli` 统一改为
  `build`；
- `release` job 保持唯一发布入口：一次下载 server、web-dist 与三平台 CLI
  归档，打包 server+web 归档后用一个 `gh release create/upload` 发布四件
  tar.gz（server+web 一份、三平台 CLI 三份），实现「server 与 CLI 一次性
  发布」；
- `version`、`test-web`、镜像构建、GHCR 与版本/仓库门控均未改动。

有意保留的行为：测试只在 Linux 条目执行一次（避免三平台重复跑同一套
workspace 测试）；CLI 三平台原生产物与 4 件 Release 附件数量不变。

## Risk Screen

- Public contract, protocol, or CLI change: no（CLI 命令面与二进制不变）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: yes（仅限本次已确认的
  standard 范围：改动 GitHub Actions job 结构与发布编排；server 与三平台 CLI
  产物内容、4 件 Release 附件与一次性发布契约保持不变，无依赖/供应链/兼容性
  变化；风险在确认范围内，不升级、不改变需求边界）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: python YAML 解析 jobs 结构通过；actionlint 1.7.12 零告警；
  `needs` 图（build-image/release → build）与上传/下载产物名映射逐一核对；
  本地 `cargo build --release -p filehub-server -p filehub-cli` 成功，
  `target/release/filehub --version` 输出 0.1.0
- Result: pass
- Residual risk or follow-up: 本环境无法真实触发 hosted runner；合并后的
  build 矩阵与一次性发布需在仓库内下一次 push/tag 运行的记录中确认（与
  020/021 相同证据边界）
