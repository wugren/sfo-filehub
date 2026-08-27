---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-26
---

# 集成测试每个测试 case 补齐日志输出

## Background and Goal

- 用户提出：`.\test-run.bat filehub integration`（即
  `harness/scripts/test-run.py filehub integration`）的每个测试 case
  都应有日志输出。
- 实测现状：四个集成套件共 46 个测试 case
  （server api_integration 16、cli api_integration 15、
  cmd_integration 14、e2e_cli_server 1），测试函数体内当前没有任何
  `println!`/`eprintln!`；命令输出只有 cargo 的 `test <name> ... ok`
  汇总行，缺少每个 case 自己的运行日志。

### 目标

`filehub integration` 运行期间，46 个测试 case 每一个都至少输出一条
带 case 标识的日志（例如 `[integration] start <test_name>`），并且在
命令控制台直接可见；测试断言语义保持不变，46/46 全部通过。

## Scope

### In scope

1. `fh-log-cases`（server/tests/、cli/tests/）
   - 在各集成测试的 common 装置（server/tests/common/mod.rs、
     cli/tests/common/mod.rs）增加一个轻量日志 helper（例如
     `pub fn log_case(name: &str)`），e2e_cli_server.rs 就地定义同名
     helper；
   - 在 server/tests/api_integration.rs（16）、cli/tests/api_integration.rs
     （15）、cli/tests/cmd_integration.rs（14）、
     cli/tests/e2e_cli_server.rs（1）的每个测试函数开头调用 helper，
     输出该 case 的日志。
2. `fh-log-suite-entry`（harness/scripts/test-run.py）
   - 为 filehub integration 四组 `cargo test` 命令统一追加
     `--nocapture`，使测试函数标准输出在 canonical 入口直接可见；
   - 为便于逐 case 阅读日志，四组命令统一 `--test-threads=1`
     （server 套件已串行，CLI 三组新增串行，避免并行交织）。

### Out of scope

- 不改任何生产代码、测试断言、Mock 契约与测试步骤逻辑；
- 不引入第三方日志框架，不新增日志文件产物；
- 不改 unit/dv/all 命令的执行方式（all 因继承 integration 命令自动
  获得同样的可见性）；
- 不动 `docs/changes/042-integration-tests.md` 内容。

## Requirement Review

- 需求合理：canonical 入口当前虽然会打印 cargo 的 `test <name> ... ok`
  汇总，但 46 个测试函数体内没有任何日志，无法从命令输出判断单个 case
  的执行进度；给每个 case 补一条启动日志并让 canonical 入口显示，是最小
  且可读的满足方式。
- 已确认取舍：不引入日志框架、不改测试断言；只在每个测试函数开头通过
  common helper 打一条带 case 名的日志，并给四组 `cargo test` 命令追加
  `--nocapture` 与串行参数，保证日志不交错、全程可见。

## Risks

- `--nocapture` 会让测试 stdout 直接进入控制台：并行时日志会交织，
  因此四组命令统一串行；CLI 三组实测约 2.2s/0.68s/7.7s，串行开销可忽略。
- 测试日志只读不写磁盘，不影响 harness 运行产物与测试结果 JSON。
- 若用户不希望在默认输出里出现测试噪音，可作为方案取舍改为仅在失败时
  展示日志（去掉 `--nocapture`）；本提案默认按「正常运行即可见日志」执行。

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 需求明确、影响集中在 filehub 模块的测试文件（server/tests/、
    cli/tests/）与 `harness/scripts/test-run.py`；
  - 修改 `harness/scripts/test-run.py` 属于 Harness 测试基础设施变更，
    命中 trivial 的「无 Harness-process impact」排除项，不按 trivial；
  - 无生产行为、契约、数据、安全、依赖、发布面或跨模块实质后果，
    不满足 high-risk 的「经确认的实质后果」门槛；按 standard 执行。
- Proposal and tier confirmation:
  - 2026-08-26 向用户展示提案路径、需求、范围、非目标、成功标准、
    建议层级 standard 与层级理由，并列出「每次运行可见日志（--nocapture）」
    的默认取舍；
  - 用户 2026-08-26 回复「确认」，按所展示清单与推荐层级 standard 定稿；
    本提案置为 `status: approved`。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|---|---|---|---|---|---|---|
| L-01 | fh-log-cases | 46 个测试 case 每个都有 case 日志输出 | 仅测试文件与 common helper | 每个 case 一行 helper 调用，最小侵入 | 46 个 case 运行时各输出至少一条日志且全通过 | 不改断言/生产代码 |
| L-02 | fh-log-suite-entry | canonical 入口显示每个 case 的日志 | 仅 test-run.py 四组命令参数 | `--nocapture` + 套件串行保证日志可读 | `test-run.py filehub integration` 控制台可见 46 条 case 日志 | 不改 unit/dv 调用方式 |

## Success Criteria

- `python3 harness/scripts/test-run.py filehub integration`（等价
  `.\test-run.bat filehub integration`）输出中，46 个测试 case 每个都
  有至少一条以 case 名开头的日志；四组套件 16/15/14/1 全部通过；
- `lower-tier-check.py --profile pre-edit` 通过并捕获任务基线；
  standard 流程产出 `docs/changes/043-*.md` 与 completion-report.md。
