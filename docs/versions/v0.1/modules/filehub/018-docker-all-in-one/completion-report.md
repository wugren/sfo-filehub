# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/018-docker-all-in-one.md

## Delivery Summary

- Outcome: 交付 filehub 统一 Docker 镜像构建与运行面：根目录多阶段 `Dockerfile`
  把 `filehub-server` release 二进制与 admin-web 静态产物装入 nginx 镜像，入口
  脚本把环境变量翻译成 server 配置并拉起 nginx + server；页面与 `/account/`、
  `/api/v1/` 同源反代；数据目录固定为容器内 `/data`，外部位置由 `-v` 卷挂载
  决定（不使用数据目录环境变量）。
- Handoff: 交付物位于 `Dockerfile`、`.dockerignore`、`docker/nginx.conf`、
  `docker/entrypoint.sh`、`docker/README.md`。已通过 server release 构建、
  admin-web 同源构建、入口脚本仿真（配置生成/JSON 转义/会话密钥持久化/非法
  输入拒绝/进程清理）以及真实 server 登录冒烟；因本环境无 Docker 守护进程，
  镜像真实构建与容器启动冒烟留待 Docker 环境/CI 执行。

## Proposal Consistency

| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-docker-multistage-build | 根目录多阶段 Dockerfile 与 .dockerignore，镜像包含 server release 二进制与 admin-web dist | proposal.md P-001 | Dockerfile 三个阶段（node build / rust build / nginx runtime）与 .dockerignore 已落地；`cargo build --release -p filehub-server` 与 `VITE_API_BASE_URL=/ npm run build` 通过 | 交付与提案一致 | pass |
| fh-docker-web-proxy | nginx 80 提供管理页，`/account/` 与 `/api/v1/` 反代到容器内 server，SPA 回退 | proposal.md P-002 | docker/nginx.conf 站点模板包含 /healthz、两个 `location ^~` 反代、`try_files` 回退；入口脚本用 FH_SERVER_PORT 替换并 `nginx -t` | 交付与提案一致 | pass |
| fh-docker-runtime-config | 数据目录固定 /data（无数据目录环境变量），会话密钥/管理员/端口/归档上限可配，入口脚本拉起双进程 | proposal.md P-003 | docker/entrypoint.sh 用 jq 生成 `db_path=/data/filehub.db` 与 `files.data_dir=/data/files`；仿真验证配置值与 JSON 转义、会话密钥持久化、非法输入拒绝、进程清理；真实 server 用生成配置登录成功 | 交付与提案一致 | pass |
| fh-docker-docs | docker/README.md 说明构建、运行、卷挂载与 env 清单；变更记录与完成报告 | proposal.md P-004 | docker/README.md 含 build/run/compose/env 表与运维提示；docs/changes/018-docker-all-in-one.md 与 completion-report.md 已按标准补齐 | 交付与提案一致 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 逐行审查 Dockerfile 阶段与 COPY 路径、docker/entrypoint.sh 的默认值/校验/JSON 生成/nginx 模板替换/双进程清理、nginx 站点 location 顺序 | 挑战带引号密码的 JSON 转义、随机会话密钥生成与跨重启复用、`__SERVER_PORT__` 替换、nginx 与 server 任一退出后的清理路径 | 密码 `p@ss"word` 生成后 JSON 解析回读一致；会话密钥 64 hex 且两次启动复用；nginx 模板按 9090 替换出两处 127.0.0.1:9090；仿真启动后无残留进程；Dockerfile COPY 目标均存在 | pass |
| boundaries-and-failure-paths | 入口入参校验分支、会话密钥文件不存在/已存在分支、`-v` 挂载数据目录固定语义、nginx client_max_body_size 0 与 server 归档上限关系 | 挑战 FH_SERVER_PORT 非法值/越界、FH_MAX_ARCHIVE_BYTES 非法值、空管理员用户名/密码、`/data` 未挂载时的目录创建、服务端超限后的行为边界 | 非法端口与空用户名被明确拒绝；max bytes 非整数被拒绝；`/data` 由入口 `mkdir -p` 兜底；上传体上限由 server 的 FH_MAX_ARCHIVE_BYTES 约束并已在 README 说明，nginx 不另设缓冲上限 | pass |
| regression-and-side-effects | 检查 server/admin-web 源码默认行为、Cargo.lock/package-lock 是否被动、既有构建链与文档消费者 | 搜索 VITE_API_BASE_URL 默认值是否被改动、仓库内是否新增了数据目录环境变量、docs/api/v1-contract.md 与 docs/modules/filehub.md 是否需要同步 | `session.ts` 默认值未改（镜像构建时仅以 `VITE_API_BASE_URL=/` 覆盖）；未引入 FH_DATA_DIR；server 源码/依赖锁文件未动；cargo release 构建与 npm build 均通过，无既有消费者回归 | pass |

## Verification

- Targeted check: `cargo build --release -p filehub-server` 通过（3m16s，仅存量
  server 警告）；`VITE_API_BASE_URL=/ npm run build` 通过（dist 正常产出）；
  `sh -n docker/entrypoint.sh` 通过；入口脚本 /tmp 仿真（真实 jq + nginx/
  filehub-server shim）覆盖配置生成、JSON 转义、会话密钥持久化、非法输入拒绝、
  nginx 模板替换与进程清理；真实链路：入口生成配置启动 release server 后
  `POST /account/login` 返回 HTTP 200、`err=0`；Dockerfile/.dockerignore 静态
  核对通过
- Result: passed
- Exception reason: not-applicable（本环境无 Docker 守护进程，`docker build`/
  `docker run` 冒烟未执行；提案成功证据允许以本机可执行验证集作为定向依据，
  真实镜像构建已在变更记录 Residual risk 中明确留待 Docker 环境）

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本环境 `command -v docker/podman/nginx` 均无结果 | 镜像真实构建与容器启动冒烟未在本机执行，属于环境能力限制而非交付缺陷；变更记录与 Handoff 已标注，待 Docker 环境/CI 验证 | no |
| F-2 | low | docker/nginx.conf 头部注释与 README | `docker/nginx.conf` 是带 `__SERVER_PORT__` 占位符的模板，直接复制进 nginx conf.d 会语法失败；镜像内由入口脚本替换后再 `nginx -t`，已文档化，非运行缺陷 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 交付完整覆盖已确认提案 P-001~P-004：多阶段镜像构建、nginx 同源反代、
  固定 `/data` + `-v` 挂载数据目录（无数据目录环境变量）、环境变量配置与
  README 均已按确认范围落地；server release、admin-web 构建、入口脚本仿真与
  真实 server 登录链路证据齐全；独立缺陷发现三分类全 pass，仅两条 low 级非阻塞
  情形（本机无 Docker 的真实构建冒烟、模板文件需由入口替换），与交付结论一致。
