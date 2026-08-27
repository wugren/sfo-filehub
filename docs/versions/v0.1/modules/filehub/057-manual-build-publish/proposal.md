---
task_manifest: task.yaml
status: approved
---

# GitHub Actions 支持受控手工编译、打包与发布，并在 Rust 编译前更新依赖

Risk profile: ./risk-profile.yaml

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: high-risk
- Tier rationale / triggered boundaries:
  - 不满足 trivial/standard 默认边界：本任务让 `workflow_dispatch` 获得向 GHCR
    推送镜像并创建或更新 GitHub Release 的能力，直接影响 produced artifact、
    release/deployment 与 `GITHUB_TOKEN` 写权限执行面；
  - 用户新增“Rust 项目编译前必须 `cargo update`”后，每次 CI 会忽略提交时锁定的
    具体依赖版本并重新解析 Cargo 允许的最新兼容版本，发布二进制的依赖图可能随
    crates.io 状态变化，构成材料级 dependency/build graph 与 supply-chain trust
    边界，因此建议升级为 high-risk；
  - 通过单次解析并把同一份更新后 `Cargo.lock` 分发给全部 Rust 构建 job，限制
    同一次运行内的跨平台依赖漂移；但不同运行仍可能解析出不同依赖，这一残余风险
    需要在设计、测试与独立验收中显式检查。
- Proposal and tier confirmation: 2026-08-27 用户补充“另外rust项目编译前必须
  cargo update”，提案按共享更新后 lockfile 的方式修订；随后用户回复“确认，
  自动完成”，批准本提案与 high-risk 层级，并显式启动从 Design 开始的
  auto-pipeline，后续阶段无需逐阶段人工确认。

## Approval Record

- approver: 用户
- approval_date: 2026-08-27
- user_statement: “确认，自动完成”
- selected_tier: high-risk
- execution_mode: auto-pipeline（Launch stage: proposal；First auto stage: design）

## Background and Goal

当前 `.github/workflows/build.yml` 已支持 `workflow_dispatch`，人工运行会完成 Rust
workspace 测试、server/CLI release 编译、admin-web 测试与构建、三平台 CLI
`.tar.gz` 打包、server/web artifact 上传、Docker 镜像构建及容器冒烟；但
`version` job 只在 canonical 仓库的 `push v*` 事件中输出 `publish=true`，因此
人工运行不会推送 GHCR，也不会创建或更新 GitHub Release。

目标是在保留现有 tag 自动发布路径的同时，让操作者可以从 Actions 页面显式选择
“编译、打包并发布”；普通人工运行仍默认只构建，防止误点即发布。所有 Rust
测试和 release 编译开始前，CI 必须先执行一次 `cargo update`，并让同一次运行的
Linux、macOS、Windows 构建共享该次解析出的同一份 `Cargo.lock`。

## Scope

### In scope

1. 为 `workflow_dispatch` 增加受控输入：
   - `publish`：boolean，默认 `false`；只有显式设为 `true` 才进入发布路径；
   - `release_tag`：string；仅在 `publish=true` 时必填。
2. 在任何 Rust 测试或编译前集中更新依赖：
   - 在前置 job 中对本次构建的精确 `GITHUB_SHA` 执行 `cargo update`；
   - 把生成的 `Cargo.lock` 作为本次运行内的 artifact 分发给 Linux、macOS、
     Windows 的 Rust build matrix；每个 Rust job 必须先安装该 lockfile，再运行
     `cargo test` 或 `cargo build`，保证同一次运行三平台使用完全相同的解析结果；
   - 不把 CI 生成的 `Cargo.lock` 回写或提交到仓库，也不触碰当前工作树中已有的
     `Cargo.lock` 修改。
3. 统一人工发布与 tag 自动发布的门控：
   - `release_tag` 必须严格等于 `v{filehub-server Cargo version}`，格式为
     `vMAJOR.MINOR.PATCH`；
   - 只允许 canonical 仓库发布；fork/非 canonical 仓库失败关闭发布写操作；
   - tag 必须已经存在，且 lightweight/annotated tag 最终解析出的 commit 必须
     等于本次 workflow_dispatch 实际构建的 `GITHUB_SHA`，避免从一个 ref 构建却
     向另一个 tag 发布；
   - 在执行 GHCR push / GitHub Release 写入前完成授权校验，校验失败则不发布。
4. 人工发布复用现有完整构建、测试、CLI 三平台归档、server+admin-web 归档、
   Docker 镜像、容器冒烟、GHCR push 与 GitHub Release create/update 流程；
   发布资产名称、数量与 `.tar.gz` 格式不变。
5. 保持 `push v*` 的既有自动发布行为不变，并同步工作流头部说明。

### Out of scope

- 不让普通 `workflow_dispatch` 默认发布，不移除显式 `publish=true` 门槛；
- 不由工作流创建或移动 git tag，不自动修改 Cargo 版本；发布前必须已有正确 tag；
- 不改变 `push v*` 自动发布、canonical 仓库约束、GHCR/GitHub Release 目标、
  四件 Release 资产名称或 `.tar.gz` 格式；
- 不修改 server/admin-web/cli 源码、测试、Cargo.toml、仓库中的 Cargo.lock、
  Dockerfile 或本地构建脚本；不增加外部密钥、签名、公证、latest 镜像 tag 或
  多架构镜像；
- 不触碰工作树中现有的 `Cargo.lock`、`harness/scripts/edit-guard.py`、
  `filehub-server.yaml`、`filehub.db` 等无关改动。

### Boundary with neighboring modules

本任务只调整 `.github/workflows/build.yml` 的手工输入、Rust 依赖解析与发布授权
路径。产品内的项目/版本发布语义不变；GitHub Release 和 GHCR 仍是仓库级交付面。

## Requirement Review

需求合理：人工构建已经能生成全部交付物，缺口仅是受控地进入现有发布步骤。
直接让所有人工运行自动发布容易因误点、错误分支或 tag/源码不一致而发布错误
内容，因此采用显式 `publish=true`、必填 tag、Cargo 版本一致、canonical 仓库和
tag commit == 构建 SHA 的失败关闭门控。这样既满足手工发布，也保留日常人工
验证只构建的低风险入口。

`cargo update` 的要求可以实现，但它有明确代价：提交中的 `Cargo.lock` 不再是
CI 发布产物的最终依赖锁，两个时间不同的 run 可能得到不同依赖。为避免同一次
run 的 Linux/macOS/Windows 各自漂移，选择“前置 job 更新一次 + artifact 分发
同一 lockfile”，不在三个 matrix job 中分别执行独立解析。

另一个权衡是发布前需要先创建正确的 `v{version}` tag，并在 Actions 的
“Use workflow from”中选择解析到同一 commit 的分支或 tag；多一步操作换取发布
内容与 tag 的可追溯一致性。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-manual-build-publish | `workflow_dispatch` 通过显式 `publish=true` 与 `release_tag` 完整执行编译、测试、打包、GHCR push 和 GitHub Release；默认人工运行只构建；人工/tag 两条发布路径均校验 Cargo 版本、canonical 仓库及 tag commit 与构建 SHA 一致 | 仅 `.github/workflows/build.yml` | 发布操作多一个开关与已有 tag 前置条件，换取防误发布及源码/tag 一致性 | YAML/actionlint 通过；静态条件真值表覆盖普通人工、人工发布、tag 发布、错误版本、非 canonical、tag/SHA 漂移；发布 job 只消费校验后的 tag/publish 输出；托管 runner 的真实发布留待确认后触发 | 不创建 tag、不改产物内容/名称、不引入外部凭据或产品代码改动 |
| P-002 | fh-ci-cargo-update | 每次 workflow run 在任何 Rust 测试/编译前执行一次 `cargo update`，将生成的同一份 `Cargo.lock` 分发给三平台构建 job 后再运行 cargo test/build | 仅 `.github/workflows/build.yml`；CI 临时 lockfile 不回写仓库 | 每次运行会采用当时最新兼容依赖、跨运行不完全可复现；集中解析保证单次运行跨平台一致 | job 依赖图证明 update 先于全部 Rust 编译；三平台下载同名 lock artifact 并在编译前校验文件；静态检查禁止 matrix 内再次独立解析；hosted run 日志显示解析与构建顺序 | 不修改/提交仓库 Cargo.lock，不执行不受 Cargo.toml 版本约束的升级，不改本地构建入口 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - Actions 页面手工运行时，`publish=false` 完成编译、测试、打包、Artifacts、镜像
    构建和冒烟，但不写 GHCR/Release；
  - `publish=true` 且输入有效已有 tag、选定 ref 对应同一 commit 时，完成同一套
    编译打包后推送 `ghcr.io/{owner}/filehub:v{version}`，并创建或更新该 tag 的
    GitHub Release，附带 1 个 server+web 与 3 个 CLI `.tar.gz`；
  - 错误版本、缺失/漂移 tag、非 canonical 仓库均失败关闭发布；`push v*` 自动
    发布路径保持可用。
  - 每次运行的日志先出现一次 `cargo update`，随后三个 Rust matrix job 使用该次
    生成的同一份 `Cargo.lock` 完成测试/编译。
- Required evidence:
  - 工作流 YAML 可解析，actionlint（环境可用时）无错误；
  - 对 inputs、event、repository、version、tag/SHA、发布 job 依赖，以及
    `cargo update` → lock artifact → 三平台 Rust 编译顺序做定向静态检查和真值表
    核对；`git diff --check` 通过；
  - 本地只能验证工作流契约，GHCR 与 GitHub Release 的真实写入结果必须由确认后
    的 hosted `workflow_dispatch` 运行提供，未运行前不会宣称已线上发布成功。
- Explicit non-goals:
  - 不自动生成 tag、版本号或 changelog，不改变现有发布资产及镜像内容。

## Risks

- GitHub-hosted runner、artifact 跨 job 传递、GHCR push 和 Release API 不能在
  本地完整复现；本地验证通过不等于线上发布已验证。
- 人工发布具有仓库内容和 packages 写权限；通过显式开关、canonical 仓库、
  精确版本、已有 tag 与 commit 一致性门控降低误发布风险。
- 若 tag 在授权校验后被有权限者强制移动，仍可能产生竞争窗口；实现时应尽量在
  写入前靠近发布动作重新解析或携带已验证 SHA，且仓库应避免可移动发布 tag。
- `cargo update` 会使不同时间的构建可能采用不同的兼容依赖版本，并扩大上游依赖
  被投毒或新版本回归对发布的影响；共享 lockfile 只能保证单次运行一致，不能恢复
  跨运行可复现性。hosted run 必须保留更新日志与 lockfile artifact 作为追溯证据。
