# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/043-integration-test-log-output.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `server/tests/common/mod.rs` 与 `cli/tests/common/mod.rs` 新增
    `pub fn log_case(name)` helper，`cli/tests/e2e_cli_server.rs` 就地定义
    同名 helper；四个集成测试文件共 46 个测试 case 的函数开头均调用
    `log_case("case-name")`，输出 `[integration] start case-name`；
  - `harness/scripts/test-run.py` 的 filehub integration 四组 `cargo test`
    命令统一追加 `--test-threads=1 --nocapture`：46 条 case 日志在 canonical
    入口直接可见，四组套件串行避免交织，未改任何测试断言与生产代码。
- Handoff:
  - `python3 harness/scripts/test-run.py filehub integration`（等价
    `.\test-run.bat filehub integration`）全绿：
    server api_integration 16/16、cli api_integration 15/15、
    cmd_integration 14/14、e2e_cli_server 1/1，共 46 个 case 每个均输出
    一条 `[integration] start case-name` 日志；
  - `lower-tier-check.py --profile pre-edit` 通过，任务基线已捕获。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|---|---|---|---|---|---|
| fh-log-cases | 46 个测试 case 每个都有带 case 标识的日志输出，且不改断言 | proposal.md L-01 | server/tests/common/mod.rs、cli/tests/common/mod.rs、e2e_cli_server.rs 新增 log_case helper；46 个测试函数开头各一次调用，实跑全部输出 `[integration] start case-name` | 匹配 | pass |
| fh-log-suite-entry | canonical 入口直接显示每个 case 的日志，四组命令串行防交织 | proposal.md L-02 | harness/scripts/test-run.py 四组 integration 命令统一 `--test-threads=1 --nocapture`；canonical 实跑控制台可见全部 46 条日志 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|---|---|---|---|---|
| behavior-and-logic | 逐文件核对 46 处 log_case 调用点（16+15+14+1）与四个 helper 定义；检查 CLI 正常回显（“Login Succeeded”“push succeeded”等）在 `--nocapture` 下的可见性 | 反向推演：若某个 case 漏插日志，实跑输出会缺 `[integration] start` 行；若 helper 放在 common 但未导入，编译期即失败——实跑 46 条日志齐全且编译无误，证明覆盖完整 | server 16、cli api 15、cmd 14、e2e 1 四个套件输出均逐 case 可见日志，无遗漏 | pass |
| boundaries-and-failure-paths | 验证失败路径语义未变：cmd_corrupt_config、cmd_pull_corrupt_body、cmd_push_unsafe_symlink 等负例仍按原退出码断言通过；`--nocapture` 仅在测试进程侧生效，不影响 run_bin 子进程的 stdout/stderr 捕获 | 推演 `--nocapture` 是否可能把子进程 stdout 双写或改变 child.wait_with_output 结果：参数只作用于测试 harness 自身 stdout，RunOutcome 仍独占子进程管道，断言不受影响 | 所有负例用例仍通过，未被日志修改影响；子进程捕获语义不变 | pass |
| regression-and-side-effects | 运行 canonical 入口两次（改前改后），对比四组套件 16/15/14/1 结果一致；检查 test-run.py 变更仅限 integration 四组命令参数，unit/dv/all 合成方式不变 | 反向推演 CLI 三组由并行改串行是否屏蔽既有并行隐藏缺陷：串行不改变断言与数据隔离，仅增加约 4 秒耗时；若未来需要并行可单独调整参数 | 改后全量 46/46 通过，日志可见；既有 042 套件结果零回归，唯一副作用是 CLI 正常回显进入控制台与串行耗时 | pass |

## Verification

- Targeted check:
  - `python3 harness/scripts/test-run.py filehub integration`（2026-08-26）：
    server api_integration 16/16、cli api_integration 15/15、
    cmd_integration 14/14、e2e_cli_server 1/1，全部通过，控制台可见全部
    46 条 `[integration] start case-name` 日志；
  - `lower-tier-check.py --profile pre-edit` 通过（任务开始基线已捕获）。
- Result: pass
- Exception reason: not-applicable（目标验证全部通过，无需豁免）。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|---|---|---|---|---|
| F-1 | low | `--nocapture` 实跑输出中，测试调用的 CLI 正常回显（如 “Login Succeeded”“push succeeded”）也随之进入控制台 | 这是 `--nocapture` 的预期可见性副作用；若产品环境希望默认安静，可去掉该参数而不是改测试代码，非缺陷 | no |
| F-2 | low | CLI 三组套件由并行改为 `--test-threads=1` 后，本层实测耗时约 7s | 串行换取日志可读性与确定性，增加少量本地测试时间；若日志改为按需输出可恢复并行，非缺陷 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: L-01/L-02 全部落地且相互一致：46 个集成测试 case 每个都通过
  `log_case` 输出 `[integration] start case-name`，canonical 入口在
  `--nocapture` 下直接可见全部日志；四组套件 16/15/14/1 全绿，既有断言与
  子进程捕获语义零回归；独立缺陷发现覆盖行为逻辑、边界失败路径与回归
  副作用，F-1/F-2 均为低危非阻塞记录。
