---
task_manifest: task.yaml
status: approved
---

## Approval Record

- approver: user
- approval_date: 2026-08-22
- user_statement: 确认；按 019 提案执行（本地脚本编译并生成镜像，Docker 内不再编译）。

# filehub 镜像改由本地脚本编译并生成（Docker 内不再编译）

Risk profile: not-created（standard 层级不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
  - 确认记录：2026-08-22 当前用户回复「确认」；确认内容即 019 提案：新增
    `build-docker.sh` 本地编译并生成镜像、新增纯运行时 `docker/Dockerfile`、
    移除根目录多阶段 Dockerfile/.dockerignore。
- Tier rationale / triggered boundaries:
  - 不满足 trivial：修改镜像构建交付面（去掉镜像内多阶段编译、新增本地构建脚本、
    调整 Dockerfile 形态），属于构建流程有界调整；
  - 未升级 high-risk：这是对 018 已交付镜像方案的**构建位置**修订，不改运行时
    行为、server/admin-web 源码、数据库和 API 契约；镜像内容与运行方式（nginx
    反代、固定 `/data` + `-v` 卷）保持不变；
  - 按仓库默认，有界单项目构建流程调整走 standard：变更记录 + 定向验证 +
    独立缺陷发现后完成报告。
- Proposal and tier confirmation: 2026-08-22 用户「确认」，层级 standard。

## Background and Goal

用户要求：Docker 里不再进行编译（即去掉 018 交付的多阶段 Dockerfile 中的
Node/Rust 构建阶段），改为本地提供一个脚本，脚本负责：1) 本地编译
filehub-server release 二进制；2) 本地构建 admin-web 静态产物；3) 组装最小
构建上下文并调用 `docker build` 生成最终镜像。镜像内只剩 nginx 运行环境与
打包好的二进制/静态文件，不安装编译链。

## Scope

### In scope

1. 新增根目录脚本 `build-docker.sh`（POSIX sh）：
   - 自动检查 `docker`、`cargo`、`npm` 是否可用；
   - 在仓库内执行 `cargo build --release -p filehub-server`；
   - 在 `admin-web/` 内执行 `npm ci` + `VITE_API_BASE_URL=/ npm run build`；
   - 用 `mktemp -d` 组装最小构建上下文（`Dockerfile`、`nginx.conf`、
     `entrypoint.sh`、`server/filehub-server`、`web/dist/*`），不包含
     `target/`、`node_modules/` 等体积目录；
   - 调用 `docker build -t "$IMAGE_TAG" <context>`（`IMAGE_TAG` 默认
     `filehub:dev`，可用环境变量覆盖）；
   - `trap` 清理临时上下文，失败/中断不留临时目录。
2. 新增 `docker/Dockerfile`（纯运行时打包，无编译阶段）：
   - `FROM nginx:1.27-alpine`，安装 `jq`，`COPY` 本地产出的
     `server/filehub-server` 与 `web/` 静态产物；
   - 保留 018 的 nginx 模板、entrypoint、健康检查与 `EXPOSE 80` 不变；
3. 移除根目录 018 的多阶段 `Dockerfile` 与 `.dockerignore`（不再作为镜像
   构建入口；镜像构建只走 `build-docker.sh` 组装的最小上下文）；
4. 更新 `docker/README.md`：构建方式改为 `./build-docker.sh`，保留 `-v`
   卷挂载运行示例、环境变量表与运维提示；
5. 按 standard 流程补 `docs/changes/019-image-local-build.md` 与任务包内
   `completion-report.md`。

### Out of scope

- 不修改 server/admin-web 源码、`docker/nginx.conf`、`docker/entrypoint.sh`
  与 `-v` 数据目录语义（018 已确认的行为保持不变）；
- 不做多架构镜像、不做镜像仓库推送、不新增 CI 工作流；
- 只提供 POSIX sh 脚本（Linux/macOS/WSL/Git Bash 可用）；Windows 原生
  cmd/PowerShell 版本不在本次范围；
- 不改镜像内 HTTPS/进程管理策略。

### Boundary with neighboring modules

只调整构建/打包边界：编译产物所有权从 Docker 构建阶段转移到宿主本地脚本；
运行期（nginx 反代、入口配置、`/data` 固定路径与 `-v` 挂载）与 018 一致。

## Requirement Review

需求合理：本地编译对开发和排障更直观，镜像体积也更小（不需要安装
Rust/Node 工具链），适合已有本地开发工具链的团队；代价是生成镜像的机器必须
具备 Rust 与 Node 环境，镜像可复现性从「镜像内锁定基础镜像」部分转移到本地
工具链版本——`Cargo.lock`/`package-lock.json` 仍然锁定依赖版本，脚本只依赖
`cargo`/`npm` 基本命令。

方案细节：脚本组装 `mktemp` 临时上下文而非直接对仓库根目录 `docker build`，
避免把 `target/`、`node_modules/`、`docs/`、`harness/` 送进构建上下文，也让
`docker/Dockerfile` 只认识打包所需的最小文件集。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-image-local-build | 新增 `build-docker.sh`：校验工具链、本地编译 server 与 admin-web、组装最小上下文、`docker build -t ${IMAGE_TAG:-filehub:dev}`、trap 清理 | 根目录脚本；不改业务源码 | 本地编译换镜像内无工具链；可复现性依赖本机 cargo/npm 版本（依赖仍由锁文件锁定） | 脚本在 stub docker 下完整跑通：两级构建产物生成、上下文包含二进制/dist 且无 target/node_modules、docker 收到正确 tag | 不提供 Windows 原生批处理 |
| P-002 | fh-image-runtime-image | 新增纯运行时 `docker/Dockerfile`（无 Rust/Node 阶段），移除根目录多阶段 Dockerfile/.dockerignore | docker/Dockerfile + 根目录旧文件移除 | 镜像不能独立编译，必须先用本地脚本；换来更小镜像与更清晰的构建位置 | `docker/Dockerfile` 无 `FROM node`/`FROM rust` 阶段且 COPY 路径与上下文布局一致；旧根目录 Dockerfile 移除 | 不做多阶段缓存优化 |
| P-003 | fh-image-docs | README 构建说明改为 `./build-docker.sh`（含 IMAGE_TAG），运行/卷挂载示例不变；变更记录+完成报告按 standard 补齐 | docker/README.md + docs/changes/019-image-local-build.md | 文档与脚本一致，避免误用旧 `docker build .` | README 示例可直接复制执行；`./build-docker.sh` 与 README 提及的文件路径一致 | 不写独立 CI 文档 |

## Success Criteria

- 用户可见结果：
  - 在本地开发机执行 `./build-docker.sh` 后得到 `filehub:dev` 镜像；
  - 镜像内不再有 `rust:`/`node:` 编译阶段，只有 nginx 运行时与打包产物；
  - `docker run -p 8080:80 -v <目录>:/data -e FH_SESSION_KEY=... \
    -e FH_ADMIN_PASSWORD=... filehub:dev` 启动行为与 018 一致。
- 必需证据（按本环境能力取可执行集）：
  - `sh -n build-docker.sh` 通过；
  - stub docker 下完整执行脚本：`cargo build --release -p filehub-server`
    与 `VITE_API_BASE_URL=/ npm run build` 真实通过，临时上下文内容核对
    （server 二进制、dist 文件、entrypoint/nginx 模板齐全，无构建缓存）；
  - `docker/Dockerfile` 语法与 COPY 路径静态核对；README 与脚本一致；
  - 若环境提供 Docker 守护进程：真实镜像构建 + 容器启动冒烟并记录结果。
- 明确非目标：不改运行期行为、不做多架构/推送/CI。

## Risks

- 本机工具链差异（低）：镜像可复现性依赖本地 `cargo`/`npm` 版本；依赖版本仍由
  Cargo.lock/package-lock 锁定，脚本检查命令存在，README 提示使用与仓库一致的
  工具链（或后续可选加 `rust-toolchain.toml`）。
- 误用旧构建入口（低）：移除根目录多阶段 Dockerfile/.dockerignore，并用
  README/变更记录指明唯一入口为 `./build-docker.sh`，避免 `docker build .`
  （根目录构建将失败并有明确提示，因为根目录不再放 Dockerfile）。
- `.env.local` 泄漏（低）：本地 `npm run build` 会读取 `admin-web/.env.local`
  （当前指向 127.0.0.1:8080）；脚本以 `VITE_API_BASE_URL=/` 显式覆盖构建参数，
  且产物只复制 `dist/`，不复制 env 文件。
