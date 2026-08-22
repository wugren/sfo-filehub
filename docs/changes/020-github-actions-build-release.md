# 020 GitHub Actions 单工作流：编译、Docker 镜像与 v 标签发布

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/020-github-actions-build-release/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/020-github-actions-build-release/proposal.md
- Affected paths: `.github/workflows/build.yml`、`.gitignore`（仅新增一行
  `node_modules.bak-esbuild-stuck/`，用于忽略本环境 Windows 锁定原生文件的
  残留目录，属交付附加的环境卫生行）
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

按用户确认的口径新增唯一个工作流 `.github/workflows/build.yml`：

- 触发：`push master`（编译+测试+构建镜像、不推送）、`pull_request`
  （编译+测试、不构建镜像）、`push v*` 标签（编译+测试+构建镜像并发布）。
- `version` job 用 `cargo metadata` 读取 workspace 版本（取
  `filehub-server`），并一次性完成两道发布门控：`v<tag>` 必须等于该版本；
  只有 canonical 仓库（默认 `<owner>/sfo-filehub`，可用仓库变量
  `CANONICAL_REPOSITORY` 覆盖）才允许发布。
- `test-rust`：ubuntu 上 `cargo test --workspace --all-targets` +
  release 构建 `filehub-server`；`test-web`：`npm ci` 后运行
  `test:unit`/`test:integration`/`test:dv`。
- `build-cli`：linux/macos/windows 原生 runner 矩阵构建 `filehub-cli`
  release，tag 运行打包 `filehub-cli_<version>_<os>_<arch>.tar.gz`。
- `build-image`：复用 018/019 的纯运行时 `docker/Dockerfile` 与最小上下文
  （server 二进制 + admin-web dist + nginx/entrypoint），镜像 tag 为
  `ghcr.io/<owner>/filehub:v<version>`（owner 转小写）；master 只构建，
  publish 门控通过时用 `GITHUB_TOKEN` push 到 GHCR。
- `release`：tag 发布时下载 server/bin、web dist 与三个 CLI 归档，打包
  `filehub-server_<version>_linux_x86_64.tar.gz`（server + web 目录结构），
  校验四件产物后用 `gh` 附到 GitHub Release（已存在则 `--clobber` 上传）。

运行器/动作引用与当前已跑通仓库一致：checkout v6、upload-artifact v7、
download-artifact v8 按 SHA 锁定，setup-node v7/cache v6 按 major tag 引用；
不引入第三方发布动作（GitHub Release 与 GHCR 均用 `gh`/docker CLI +
`GITHUB_TOKEN`）。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no（发布只用仓库内置
  GITHUB_TOKEN；GHCR 仅 `packages: write`、Release 仅 `contents: write`
  的最小 job 权限，无 PAT/npm/Docker 外部密钥）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: yes —— 新增了发布交付面（GHCR 镜像 + GitHub Release
  产物）。证据/缓解：发布仅由 canonical 仓库的 `v*` 标签触发，tag 必须与
  Cargo workspace 版本一致；镜像 tag 不可变（不覆盖 `latest`）；Release 与
  镜像可由维护者删除/重跑，无不可逆迁移。保留用户确认的 standard 层级，
  残余风险记录在提案 Risks 与完成报告。
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: actionlint 1.7.12 全量语法校验；python yaml 解析；CI 命令等价
  本地实跑——`cargo test --workspace --all-targets`、`cargo build --release -p
  filehub-server -p filehub-cli`、`npm ci` + `test:unit`（40 通过）+
  `test:integration`（7 通过）+ `test:dv`（构建+dist 校验通过）；发布门控 shell
  模拟 6 场景（canonical v 标签/mismatch tag/非 canonical/变量覆盖/branch/PR）；
  镜像上下文与 release 打包结构模拟（用真实 server 二进制与 admin-web dist，
  产物 4 件 tar.gz 与上下文路径核对通过）。
- Result: passed
- Residual risk or follow-up: hosted runner 实跑与真实 GHCR/Release 发布需在
  仓库推送 tag 后验证；两个 Windows 锁定原生文件保留在
  `admin-web/node_modules.bak-esbuild-stuck/`（已 gitignore，需在 Windows 侧
  删除），不影响 CI。
