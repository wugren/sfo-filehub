---
task_manifest: task.yaml
status: approved
---

## Approval Record

- approver: user
- approval_date: 2026-08-22
- user_statement: 确认（2026-08-22 用户回复「确认」，采纳方案乙 + standard
  层级：单一 build 矩阵 job 编译 server 与三平台 CLI，release 单 job
  一次性发布 server 与三平台 CLI）

# GitHub Actions：CLI 三平台编译并入同一构建 job，server 与 CLI 一次性发布

Risk profile: not-created（确认层级为 trivial/standard 时不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 不满足 trivial：本任务改变 GitHub Actions 的 job 结构（合并 Rust 编译
    job），并承诺「server + 三平台 CLI 一次性发布」的 Release 交付面，属于
    produced artifact 与 release/deployment 影响面，不符合 trivial 的
    「无材料影响」条件；
  - 未升级 high-risk：只改 `.github/workflows/build.yml` 的 job 编排与依赖，
    不改 server/admin-web/cli 源码、测试、依赖、数据 schema、API 契约，不引入
    外部凭据，版本/仓库发布门控、镜像与 Release 语义保持不变，与 020 同为
    构建/发布编排面，按 standard 处理。
- Proposal and tier confirmation: 用户于 2026-08-22 回复「确认」，采纳默认
  方案乙（单一 build 矩阵 job + release 单 job 一次性发布）与 standard 层级；
  本提案置为 `status: approved`，任务按 standard 层级执行。

## Background and Goal

用户最新口径：
1. CLI 必须支持三平台（linux/macos/windows 原生编译），不允许合并后只剩
   Linux；
2. 发布必须「一次性发布 CLI 和 server」：一次 v 标签发布同时产出 server
   归档与三平台 CLI 归档，进入同一个 GitHub Release。

当前 build.yml 已具备「单一 release job 一次性发布 server + 三平台 CLI」的
结构（release job 下载 filehub-server、web-dist 与
`filehub-cli_<version>_<平台>.tar.gz` 三个归档，一次 `gh release create/
upload` 发布四件归档）。本任务在此基础上把 Rust 编译侧也收敛为同一个
`build` 矩阵 job，并把「一次性发布」作为明确交付验收点保留下来。

## Scope

### 默认方案（方案乙：单一 `build` 矩阵 job + 单 job 一次性发布）

1. 删除独立 `test-rust` 与 `build-cli` 两个 job，合并为单一 `build` job，
   使用原有三平台矩阵：
   - linux-x86_64（ubuntu-24.04）：`cargo test --workspace --all-targets`
     + `cargo build --release -p filehub-server` +
     `cargo build --release -p filehub-cli`；
   - macos-aarch64（macos-14）：`cargo build --release -p filehub-cli`；
   - windows-x86_64（windows-2022）：`cargo build --release -p filehub-cli`；
2. server 二进制与 CLI 归档上传逻辑沿用现有约定：
   - server 二进制文件 `target/release/filehub-server` 在非 PR 运行上传
     （`archive: false`，实际产物名 `filehub-server`）；
   - CLI 在 v 标签触发时打包并上传
     `dist/filehub-cli_<version>_<平台>.tar.gz`；
3. `release` job 保持为唯一发布入口并同步依赖：
   - `needs` 中 `test-rust`/`build-cli` 替换为 `build`；
   - 仍一次下载 server 归档、web-dist 与三平台 CLI 归档，打包 server+web
     归档，用一个 `gh release create/upload` 同时发布四件交付物；
   - 产物校验与 Release 说明保留三平台 CLI（4 件 tar.gz），不缩减为 2 件；
4. `version`、`test-web`、`build-image` job 保持不变；`build-image` 的
   `needs` 由 `test-rust` 改为 `build`。

### Out of scope

- 不修改 server/admin-web/cli 任何源码、测试、Cargo.toml/Cargo.lock、
  package.json/package-lock.json 或运行时行为；
- 不修改镜像构建上下文、Dockerfile、GHCR 发布门控与镜像 tag 规则；
- 不修改版本门控（v 标签 == workspace 版本）与 canonical 仓库门控；
- 不做交叉编译（本节方案丙说明，不在默认范围）；
- 不修改 `version`、`test-web` job 的步骤内容。

### Boundary with neighboring modules

只改 `.github/workflows/build.yml`。CLI 三平台发布面（020 确认的
linux/macos/windows 三份 CLI 归档）保持不缩；server 仍只构建 Linux x86_64。
「一次性发布」以 release job 的单一 `gh release create/update` 动作验收。

## Requirement Review

- 需求合理：三平台 CLI 是已确认的产品发布面；「一次性发布 CLI 和 server」
  要求发布动作单一、产物齐全，当前 release job 已满足该意图，本任务把
  Rust 编译收敛为单一 `build` 矩阵 job，并保留该发布链路作为验收点。
- 关键工程技术约束：GitHub Actions 一个 job 只能运行在一种 runner OS 上，
  三个平台的 CLI 只能通过矩阵（多个 runner）实现原生编译。因此：
  - 方案乙（默认）：单一 `build` job 定义 + 三平台矩阵，运行列表仍会显示
    三个矩阵子 job（linux/macos/windows），这是原生三平台的必要代价；
  - 方案丙：要求运行列表只有一个 job 且能出三平台二进制，只能做单 job
    交叉编译（Linux 上交叉编译 macOS/Windows 需要 osxcross/mingw/xwin 等
    工具链），复杂度与失败面显著增大，不适合本任务；
- 建议：默认采用方案乙；若用户真正要求「运行里只有一个 job」，再单独评估
  方案丙。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-consolidate-rust-build | test-rust 与 build-cli 合并为单一 build 矩阵 job：linux 条目测试并编译 server+CLI，macos/windows 条目编译 CLI；上传逻辑不变 | 只改 .github/workflows/build.yml | 编译侧集中为一个 job 定义；运行列表仍按平台显示三个子 job | YAML 可解析；build.yml 中不再存在 test-rust/build-cli job；三平台矩阵与上传条件保留 | 不改源码/依赖；不做交叉编译 |
| P-002 | fh-one-shot-release | release job 依赖与产物校验对齐 build job；保留一次 gh release 发布 server+web 与三平台 CLI 共四件归档 | 只改 .github/workflows/build.yml | 发布动作单一，产物齐全；下载/校验引用需与 build 上传名一致 | needs 替换正确；四件归档校验与 gh release 附件引用一致；v 标签发布产物完整 | 不缩减 CLI 平台；不改镜像/GHCR/门控 |

## Success Criteria

- 用户可见结果：
  - push/PR 到 main：Actions 运行只出现一个 Rust 编译 job（`Build`，按
    三个平台子 job 展开），不再出现 `test-rust` 与 `build-cli` 两个独立 job；
  - 推送 v 标签：release job 一次创建/更新同一个 GitHub Release，附件为
    `filehub-server_<version>_linux_x86_64.tar.gz` 与三平台
    `filehub-cli_<version>_<os>_<arch>.tar.gz` 共四件，GHCR 镜像照常发布。
- Required evidence:
  - `python` YAML 解析通过；
  - job 依赖图核对：build-image/release 的 `needs` 指向 `build`；
    上传/下载产物名映射（server：`filehub-server`；CLI：
    `filehub-cli_<version>_<平台>.tar.gz`）一一对应；
  - 托管 runner 真实运行结果（下一次 main push 或 v 标签触发后确认，与
    020/021 相同的证据边界）。
- Explicit non-goals:
  - 不做 Linux-only CLI（用户已明确三平台保留）；
  - 不做交叉编译（默认方案）；
  - 不改变测试命令、镜像构建与发布门控。

## Risks

- 矩阵子 job 仍是三个：若用户接受「单 job 定义 + 矩阵展开」，运行列表
  依然显示三个平台子 job，这是原生三平台编译的固有形态；若必须单 job
  单 runner，则只能转向方案丙交叉编译（本任务范围外）。
- 上传/下载一致性：合并后 release 与 build-image 的 `needs` 以及产物名
  引用必须同步更新，否则 tag 发布会失败；限于同一工作流内，可静态核对。
- 托管运行确认：本环境无法真实触发 hosted runner，最终以上传仓库后的运行
  记录为准。

## 未决问题（需要用户在确认时一并回答）

1. 编译侧结构采用哪个方案？
   - 方案乙（默认）：单一 `build` 矩阵 job（三平台原生 CLI，运行列表仍为
     三个平台子 job），release 单 job 一次性发布 server + 三平台 CLI；
   - 方案丙：单 job 单 runner 交叉编译三平台 CLI（工具链复杂，另行评估）；
   - 若两个方案都不接受，请说明期望的 job 形态。
