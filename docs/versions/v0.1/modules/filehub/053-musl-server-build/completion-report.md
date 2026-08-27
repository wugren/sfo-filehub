# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/053-musl-server-build.md

## Delivery Summary

- Outcome: Linux filehub-server 已改用 musl 工具链构建——`build-docker.sh` 与
  CI `build` job 均使用 `x86_64-unknown-linux-musl`，产物为 static-pie linked
  （不再依赖 `/lib64/ld-linux-x86-64.so.2`），复制/上传路径切换到
  `target/x86_64-unknown-linux-musl/release/filehub-server`，artifact 名
  `filehub-server` 与全部下载端保持不变；CI `build-image` 新增真实容器冒烟，
  同时等待 filehub-server 进程存在与 `/healthz` 可达，失败输出 `docker logs`；
  `docker/README.md` 已同步 musl 前置要求；macOS/Windows 与 Linux CLI 构建
  路径未改动。
- Handoff: 本地定向验证全部通过（musl release 完整链接、`file` 静态链接
  检查、musl 产物实跑、`/proc/*/comm` 进程探活复现、stub docker 整脚本流程、
  YAML 解析与路径映射）；真实 `docker run` 冒烟需由下一次托管 runner 的
  workflow_dispatch / v 标签运行确认。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-musl-server-build | Linux filehub-server 改用 x86_64-unknown-linux-musl 编译并打包进 Alpine 镜像，无需 glibc loader；仅改 build-docker.sh、CI Linux server 构建/上传路径与 docker 构建说明 | proposal.md P-001 | build-docker.sh 使用 `--target "$MUSL_TARGET"` 并从 target/x86_64-unknown-linux-musl/release 复制；build.yml 安装 musl 工具链、server 构建加 `--target ${MUSL_TARGET}`、上传路径 target/${{ env.MUSL_TARGET }}/release/filehub-server；`file` 显示 static-pie linked，stub docker 上下文内产物同样验证 | 匹配 | pass |
| fh-image-smoke-run | CI 构建镜像后真实启动并探测 `/healthz`，失败输出 `docker logs`；仅改 build-image job | proposal.md P-002 | build.yml build-image 在 Docker image inspect 后新增 Smoke test container startup 步骤：docker run 后循环探测 filehub-server 进程与 /healthz，失败输出 docker logs；YAML 解析通过，进程探活命令本机复现通过 | 匹配 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | build-docker.sh 全流程、build.yml build/build-image/release 各步骤、docker/Dockerfile 与 entrypoint.sh、musl 产物 file/实跑结果 | 反向推演：若 smoke 只探测 nginx 提供的 /healthz，entrypoint 以后台方式启动 filehub-server，二进制 exec 失败时 nginx 仍会启动、healthz 仍可达，因此必须额外检查进程存在；`/proc/[0-9]*/comm` 匹配 filehub-server（13 字符未超 comm 上限）在本机实跑复现通过；上传名 filehub-server 与 build-image/release 下载名一致，仅上传源路径变化 | 冒烟检查点选择正确，能捕获本次“二进制 not found”缺陷；上传/下载映射无断链 | pass |
| boundaries-and-failure-paths | 三平台矩阵条件、workflow_dispatch 与 v 标签两种触发、容器启动失败路径、busybox 工具可用性、release 打包引用 | 检查 matrix.server 仅在 ubuntu 行为 true，macos/windows 不安装 musl 工具链且保持原生路径；容器启动即退出的场景由 45 秒探测循环 + docker logs 转非零退出覆盖；alpine 内 wget 已由 Dockerfile HEALTHCHECK 证明可用，busybox grep 支持 -q；缺失 musl 目标时脚本 rustup 自动安装、缺失 musl-gcc 仅告警不掩盖既有验证结论 | 边界路径成立：平台隔离正确、失败时可诊断、工具可用性有据 | pass |
| regression-and-side-effects | 全仓 rg 扫描、release job 下载/打包/资产名校验引用、docker README 构建章节、根 README 本地开发命令 | 检查是否残留按 target/release/filehub-server 上传的镜像路径（仅根 README 本地开发快速启动引用原生路径，属 051 在制文档且与 Docker 构建无关，不误改）；检查 release 端是否依赖 Linux 产物链接形态（仅打包与校验大小，artifact 名未变）；检查 Cargo.lock/依赖图与 admin-web 源码未被本任务改动 | 无镜像/发布链路残留引用，其他交付面零改动，改动与既有在制任务边界清晰 | pass |

## Verification

- Targeted check: `sh -n build-docker.sh`；CI `build.yml` Python YAML 解析
  （jobs、musl 步骤、冒烟步骤齐全）；`cargo build --release -p filehub-server
  --target x86_64-unknown-linux-musl` + `file`/`ldd` 静态链接检查；
  musl 产物 `timeout 5` 实跑；`/proc/*/comm` 进程名探活复现；stub docker 下
  完整执行 `./build-docker.sh`（musl release + Vite 构建 + 上下文产物 file
  检查 + docker build 参数核对）；上传→下载→运行路径与 artifact 名映射核对
- Result: pass
- Exception reason: not-applicable

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本地环境无 Docker 守护进程，仅完成 stub docker 与产物级验证 | 真实 `docker run` 冒烟需托管 runner（下一次 workflow_dispatch/v 标签）或带 Docker 的机器执行确认 | no |
| F-2 | low | 环境未安装 actionlint | 以 Python YAML 解析与关键 shell 行为复现代替 actionlint，不改变结论 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 交付与已确认提案逐项一致（musl 构建路径 + CI 容器冒烟），定向
  验证与三类缺陷发现全部通过；剩余仅为真实容器运行确认与 actionlint 替代
  两项非阻塞说明。
