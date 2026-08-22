---
task_manifest: task.yaml
status: approved
---

# filehub cli/server Cargo Workspace 提案

Risk profile: not-created（standard 层级不创建 risk-profile.yaml）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- 层级理由 / 触发边界：本任务把 `cli/` 与 `server/` 两个独立 Rust crate 收编为同一个 Cargo workspace，属于构建行为/依赖解析图变更：合并两个独立 `Cargo.lock` 为单一 workspace 锁文件，统一依赖版本解析，并可统一从仓库根目录构建与测试。该变更命中 trivial 排除项中的 dependency/build graph 类别，不能按 trivial 处理；但影响局限在本仓库两个 crate、无新增依赖、无公开契约/数据/安全边界/发布部署变更，也不需要跨项目协调，属于 bounded 重构，因此按 standard 默认流程（提案 -> 变更记录 -> 实现/验证 -> 完成报告）执行，不升级 high-risk。
- 提案与层级确认陈述：当前用户已于 2026-08-20 对 standard 层级提案明确确认（回复「确认」），按本提案范围与成功标准执行。

## Background and Goal

当前 `cli/`（`filehub-cli`）与 `server/`（`filehub-server`）是互不依赖的两个独立 Rust crate，各自携带 `Cargo.toml` 与 `Cargo.lock`，需要分别构建、分别锁依赖。两个 crate 共用 `flate2`、`log`、`serde`、`serde_json`、`sha2`、`tar`、`tokio`、`reqwest`、`sfo-log` 等依赖，却维护两套独立的依赖解析结果。

目标：用 Cargo workspace 统一管理这两个 crate，使其共享一个依赖解析/锁文件，并可从仓库根目录对两个成员统一执行 `cargo check`/`cargo test` 等操作。

## Scope

### In scope

1. 在仓库根目录新增虚拟 workspace manifest（根 `Cargo.toml`）：
   - `[workspace] members = ["cli", "server"]`；
   - 明确 `resolver = "3"`（与 edition 2024 匹配）；
   - 通过 `[workspace.package]` 集中共享 `version`、`edition`、`license` 元数据。
2. `cli/Cargo.toml` 与 `server/Cargo.toml` 中重复的包元数据改为从 workspace 继承（`version.workspace`、`edition.workspace`、`license.workspace`）。
3. 在仓库根目录生成单一 `Cargo.lock`，删除 `cli/Cargo.lock` 与 `server/Cargo.lock`（workspace 成员不再维护独立锁文件）。
4. 验证 workspace 迁移不影响现有测试入口：现有 testplan 中 `cargo test --manifest-path cli/Cargo.toml` 与 `cargo test --manifest-path server/Cargo.toml`（含 `CARGO_HOME`/`CARGO_TARGET_DIR` 环境）仍可运行。

### Out of scope / non-goals

- 不重命名 package、不改动 crate 模块结构、不新增 CLI/API 行为；
- 不迁移 `[dependencies]` 到 `[workspace.dependencies]`（依赖版本收敛属于后续可选优化，不在本任务内）；
- 不引入 `cli` 与 `server` 之间的 path dependency（二者保持独立成员，仅通过 API 契约交互）；
- 不修改 `docs/modules/filehub.md` 的任务归属、不重跑或改写既有任务包/测试计划；
- 不涉及发布、部署、CI 工作流变更。

### Boundary with neighboring modules

- 本任务只改变仓库根部的构建组织方式与锁文件位置；`admin-web`（002-filehub-web）尚未落地，不受影响。
- 既有 `001-filehub-core-platform` 与 `003-filehub-cli` 的 testplan 命令保持 `--manifest-path` 形式，在 workspace 下仍然有效，因此已验证完成的任务证据不因本任务失效。

## Requirement Review

需求合理：两个 crate 共用大量依赖且同属 `filehub` 模块，workspace 统一管理是 Rust 仓库的标准做法，能消除两套锁文件的不一致，并提供单一 `cargo check/test` 入口。

关键取舍与建议方向：

- 依赖解析统一后，某个共享传递依赖的锁定版本可能从原两套 lock 中选取不同版本；这属于本任务预期内的收敛，通过全量 `cargo check --workspace` 与两 crate 既有测试验证。
- 保留 `--manifest-path` 命令兼容现有 harness testplan；同时提供根目录 `--workspace` 入口作为新的标准用法。
- package 元数据继承属于最小必要的 workspace 化管理；依赖级 `[workspace.dependencies]` 收敛留作后续任务，避免本任务 diff 过大。

### 未决问题

无。若用户希望同时做 `[workspace.dependencies]` 依赖收敛或保留成员独立锁文件，可在此提案下修订后重新确认。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-01 | fh-cargo-workspace | 在仓库根目录建立虚拟 Cargo workspace（members=cli/server、resolver 3、[workspace.package] 共享元数据），成员 manifest 改 workspace 继承，生成根 Cargo.lock 并删除成员独立锁文件 | 不引入 cli/server 之间 path dependency，不迁移 [workspace.dependencies]，不改公开契约与既有 testplan 命令 | 锁文件统一后共享传递依赖版本可能与迁移前两套锁不同，属预期内收敛，用全量编译与既有测试验证 | cargo metadata 输出 filehub-cli 与 filehub-server 两个成员；根 Cargo.lock 存在且 cli/Cargo.lock、server/Cargo.lock 不存在；cargo check --workspace 通过；既有 testplan 的 --manifest-path 测试命令通过 | 不改依赖版本、不改 crate/模块结构、不改 docs 模块文档与 CI/发布流程 |

## Success Criteria

可见结果与必须的证据：

1. 仓库根目录存在 workspace `Cargo.toml`，`cargo metadata --workspace`（或 `cargo metadata` 输出）列出 `filehub-cli` 与 `filehub-server` 两个成员；
2. 根目录 `Cargo.lock` 生成，`cli/Cargo.lock` 与 `server/Cargo.lock` 不再存在；
3. `cargo check --workspace` 通过（含 `CARGO_TARGET_DIR=.harness/cargo-target` 环境）；
4. 现有 testplan 中的两 crate 测试命令（unit/dv/integration 集合）经 `--manifest-path` 仍通过，或在 workspace 下用等价 `--workspace` 命令通过；
5. 交付证据：standard 变更记录（`docs/changes/004-cargo-workspace.md`）与任务本地 `completion-report.md` 完整，`lower-tier-check.py --profile completion` 校验通过。

非目标成功证据：本任务不验收 CLI/服务端功能行为本身，也不要求改动测试用例。

## Risks

- 锁文件统一（中低）：两套 lock 合并后部分共享依赖的锁定版本可能与迁移前不同；通过全量编译与既有测试覆盖，若出现版本不兼容则优先选择兼容版本组合。
- 构建命令兼容（低）：现有 harness testplan 使用 `--manifest-path` 加 `CARGO_HOME`/`CARGO_TARGET_DIR`，workspace 下仍受支持；完成后用真实 testplan 命令复跑验证。
- 文档/任务证据影响（低）：删除成员锁文件会改变 `cli/`、`server/` 目录内容，但不触碰既有任务包的批复文档与测试结果。
