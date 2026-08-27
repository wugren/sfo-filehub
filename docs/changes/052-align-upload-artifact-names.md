# 052 GitHub Actions 上传 artifact 命名对齐：服务端 filehub-server、客户端 filehub-cli-{平台}

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/052-align-upload-artifact-names/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/052-align-upload-artifact-names/proposal.md
- Affected paths: `.github/workflows/build.yml`（server/CLI 上传步骤、release
  CLI 下载步骤）
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

按用户 2026-08-26「确认」采纳的方案 A 执行，方向与 021 相反：不再利用
`archive: false` 的 basename 直传语义，而是让上传 `name` 成为真实 artifact
名，并同步下载端匹配：

- server 上传：`name: server-binary` → `name: filehub-server`，移除
  `archive: false`；`build-image`/`release` 的 `name: filehub-server` 下载
  保持不变；
- CLI 上传：`name: cli-${{ matrix.artifact }}` →
  `name: filehub-cli-${{ matrix.artifact }}`（三平台唯一：linux-x86_64 /
  macos-aarch64 / windows-x86_64），移除 `archive: false`；
- release CLI 下载：`pattern: filehub-cli_*.tar.gz` →
  `pattern: filehub-cli-*`（保留 `merge-multiple: true`）。

依据：pinned `upload-artifact@v7` 在 `archive: false` 单文件直传时忽略 `name`
参数（artifact 名取文件 basename）；上传到同一 artifact 名在 v4+ 被明确禁止，
因此三平台采用 `filehub-cli-{platform}` 唯一后缀。移除 `archive: false` 后
单文件 artifact 下载布局不变（`ctx/server/filehub-server`、
`dist/raw/server/filehub-server`、`dist/raw/cli/*.tar.gz`），CLI 打包文件名
`filehub-cli_${VERSION}_${ARTIFACT}.tar.gz` 与 Release 四件资产名均不变。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: yes —— 上传 artifact 从「basename 直传」变为「默认归档 +
  name 权威（filehub-server / filehub-cli-{platform}）」，release 下载匹配面
  同步改为 `filehub-cli-*`。证据/缓解：下载后的单文件布局与四件 Release 资产
  文件名、镜像与发布逻辑不变；改动仅限 `.github/workflows/build.yml`，回滚
  即撤销该文件改动，无需协调。
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no（本任务的变更记录/
  完成报告是 standard 普通交付件，非规则或检查器变更）
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: python YAML 解析（5 个 job 结构完整）；`rg` 扫描确认无
  `server-binary`、`cli-${{ matrix.artifact }}` 上传名与
  `filehub-cli_*.tar.gz` 下载 pattern 残留；上传名↔下载名↔后续
  `chmod`/`cp`/`verify` 文件引用映射逐一核对（server：`filehub-server`；
  CLI：`filehub-cli-{platform}` → `pattern: filehub-cli-*` →
  `dist/raw/cli/*.tar.gz` → 四件资产校验）；actionlint 未安装，以完整 YAML
  语法校验替代。
- Result: pass
- Residual risk or follow-up: 本地沙箱无法运行托管 artifact 服务，真实命中
  需由下一次 `workflow_dispatch` 与 v 标签运行确认；若 GitHub 后续调整
  upload/download artifact 语义，以托管运行日志为准复检。
