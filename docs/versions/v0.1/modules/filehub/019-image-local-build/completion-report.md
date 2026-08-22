# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/019-image-local-build.md

## Delivery Summary

- Outcome: filehub 镜像构建方式改为本地脚本驱动：新增根目录 `build-docker.sh`
  （本地编译 release server + admin-web，组装最小上下文并 `docker build`），
  新增纯运行时 `docker/Dockerfile`（不再有 Node/Rust 编译阶段），移除根目录
  多阶段 Dockerfile/.dockerignore，README 构建说明同步更新；运行时行为（nginx
  反代、固定 `/data` + `-v` 挂载、环境变量）保持不变。
- Handoff: 在本地执行 `./build-docker.sh` 生成 `filehub:dev`（可用
  `IMAGE_TAG` 覆盖），随后按 018 的 `docker run -v` 示例启动。已验证脚本语法、
  server/admin-web 本地构建、最小上下文内容与 docker 调用参数；真实 Docker
  构建因本环境无守护进程留待 Docker 环境/CI 执行。

## Proposal Consistency

| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-image-local-build | 新增 build-docker.sh：工具链检查、本地编译 server 与 admin-web、组装最小上下文、docker build、trap 清理 | proposal.md P-001 | build-docker.sh 四步流程与 stub docker 全流程验证：cargo release 通过、admin-web 同源构建通过、上下文无编译缓存、`build -t filehub:test` 参数正确、退出清理 | 交付与提案一致 | pass |
| fh-image-runtime-image | 新增纯运行时 docker/Dockerfile，移除根目录多阶段 Dockerfile/.dockerignore | proposal.md P-002 | docker/Dockerfile 仅 `FROM nginx`、COPY 上下文路径逐一核对；根目录 Dockerfile/.dockerignore 已删除 | 交付与提案一致 | pass |
| fh-image-docs | README 构建说明改为 ./build-docker.sh 与 IMAGE_TAG，变更记录与完成报告补齐 | proposal.md P-003 | docker/README.md 构建/运行/compose/env 表与脚本一致；docs/changes/019-image-local-build.md 已完成 | 交付与提案一致 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 逐行审查 build-docker.sh（工具链检查、cargo/npm 分支、mktemp 上下文组装、docker 调用、trap 清理）与 docker/Dockerfile COPY 路径 | 挑战 IMAGE_TAG 覆盖、node_modules 缺失/已有两条分支、临时上下文内容与 docker 参数、Dockerfile 是否残留编译阶段 | 脚本在 IMAGE_TAG=filehub:test 下完整跑通；已有 node_modules 分支输出复用提示并直接构建；上下文仅含二进制/dist/运行时文件；Dockerfile 无 FROM node/rust | pass |
| boundaries-and-failure-paths | 工具链缺失检查、npm ci 失败路径、mktemp 失败/中断清理、.env.local 对构建的影响、无 Docker 守护进程场景 | 挑战缺失命令时是否提前退出、临时目录是否随退出清理、错误 env 文件是否进入镜像、镜像层是否包含 target/node_modules | 缺失工具链会在步骤 0 报错退出；trap 使用临时上下文变量；构建产物只复制 dist 与 release 二进制，env 文件不进入上下文；上下文树核对无 target/node_modules | pass |
| regression-and-side-effects | 对比 018 交付面：根目录 Dockerfile/.dockerignore 删除、docker/nginx.conf 与 entrypoint 路径、README 引用、server/admin-web 源码与锁文件 | 搜索构建文档/docker 命令残留、Dockerfile COPY 目标是否匹配上下文、package-lock/Cargo.lock 是否被脚本改动 | 根目录不再放 Dockerfile，`docker build .` 会明确失败并指向脚本；README 已同步；脚本不写锁文件；server/admin-web 源码未改 | pass |

## Verification

- Targeted check: `sh -n build-docker.sh`；完整运行 `build-docker.sh`（stub
  docker、`IMAGE_TAG=filehub:test`）——`cargo build --release -p filehub-server`
  通过、admin-web `VITE_API_BASE_URL=/ npm run build` 通过、最小上下文内容与
  docker 参数核对通过；`docker/Dockerfile` 静态核对；根目录旧 Dockerfile/
  .dockerignore 删除确认
- Result: passed
- Exception reason: 本环境无 Docker 守护进程，真实镜像构建与容器启动冒烟未执行
  （stub docker 已验证脚本传给 Docker 的参数与上下文）；`npm ci` 全新安装路径
  因环境残留的 Windows 可选依赖文件 EIO 未跑通，复用已有 node_modules 的构建
  路径已验证，残余项已在 Findings 记录。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `command -v docker/podman/nginx` 均无结果 | 真实 `docker build`/`docker run` 冒烟未在本机执行，属于环境能力限制；已用 stub docker 验证脚本调用与上下文，交由 Docker 环境/CI 完成最终冒烟 | no |
| F-2 | low | 本环境 admin-web/node_modules 中 `@esbuild/win32-x64/esbuild.exe` 无法从 WSL 侧 unlink（EIO） | 该预置残留使本环境无法跑通 `npm ci` 全新安装路径；脚本已有“存在 node_modules 则复用”分支并验证通过，用户在干净 checkout 或清理 node_modules 后即可走 `npm ci` | no |
| F-3 | low | docker/README.md 与脚本输出 | 根目录 Dockerfile 移除后，原先 `docker build .` 习惯会失败；README 已改为 `/build-docker.sh` 唯一入口并说明，属于有意的构建面收敛而非缺陷 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 交付完整覆盖已确认提案 P-001~P-003：本地脚本编译并生成镜像、纯运行时
  Dockerfile、移除镜像内编译阶段与 README 同步均已落地；脚本语法、两级本地构建、
  最小上下文与 docker 调用参数验证通过；独立缺陷发现三分类全 pass，仅三条 low
  级非阻塞情形（无 Docker 守护进程、环境残留导致 `npm ci` 全新路径未在本机跑通、
  构建入口收敛），不影响已确认交付结论。
