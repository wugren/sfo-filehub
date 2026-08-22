---
task_manifest: task.yaml
status: approved
---

## Approval Record

- approver: user
- approval_date: 2026-08-22
- user_statement: 确认（2026-08-22 用户回复「确认」，按修订后标准级提案执行；v
  标签发布 = GHCR 镜像推送 + GitHub Release 附件）

# GitHub Actions 单工作流：编译、Docker 镜像与 v 标签发布

Risk profile: not-created（standard 层级不创建 risk-profile；若用户选择或升级
high-risk 则补充 risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 不满足 trivial：本任务新增构建/发布工作流，属于构建产物与发布交付面
    （produced artifacts、Docker 镜像发布、GitHub Release），并涉及
    GITHUB_TOKEN 权限与跨平台构建矩阵，不满足 trivial 的「无材料影响」条件；
  - 未升级 high-risk：按后果而非文件类型分类——本任务不改 server/admin-web/cli
    源码、测试、依赖、数据 schema 或 API 契约，不引入外部密钥/凭据（镜像与
    Release 均只用仓库内置 GITHUB_TOKEN + tag 门控），发布只有 canonical 仓库
    推送 `v*` 标签时触发、可重跑且不产生不可逆迁移；仓库内 018/019 镜像构建
    交付面同类构建工作及既有仓库的 Actions 发布工作均按 standard 完成；
  - 若用户认为「自动化发布到 GHCR/GitHub Releases」本身已构成材料级发布面
    影响，可在确认时替换为 high-risk（补 risk-profile 与 design/testing/
    acceptance 全流程）。
- Proposal and tier confirmation: 2026-08-22 用户给出范围修订：「一个工作流，
  同时编译 CLI 与 server 并生成 Docker 镜像，v 标签触发编译+发布」，本提案按
  该口径更新回 draft；同日用户回复「确认」，最终层级 standard，提案 approved。

## Background and Goal

用户要求为 sfo-filehub 新增一个 GitHub Actions 工作流：在 Actions 中统一完成
编译（Rust workspace 的 CLI 与 server、admin-web）并生成 Docker 镜像；推送
`v*` 标签时触发编译与发布（GHCR 镜像 + GitHub Release 产物）。当前仓库没有
任何 `.github/workflows/` 配置，只有本地构建入口 `build-docker.sh` 与
test-run 脚本；本任务补齐 CI 编译、镜像构建与标签发布能力，保持仓库已有
「发布统一 `.tar.gz`」的产品约定（见 001 提案与 docs/modules/filehub.md）。

## Scope

### In scope

1. 新增唯一工作流 `.github/workflows/build.yml`，触发方式：
   - `push` 到 `master`：编译 + 测试 + 生成 Docker 镜像（不推送）；
   - `pull_request`：编译 + 测试（快速门禁，不构建镜像）；
   - `push` `v*` 标签（`GITHUB_REF_TYPE == tag`）：编译 + 测试 + 生成 Docker
     镜像并发布。
2. 编译与测试（复用仓库现有本地命令）：
   - Rust job：安装 stable 工具链，`cargo test --workspace --all-targets`；
     release 构建 `filehub-server`（Linux runner）与 `filehub-cli`
     （linux/macos/windows 原生 runner 矩阵，对齐模块文档确认的跨平台 CLI）；
     cargo registry/git 缓存与 `CARGO_TERM_COLOR=always`；
   - admin-web job：`npm ci`（以 package-lock.json 为准）→
     `test:unit` → `test:integration` → `test:dv`（dv 含真实构建与 dist 校验）；
3. Docker 镜像（对齐 018/019 交付面）：
   - 在 Linux job 中按 build-docker.sh 的语义组装最小上下文（release server
     二进制 + admin-web dist + `docker/Dockerfile`/nginx/entrypoint），用纯运行时
     `docker/Dockerfile` 执行 `docker build`，不做镜像内编译；
   - 镜像 tag：`ghcr.io/{owner}/filehub:v{version}`（`{version}` 来自根
     Cargo.toml workspace 版本，v 标签必须一致；不自动打 `latest`，避免覆盖
     已发布镜像）；
   - `master` 只构建验证镜像可生成；`v*` 标签时推送 GHCR
     （`packages: write`，仅 GITHUB_TOKEN，无外部凭据）。
4. v 标签发布（`GITHUB_REF_TYPE == tag` 且 `GITHUB_REF_NAME == v{version}`）：
   - 版本门控：`v<tag>` 必须等于根 `Cargo.toml` 的 workspace 版本，不一致立即
     失败且不发布；
   - 仓库门控：仅 canonical 仓库执行 GHCR push 与 GitHub Release 创建；
     fork/非目标仓库只构建与上传 Actions Artifacts；
   - 产物（统一 `.tar.gz`，不引入其它归档格式）：
     - `filehub-server_{version}_linux_x86_64.tar.gz`（server 二进制 +
       admin-web dist）；
     - `filehub-cli_{version}_{os}_{arch}.tar.gz`（三平台 CLI）；
   - Actions Artifacts 以 `archive: false` 直接上传文件；GitHub Release 通过
     `contents: write` 附加全部校验后的产物。
5. 定向验证（本环境能力范围内）：YAML 语法解析、workflow 引用的命令与本地
   构建命令逐一核对、镜像上下文/Dockerfile COPY 路径与打包结构核对；hosted
   runner 真实运行留待推送仓库后执行（与 019 相同的证据边界，确认时如实告知）。

### Out of scope

- 不修改 server/admin-web/cli 任何源码、测试、Cargo.toml/package.json、
  Cargo.lock/package-lock.json 或运行时行为；
- 不改动 `build-docker.sh` 与 `docker/` 现有镜像构建/运行语义（工作流只复用其
  产物组装方式与 Dockerfile）；
- 不做自动版本号 bump、CHANGELOG 生成、代码签名/公证；
- 不新增外部密钥（npm token、Docker registry 凭据、签名密钥等）；GHCR 只用
  GITHUB_TOKEN；
- 不做 server 的 Windows/macOS 构建（server 仅 Linux x86_64；Windows/macOS
  只构建 CLI）；不做多架构镜像与交叉编译；
- 不提供 workflow_dispatch 手动发布入口（v 标签推送 / Actions 重跑即可触发）；
- 不改动现有 `test-run.sh` / Harness 脚本。

### Boundary with neighboring modules

只新增 `.github/workflows/build.yml`：产品源码、本地构建入口（build-docker.sh）
与镜像运行面（018/019）保持不动。发布的打包格式沿用产品约定「统一
`.tar.gz`」；GitHub Release / GHCR 是仓库级交付面，与 filehub 产品内通过 CLI
发布的版本语义（项目/版本/不可覆盖）相互独立，不互相耦合。

## Requirement Review

需求合理：单工作流统一编译（CLI + server + admin-web）、生成镜像并由 v 标签
驱动发布，符合「一个配置文件管编译与发布」的直觉，也补齐了仓库目前缺失的 CI
与 tag 发布能力。主要权衡：

- 可复现性：Rust 依赖由 Cargo.lock 锁定、前端由 package-lock.json 锁定，
  Actions 只安装 stable 工具链与 Node/npm，命令与本地构建一致；
- 发布安全：v 标签必须等于 Cargo workspace 版本，仅 canonical 仓库发布；
  GHCR push 与 Release 创建只用 GITHUB_TOKEN，不引入外部凭据；
- 镜像一致性：镜像生成复用 018/019 的纯运行时 Dockerfile 与最小上下文，
  镜像内不安装编译链；标签用不可变的 `v{version}`，不自动覆盖；
- 门槛：与 019 一样，本环境无法真实触发 hosted runner，配置会做 YAML/命令/
  结构定向验证，真实构建与发布留待仓库推送后验证并在完成报告中注明。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-single-ci-rust | 唯一工作流 build.yml 的 Rust 部分：stable + cargo test --workspace --all-targets + release 构建 server（Linux）与 CLI（三平台矩阵） | 只写 .github/workflows/build.yml | 三平台矩阵并行但总耗时增加；换来跨平台 CLI 发布 | workflow 命令与本地一致；YAML 可解析；cargo cache key 覆盖 Cargo.lock | 不改 Cargo 配置/依赖 |
| P-002 | fh-single-ci-web | build.yml 的 admin-web 部分：npm ci + test:unit + test:integration + test:dv | 只写 .github/workflows/build.yml | npm ci 严格按锁文件，避免浮动依赖 | 四个 npm script 全部被调用，dist 校验通过 | 不改 package.json/锁文件 |
| P-003 | fh-single-docker-image | build.yml 镜像部分：最小上下文 + docker/Dockerfile 生成 ghcr.io/{owner}/filehub:v{version}；master 只构建，v 标签推送 GHCR | 只写 .github/workflows/build.yml | 镜像内容与 018/019 一致，CI 可直接复用；每次 master 构建镜像增加时长 | 镜像 tag 来自根 Cargo.toml 版本；master/tag 的 push 条件与上下文路径正确 | 不做多架构/本地 Dockerfile 修改 |
| P-004 | fh-single-release-publish | v 标签发布：v==workspace 版本门控 + canonical 仓库门控；统一 .tar.gz 产物上传 Actions Artifacts 并创建 GitHub Release | 只写 .github/workflows/build.yml | 门控稍复杂，发布可审计、可防误发 | 错误 tag/非 canonical 仓库在发布前失败；Release 含 server+CLI 六件 tar.gz | 不做 zip/其它格式、不做签名 |

## Success Criteria

- 用户可见结果：
  - push/PR 到 `master` 时，Rust 与 admin-web 测试/构建自动运行并给出结果；
  - 推送 `v0.1.0`（与根 Cargo.toml workspace 版本一致）时：GHCR 出现
    `ghcr.io/{owner}/filehub:v0.1.0`，GitHub Release 出现
    `filehub-server_0.1.0_linux_x86_64.tar.gz` 与三平台
    `filehub-cli_0.1.0_<os>_<arch>.tar.gz`；
  - 错误 tag（如 `v0.2.0` 与 Cargo 版本不符）在发布前失败，不产生镜像/Release。
- 必需证据（本环境可执行集）：
  - `.github/workflows/build.yml` YAML 语法校验通过（python yaml/actionlint
    可用时）；
  - workflow 内命令与本地等价命令逐一核对：`cargo test --workspace
    --all-targets`、`cargo build --release -p filehub-server -p filehub-cli`、
    `npm ci && npm run test:unit && npm run test:integration && npm run
    test:dv`；
  - 镜像上下文组装与 `docker/Dockerfile` COPY 路径、打包文件名/`--strip-
    components` 结构核对；
  - hosted runner 实跑与真实 GHCR/Release 发布：由仓库推送 tag 后验证，作为
    已知证据边界记录。
- 明确非目标：不改产品代码、不引入外部密钥、不做多架构/签名/自动 bump。

## Risks

- hosted runner 与本地工具链差异（中）：CI 首次实跑可能暴露 runner 上
  cargo/node 版本差异或 Windows 构建细节（本仓库已有 Windows 兼容修复先例）；
  以 Cargo.lock/package-lock + 固定 runner 镜像缓解，实跑结果待推送后记录。
- 发布误操作（中低）：tag 与 Cargo 版本不一致、fork/非 canonical 仓库
  publish 门控缺失都会导致误发布；本提案内置两道门控，GHCR/Release 由
  GITHUB_TOKEN 操作、可删除重跑。
- GHCR 权限（低）：`packages: write` 是发布 job 的最小权限，镜像发布使用
  GITHUB_TOKEN 而非 PAT/外部密钥；canonical 仓库需确认包写入权限已开启。
- 依赖缓存污染（低）：cargo/node 缓存命中错误版本可能掩盖依赖漂移；
  缓存 key 绑定锁文件 hash，CI 全量测试兜底。
