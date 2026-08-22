# filehub 镜像改由本地脚本编译并生成（Docker 内不再编译）

- Status: complete
- Owner module: filehub（文件集散 v0.1）
- Task manifest: `docs/versions/v0.1/modules/filehub/019-image-local-build/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/019-image-local-build/proposal.md`
- Affected paths: `build-docker.sh`、`docker/Dockerfile`、`docker/README.md`、`docs/changes/019-image-local-build.md`；并移除 018 的根目录 `Dockerfile` 与 `.dockerignore`
- Explicit tier override: none（用户确认 standard）
- Expanded high-risk packet: none

## Approach

1. 新增根目录 `build-docker.sh`（POSIX sh）：先检查 docker/cargo/npm，随后
   `cargo build --release -p filehub-server`，再在 `admin-web/` 执行
   `VITE_API_BASE_URL=/ npm run build`（`node_modules` 缺失时才先 `npm ci`，
   已有依赖树直接复用），然后用 `mktemp -d` 组装最小构建上下文
   （`Dockerfile`、`nginx.conf`、`entrypoint.sh`、`server/filehub-server`、
   `web/dist/*`），最后执行 `docker build -t "${IMAGE_TAG:-filehub:dev}"`；
   `trap` 保证临时上下文随 EXIT/INT/TERM 清理。
2. 新增 `docker/Dockerfile` 为纯运行时打包：`FROM nginx:1.27-alpine`，
   安装 `jq`、准备 `/data`、`COPY` 本地构建上下文里的二进制与静态产物、
   保留 healthz 健康检查与 `EXPOSE 80`；不再出现 `FROM node`/`FROM rust`
   编译阶段。
3. 移除 018 的根目录多阶段 `Dockerfile` 与 `.dockerignore`，镜像构建唯一入口
   收敛为 `./build-docker.sh`。
4. `docker/README.md` 构建章节改为 `./build-docker.sh`/`IMAGE_TAG` 用法，
   并说明 `npm ci` 仅在节点依赖缺失时执行。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no（运行时数据布局不变）
- Security, privacy, or trust-boundary change: no（凭据/数据目录语义不变；
  本地 env 文件不进入镜像上下文）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: **yes**——镜像构建方式从 Docker 内多阶段编译改为本地脚本
  编译 + 纯运行时镜像打包，属于已确认的构建面调整；生成镜像的机器需具备
  cargo/npm 工具链，依赖仍由 Cargo.lock/package-lock 锁定，且不影响既有本地
  调试与测试链。
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check:
  - `sh -n build-docker.sh` 通过；
  - 完整执行 `build-docker.sh`（stub docker，`IMAGE_TAG=filehub:test`）：
    `cargo build --release -p filehub-server` 通过（仅存量 server 警告）；
    admin-web 复用已有 node_modules 并以 `VITE_API_BASE_URL=/` 完成构建；
    最小上下文核对通过（server/filehub-server 可执行、web/index.html 存在、
    nginx.conf/entrypoint.sh 存在、无 target/node_modules、Dockerfile 无编译
    阶段）；stub docker 收到 `build -t filehub:test <context>`；
  - `docker/Dockerfile` 静态核对：仅 nginx 运行时，COPY 路径与上下文布局一致；
  - 根目录 `Dockerfile`/`.dockerignore` 确认移除。
- Result: passed
- Residual risk or follow-up: 本环境无 Docker 守护进程，真实 `docker build` 与
  `docker run` 冒烟未执行，建议在 CI 或有 Docker 的机器完成镜像构建与 `-v`
  持久化冒烟；本环境预置的 node_modules 里有一个无法从 WSL 侧删除的 Windows
  可选依赖残留，脚本的 `npm ci` 全新安装路径未在此环境跑通（复用已有依赖的
  构建路径已验证），在干净 checkout 或宿主侧清理后执行即可。
