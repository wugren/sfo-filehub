# filehub cli/server Cargo Workspace 统一管理

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/004-cargo-workspace/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/004-cargo-workspace/proposal.md
- Affected paths: Cargo.toml（新增）, Cargo.lock（新增）, cli/Cargo.toml（包元数据改 workspace 继承）, server/Cargo.toml（包元数据改 workspace 继承）, cli/Cargo.lock（删除）, server/Cargo.lock（删除）
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

在仓库根目录新增虚拟 Cargo workspace：`[workspace] members = ["cli", "server"]`、`resolver = "3"`，并通过 `[workspace.package]` 集中共享 `version`（0.1.0）、`edition`（2024）、`license`（MIT）。`cli/Cargo.toml` 与 `server/Cargo.toml` 的相同包元数据改为 `*.workspace = true` 继承，依赖声明保持原样。

生成仓库根 `Cargo.lock`（365 个锁定包，Rust 1.96.0 兼容），删除两个成员 crate 的独立 `Cargo.lock`，使 cli 与 server 共享同一份依赖解析结果。既有的 harness testplan 命令仍使用 `--manifest-path` 加 `CARGO_HOME`/`CARGO_TARGET_DIR` 环境，在 workspace 下无需改写即可继续运行。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: yes
  - 证据：本任务正是把两个独立 crate 的依赖解析合并为一个 workspace 锁文件，属于预判内的依赖/构建图变更，不新增或升级任何直接依赖，不改变 crate 名称、版本号、公开契约或产物形态；统一解析后的全量 `cargo check --workspace` 与两 crate 全部既有测试均通过，未发现供应链信任变更。
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo metadata`（workspace 含 filehub-cli 与 filehub-server）、`cargo check --workspace`、并按既有 testplan 复跑 `cargo test --manifest-path cli/Cargo.toml --lib/--test dv_tests/--test api_integration` 与 `server/Cargo.toml --test unit_tests/--test dv_tests/--test api_integration`（均带 `CARGO_HOME`/`CARGO_TARGET_DIR` 指向 .harness）
- Result: passed
- Residual risk or follow-up: workspace 统一解析后部分共享传递依赖的锁定版本与迁移前两套锁文件不同，当前全量编译与测试全部通过；后续若新增依赖，应在 workspace 根目录统一 `cargo add`/锁定并复跑 testplan。server 源码中已存在的 unused-import/dead-code 警告与本任务无关，未在本任务内清理。
