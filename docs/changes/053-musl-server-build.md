# 053 filehub-server Linux 构建改用 musl 工具链，修复 Alpine 镜像启动失败

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/053-musl-server-build/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/053-musl-server-build/proposal.md
- Affected paths: `build-docker.sh`、`.github/workflows/build.yml`、
  `docker/README.md`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

按用户 2026-08-26「确认」采纳的默认方案执行：

- `build-docker.sh`：filehub-server 改用
  `cargo build --release -p filehub-server --target x86_64-unknown-linux-musl`，
  复制路径改为 `target/x86_64-unknown-linux-musl/release/filehub-server`；脚本
  在 rustup musl 目标缺失时自动安装，musl-gcc 缺失时仅告警并提示
  `musl-tools`（本机已验证无 musl-gcc 也能完整链接）；
- CI `build` job：Ubuntu/Linux 矩阵安装 musl 目标与 `musl-tools`，server
  构建与上传路径切换到 musl 目标目录，artifact 名 `filehub-server` 与全部
  下载端（build-image/release）保持不变；macOS/Windows/CLI 构建路径不变；
- CI `build-image` job：新增真实容器冒烟——`docker run` 后同时等待
  `/proc/*/comm` 中出现 `filehub-server` 进程并访问 `http://127.0.0.1/healthz`，
  失败时输出 `docker logs` 后退出非零；该检查同时覆盖「server 二进制无法
  exec」与 nginx 健康检查两条路径；
- `docker/README.md`：构建章节补充 rustup/musl 目标前置要求、musl-gcc
  提示与 Alpine 无需 gcompat 的说明。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: yes —— Linux filehub-server 产物链接形态从 glibc 动态链接
  变为 musl 静态链接，镜像 CI 新增真实 `docker run` 冒烟。证据/缓解：artifact
  名、镜像 tag、entrypoint/nginx/数据布局与 API 契约均不变；回滚等于撤销
  脚本/CI 修改并用旧命令重建，无需协调；musl 构建目标与 `musl-tools` 均为
  CI 显式声明，Cargo.lock 未变。
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no（本任务改动的是
  产品 CI 与构建脚本，非 Harness 规则/检查器）
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `sh -n build-docker.sh` 通过；CI `build.yml` python YAML
  解析通过（jobs/musl 步骤/冒烟步骤齐全）；
  `cargo build --release -p filehub-server --target x86_64-unknown-linux-musl`
  完整通过且 `file` 显示 static-pie linked；musl 产物实跑 5 秒持续运行
  （timeout 退出码 124）；`/proc/*/comm` 进程名探活命令在本机复现通过；
  stub docker 下完整运行 `./build-docker.sh`（musl release 构建 + Vite 构建
  + 上下文内 server 产物 file 检查 + docker build 参数核对）；上传→下载→
  运行路径与 artifact 名映射核对无残留
- Result: pass
- Residual risk or follow-up: 本环境无 Docker 守护进程，真实 `docker run`
  冒烟需由托管 runner（下次 workflow_dispatch/v 标签运行）或带 Docker 的
  机器执行确认；actionlint 未安装，以 Python YAML 解析与关键 shell 行为
  复现代替
