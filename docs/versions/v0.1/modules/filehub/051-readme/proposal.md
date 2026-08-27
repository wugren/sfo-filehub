---
task_manifest: task.yaml
status: approved
---

# 生成项目根 README 文档

Risk profile: not-created（仅 high-risk 确认后替换为 ./risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: trivial
- Final tier: trivial
- Tier rationale / triggered boundaries:
  - 纯文档交付：在仓库根目录新增一个 `README.md`，不改代码、配置、API 契约、
    数据、构建/依赖、发布产物或运行时行为；
  - 内容来源是仓库现有权威材料（`docs/modules/filehub.md`、
    `docs/api/v1-contract.md`、`cli/README.md`、`docker/README.md`、工程配置），
    不新增或更改任何长期文档的意图描述；
  - 影响局限为一个根级文件，存在明确的目标验证信号（渲染正常、内部链接可点、
    模块/命令/环境变量描述与既有文档一致），满足 trivial 判定条件；
  - 不触发 standard/high-risk：无多模块实现面改动、无 governed intent 变更、
    无迁移/安全/架构边界影响。
- Proposal and tier confirmation: 用户 2026-08-26 前四轮「确认」分别批准首版、
  「使用优先」、「基于 GitHub Actions 发布产物」与「GitHub 用户名实写
  wugren」修订，层级均为 trivial；2026-08-26 用户提出「服务端配置应该通过
  配置文件来控制，文档中不要包含环境变量的方式」，本提案回到 draft 后，用户
  回复「确认」批准本轮范围：根 README 服务端配置一律以配置文件描述，移除
  FH_*/FH_CONFIG 等环境变量配置形式，层级维持 trivial。

## Approval Record

- approver: 用户
- approval_date: 2026-08-26
- user_statement: 2026-08-26 五轮确认均为 trivial 层级；末轮确认：用户「确认」
  批准根 README 移除服务端环境变量配置方式、仅保留配置文件方式，范围仍仅限
  README.md，不改 entrypoint/镜像行为与 docker/README.md。

## Background and Goal

- 仓库经过多轮任务（001-050）已形成三个完整交付面（filehub-server、
  filehub-web、filehub-cli）与 Docker 部署路径，但根目录没有面向使用者/维护者
  的 README，入口信息分散在 `docs/modules/filehub.md`、`cli/README.md` 与
  `docker/README.md`。
- 目标：根 `README.md` 的使用指引**以 GitHub Actions 发布之后的方式**为主线：
  普通使用者从 GitHub Release 下载已构建归档、或拉取 GHCR 镜像直接运行
  （服务端 + 管理后台 + CLI 三种入口），而不是从源码构建；源码构建仅保留为
  开发者/贡献者路径。

## Scope

### In scope

- 根 `README.md`（中文）**以 GitHub Actions 发布产物为使用入口**：
  - 发布机制说明：`.github/workflows/build.yml` 在推 `v*` 标签时执行完整构建、
    测试并发布——GitHub Release 附带 4 个归档（`filehub-server_<ver>_linux_x86_
    64.tar.gz`、`filehub-cli_<ver>_{linux-x86_64,macos-aarch64,windows-x86_64}
    .tar.gz`），同时推送 GHCR 镜像 `ghcr.io/wugren/filehub:v<version>`
    （用户名已由用户确认：仓库 remote 为 `github.com/wugren/sfo-filehub`）；
    手动触发 `workflow_dispatch` 只构建并上传临时 artifact，不发布；
  - **快速开始（发布产物路径）**：
    - CLI：从 GitHub Release 下载对应用户平台的归档，解压得到 `filehub`/
      `filehub.exe` 后直接 `login`/`push`/`pull`；
    - 服务端+管理后台：下载 `filehub-server_<ver>_linux_x86_64.tar.gz`
      （内含 server 二进制与 admin-web 静态文件）配置运行，或直接
      `docker pull ghcr.io/wugren/filehub:v<version>` 后 `docker run`；
  - GitHub 用户名写实：Releases 页面链接为
    `https://github.com/wugren/sfo-filehub/releases`；当前仓库尚无 `v*` tag 与
    Release，README 以 `v<版本>` 占位并在发布说明注明首次发布后生效；
  - **服务端配置一律通过配置文件**：README 中服务端配置只描述
    `filehub-server.json`（`server/config.example.json` 为示例，运行参数为
    配置文件路径），不出现 `FH_*`、`FH_CONFIG` 等服务端环境变量配置方式；
    Docker 快速开始移除 `-e` 参数与服务端环境变量表，保留卷挂载并链接
    `docker/README.md` 作运维细节；CLI 段落移除 `FILEHUB_*` 显式清单，只保留
    `cli/README.md` 链接；
  - **使用操作**（与已确认「使用优先」一致）：Web 后台登录→建项目→建版本→
    上传 app→下载/锁定；CLI `login`→`new-version`→`push`→`versions`→`pull`→
    `lock-version` 示例工作流，命令与现有 `cli/README.md` 契约一致；
  - 源码构建（`cargo build`、`npm run dev/build`）移到「开发者/本地构建」小节，
    定位为从源码运行或贡献时使用；
  - 测试、配置/安全提醒与文档索引保留在后方，内容摘要级并链接权威文档。

### Out of scope

- 不修改任何代码、配置、脚本、CI、既有 sub-README 或 `docs/` 文档内容；
- 不写未来才具备的能力（如尚未实现的部署平台/协议支持）；不新增 release 下载
  链接占位或无法验证的 URL；GitHub 用户名 `wugren` 已确定为实际值，不再使用
  占位符；
- 不改 `docker/entrypoint.sh` 或镜像行为：容器入口仍以环境变量生成服务端配置
  是既有实现，`docker/README.md` 运维说明保持不动，根 README 只是不再重复
  展示该方式；
- 不把未完成任务或在制工作树的临时状态写成正式能力；
- 不执行仓库级格式化或触碰其他在制任务的未提交改动。

## Requirement Review

- 需求合理：产品已有 `build.yml` 的发布链路，普通使用者的正确入口是 CI 发布
  产物；README 以源码构建为主会引导用户做不必要的本地编译。
- 方向选择：使用章节以「GitHub Release 归档 + GHCR 镜像」为主入口，发布机制
  按 `.github/workflows/build.yml` 实际行为描述（tag 触发发布、手动触发仅
  构建 artifact），CLI 操作示例仍逐条对照 `cli/README.md` 与 v1 API 契约。
- 方向补充（第五轮）：服务端配置在 README 中只讲配置文件
  （`filehub-server.json` / 命令行参数），环境变量配置形式从根 README 移除，
  避免把入口脚本的转换细节暴露为使用方式。
- 偏差权衡：README 是快照，且 CI 发布物仍需真实发布一次才会存在；文档按
  build.yml 声明的产物命名与镜像名描述，并以 `v<version>` 形式给出，
  不硬编码尚未验证的 URL。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-root-readme | 根 `README.md` 的使用说明以 GitHub Actions 发布产物为主：从 GitHub Release 下载 CLI/服务端归档、拉取 GHCR 镜像运行，再给 Web/CLI 操作示例；源码构建只保留为开发路径；服务端配置一律以 `filehub-server.json` 配置文件描述，README 不出现 FH_*/FH_CONFIG 等服务端环境变量配置方式 | 仅新增根级 `README.md`（覆盖前四版交付物）；不改动代码/配置/CI/既有文档与 docker/README.md | 发布产物依赖 CI 实际发布动作，README 只能按 build.yml 声明的命名/镜像描述；容器入口的环境变量机制属既有实现，根 README 不展示但保留 docker/README.md 链接 | README 可正常渲染；Release 产物命名、GHCR 镜像名、触发条件与 `build.yml` 完全一致；全文不含服务端环境变量配置说明；CLI/Web 示例与 `cli/README.md`、v1 契约一致；「下载→运行→发布→下载」路径可照做 | 不改 CI/入口脚本/镜像行为、不加不可验证 URL、不重写现有 README |

## Success Criteria

- 根目录 `README.md` 的使用章节以 CI 发布产物为入口占主要篇幅：Release 归档
  (4 个) 与 GHCR 镜像的获取/运行方式均与 `build.yml` 一致，Web/CLI 有可照做
  示例；源码构建降级为开发小节；
- 根 `README.md` 全文不再出现 `FH_*`、`FH_CONFIG`、`FILEHUB_*` 等服务端/客户端
  环境变量配置清单；服务端配置只通过配置文件描述；
- 内容中每个仓库路径、命令、环境变量、配置字段和文档链接均能在当前仓库验证
  存在/正确；
- 不产生本任务之外的任何文件改动（`git status` 中 README 之外的新增/修改仍只
  来自其他在制任务）；
- 按 trivial 流程输出 `completion-report.md`，`lower-tier-check.py --profile
  completion` 校验通过。

## Risks

- 工作树含大量其他任务的未提交改动：本任务只新增根 README，不修改共享文件，
  changed-path 应仅为 `README.md`；
- 使用示例必须与真实 CLI/API 语义一致：逐条对照 `cli/README.md` 命令面与
  `docs/api/v1-contract.md`，命令参数若与实际不符会误导使用者；
- CI 发布链路 BUG 风险：`build.yml` 上传 server artifact 名称为
  `server-binary`，但 build-image/release 下载名称为 `filehub-server`，写 README
  时按设计意图描述发布产物，并在完成报告记录该不一致，不在本任务修复 CI；
- 容器入口的既有环境变量机制与「文档不展示环境变量」之间的落差：根 README
  只保留 docker/README.md 链接、不展开环境变量细节，避免与用户要求冲突；若
  后续需要容器直接挂载配置文件，应另开任务修改 entrypoint；
- 内容快照可能落后于后续行为演进：README 明确指向权威文档，降低漂移影响；
- README 若包含不准确的环境变量或 CLI 语义会造成误导：实施时逐项核对
  `docker/entrypoint.sh`、`cli/README.md` 与 `server/config.example.json`。
