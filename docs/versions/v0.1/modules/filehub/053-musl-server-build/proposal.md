---
task_manifest: task.yaml
status: approved
---

# filehub-server Linux 构建改用 musl 工具链，修复 Alpine 镜像无法启动问题

Risk profile: not-created（待确认层级后再决定；standard/trivial 不创建）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 根因与方向已由用户溯源并指定：本地与 CI 都在 glibc Linux 上
    `cargo build --release`，产物动态链接 `/lib64/ld-linux-x86-64.so.2`，
    而 `docker/Dockerfile` 基于 `nginx:1.27-alpine`（musl libc），仅安装 jq，
    无 glibc loader 或 gcompat，容器启动 filehub-server 时必然
    `not found`；修复方向为 Linux 版本改用 musl 工具链编译；
  - 不满足 trivial：本次直接改变「产物链接形态 + 镜像构建/CI 运行验证」，
    属于 produced artifact / release、deployment、runtime integration 实质
    影响面，需要不低于 standard 的变更记录与比例化缺陷发现；
- 不触发 high-risk：无 schema/迁移、无安全/隐私边界、无 API/CLI 契约
    变化、无跨项目/架构边界；改动的构建目标与路径是确定的，回滚等于撤销
    脚本/CI 单一改动并重新构建，无需对外协调；与 019/020/052 等构建/发布面
    任务的 standard 先例一致。
- Proposal and tier confirmation: 用户 2026-08-26 回复「确认」，确认采纳本
  提案默认范围（仅 filehub-server 切 musl、Linux CLI 保持 glibc 原生构建、
  保留 CI docker run 冒烟）与 standard 层级。

## Approval Record

- approver: 用户
- approval_date: 2026-08-26
- user_statement: 用户 2026-08-26 回复「确认」，采纳默认范围与 standard
  层级；两个默认项一并确认：server-only（Linux CLI 不切 musl）、CI 增加
  docker run + /healthz/进程探活冒烟。

## Background and Goal

用户评审项「2. 高危：构建出的 Docker 镜像无法运行服务端二进制」确认：

- `build-docker.sh:18` 与 CI `build` job 都在 glibc Linux 上执行普通
  `cargo build`；
- 本地实际产物已核实为 glibc 动态链接：interpreter
  `/lib64/ld-linux-x86-64.so.2`、依赖 `libc.so.6`；
- `docker/Dockerfile:4` 仅 `apk add jq`，镜像内没有 glibc loader 或
  gcompat，因此即使 artifact 名称修好，容器启动 `filehub-server` 仍会报
  `not found`；
- 当前 CI `build-image` 只执行 `docker build` 与 `docker image inspect`，
  没有真正 `docker run`，同类问题不会被 CI 拦下。

目标：Linux 服务端改用 `x86_64-unknown-linux-musl` 工具链编译（静态 musl
链接，不再依赖 glibc loader），使 `nginx:alpine` 镜像可以直接执行
filehub-server；同时让 CI 真正启动一次镜像并探测 `/healthz`，把
「容器能跑起来」变成回归防线。

当地探针（只读验证）：`cargo build --release -p filehub-server --target
x86_64-unknown-linux-musl` 在当前环境完整通过，产物为 `static-pie linked`
（musl 静态链接，不再依赖 `/lib64/ld-linux-x86-64.so.2`）；本机未装
musl-gcc 也能链接成功，但构建前置要求仍显式声明 musl 目标与可选 musl-gcc
（部分依赖 C 编译/链接需要），以保证可复现。

## Scope

### In scope

- `build-docker.sh`：filehub-server 改用
  `cargo build --release -p filehub-server --target x86_64-unknown-linux-musl`，
  复制路径改为 `target/x86_64-unknown-linux-musl/release/filehub-server`，
  并在前置检查中给出 musl 目标/工具链缺失时的明确报错；
- `.github/workflows/build.yml`：
  - Ubuntu/Linux 矩阵安装 `x86_64-unknown-linux-musl` 目标与 musl C 工具链
    （`musl-tools`，提供 musl-gcc），server 构建与上传步骤切换到
    `target/x86_64-unknown-linux-musl/release/filehub-server` 路径；artifact
    名保持 `filehub-server` 不变，所有下载端（build-image/release）无需改动；
  - `build-image` 增加真实 `docker run` 冒烟：启动容器后等待
    `http://127.0.0.1/healthz` 可访问、失败时输出 `docker logs`，从而覆盖
    loader/runtime 兼容性；
- `docker/README.md`：构建章节补充 musl 目标与 musl C 工具链前置要求，
  说明镜像内无需 gcompat/glibc loader。
- 任务本地 `completion-report.md` 与 `docs/changes/053-musl-server-build.md`
  变更记录（standard 交付件）。

### Out of scope

- 不修改 `docker/Dockerfile`（保持 `nginx:1.27-alpine` 纯运行时、仅 jq、
  COPY 布局不变；musl 二进制不再需要 gcompat）；
- 不切换到 glibc 多阶段镜像、不添加 gcompat，除非 musl 链接被证实不可行；
- 不改变 macOS/Windows 矩阵构建与 CLI 发布路径；
- Linux CLI（`filehub-cli-linux-x86_64` release artifact）默认保持 glibc
  原生构建，除非用户确认一并对 Linux CLI 切 musl；
- 不改 artifact 名、Release 四件资产名、镜像 tag、触发策略与 GHCR 发布逻辑；
- 不触碰其他在制任务的未提交改动，不做仓库级格式化。

### Boundary with neighboring modules

- 改动局限在构建入口（`build-docker.sh`）、CI（`.github/workflows/build.yml`）
  与 docker 构建说明；server/admin-web/cli 源码、API 契约、数据布局与容器
  环境变量语义不变。
- `filehub-server` 打包进镜像的路径与形态变化只影响镜像构建链，release
  服务端归档里同样是 musl 产物，CLI 不参与镜像。

## Requirement Review

- 需求合理：musl 静态链接与 Alpine 基础镜像天然匹配，是消除
  「glibc loader 缺失」的根治方案；保持「本地/CI 编译、镜像内只打包」的
  既有 019 设计比引入多阶段编译更小。
- 关键权衡 1（C 工具链）：server 依赖 sqlx-sqlite（bundled libsqlite3）与
  ring/jsonwebtoken 链路，musl 目标编译通常需要 `musl-gcc`（Debian/Ubuntu
  的 `musl-tools`）。本机 `cargo check` 已通过；若完整链接证明当前环境缺
  linker，则把「安装 musl-tools」写入 CI 与本地前置要求，而不是改链接方案。
- 关键权衡 2（CI 冒烟）：现有 CI 只 build+inspect，无法发现
  loader/entrypoint 运行时失败；新增 `docker run` + `/healthz` 探测把本次
  缺陷变成可回归验证，成本仅一个 job 步骤。
- 关键权衡 3（CLI 平台范围）：用户指令字面是「linux 版本」，提案默认最小
  范围只切镜像实际使用的 filehub-server；若希望 Linux CLI 发布物同样 musl，
  需用户在确认时一并说明。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-musl-server-build | Linux filehub-server 改用 `x86_64-unknown-linux-musl` 编译并打包进 Alpine 镜像，无需 glibc loader | 仅 `build-docker.sh`、CI Linux server 构建/上传路径、docker 构建说明 | musl 构建要求 `musl-tools`（musl-gcc）；产物为静态 musl | `file` 显示 static-pie linked（`ldd` 为 statically linked）；`build-docker.sh` 与 CI 命令一致；本地 stub build 流程通过 | 不改 Dockerfile 基础镜像；不加 gcompat；不改 macOS/Windows/CLI 构建与资产名 |
| P-002 | fh-image-smoke-run | CI 构建镜像后真实启动并探测 `/healthz`，失败输出 `docker logs` | 仅 `build-image` job | 增加一次容器运行时间，换取 loader/runtime 回归拦截 | workflow YAML 通过；冒烟步骤在可用 Docker 环境（托管 runner/本机）跑通 | 不做端到端 API 行为测试；不影响已有 image inspect 与 GHCR push |

## Success Criteria

- 系统可见结果：Linux server 产物为 musl 静态链接（`file` 不再显示
  `/lib64/ld-linux-x86-64.so.2`）；Alpine 镜像内 `/usr/local/bin/filehub-server`
  可执行并配合 entrypoint 正常启动；CI 的 `docker run` 冒烟通过。
- 所需证据：`file target/x86_64-unknown-linux-musl/release/filehub-server`
  为 statically linked；`sh -n build-docker.sh` 与 stub-docker 流程通过；
  build.yml 经 YAML 解析/actionlint（如可用）与上传→下载→运行路径映射核对；
  Docker 可用的机器执行镜像 `docker run` 冒烟（托管 runner 或本地）。
- 显式非目标：不改容器内服务端端口/配置/数据语义；不重命名 artifact 与
  Release 资产；不把 CI 冒烟扩展为完整 API/上传测试；不以本地代码检查替代
  真实 `docker run` 结果（本环境无 Docker 守护进程时，以托管 runner 为准）。

## Risks

- musl 工具链依赖：musl 目标是新增的构建前置条件（本地缺目标时脚本给出明确
  报错，CI 显式安装）；本机验证 musl-gcc 缺失仍可链接，但 CI 仍安装
  `musl-tools` 保证 sqlx/ring 等 C 依赖编译环节可复现，不回退为「悄悄用
  glibc 编译」。
- 依赖链接兼容：若 sqlx/ring 在发行版 musl 工具链上出现新链接错误，问题会
  集中在链接阶段且可在 CI 提前暴露；备选是文档化安装正确版本 musl-tools，
  不改镜像为 glibc。
- 平台差异：Linux 产物从 glibc 变为 musl 后，使用方若在 glibc-only 环境以
  dlopen 方式加载服务器二进制（当前不存在该用法）会受影响；server 自身不
  依赖 glibc 扩展接口，静态链接反而提高可移植性。
- 共享工作树：仓库有多个在制任务的未提交改动，执行时先记录 pre-edit 基线，
  只触碰本任务范围文件，避免仓库级格式化或覆盖其他任务改动。
