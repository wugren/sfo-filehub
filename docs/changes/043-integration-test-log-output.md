# 集成测试每个测试 case 补齐日志输出

- Status: complete
- Owner module: filehub（filehub-server / filehub-cli 集成测试输出可见性）
- Task manifest: docs/versions/v0.1/modules/filehub/043-integration-test-log-output/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/043-integration-test-log-output/proposal.md
- Affected paths: server/tests/common/mod.rs、server/tests/api_integration.rs、
  cli/tests/common/mod.rs、cli/tests/api_integration.rs、
  cli/tests/cmd_integration.rs、cli/tests/e2e_cli_server.rs、
  harness/scripts/test-run.py、docs/changes/043-integration-test-log-output.md、
  docs/versions/v0.1/modules/filehub/043-integration-test-log-output/
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 在 `server/tests/common/mod.rs` 与 `cli/tests/common/mod.rs` 各增加
  `pub fn log_case(name: &str)`，`cli/tests/e2e_cli_server.rs` 就地定义同名
  helper；helper 统一输出 `[integration] start case-name` 一行日志。
- 在四个集成测试文件共 46 个测试 case（server api_integration 16、
  cli api_integration 15、cmd_integration 14、e2e_cli_server 1）的函数开头
  各调用一次 `log_case("case-name")`，不改任何断言与测试步骤。
- `harness/scripts/test-run.py` 的 filehub integration 四组 `cargo test`
  命令统一追加 `-- --test-threads=1 --nocapture`：日志在 canonical 入口
  直接可见，且四组套件串行避免 parallel 下日志交织（server 套件原本即串行，
  用于规避 16 路并发 502 flake）。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no（仅测试执行串行化；
  生产运行行为不变）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: yes（test-run.py 的
  integration 四组命令追加 `--nocapture` 与 `--test-threads=1`，只改变
  canonical 入口的测试输出可见性与执行串行方式，不改变规则语义与 checker
  逻辑；套件结果 JSON 与运行产物格式不变）
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `python3 harness/scripts/test-run.py filehub integration`
  （2026-08-26，等价 `.\test-run.bat filehub integration`）实跑全量：
  server api_integration 16/16、cli api_integration 15/15、
  cmd_integration 14/14、e2e_cli_server 1/1，控制台可见全部 46 条
  `[integration] start case-name` 日志。
- Result: pass
- Residual risk or follow-up:
  - `--nocapture` 会让测试调用的 CLI 正常回显（如 “Login Succeeded”、
    “push succeeded”）一并出现在控制台，属预期可见性副作用；
  - CLI 三组套件由并行改为串行，实测该层从约 3s 增至约 7s，开销可接受；
    若后续需要默认安静输出，可去掉 `--nocapture` 参数而不改测试代码。
