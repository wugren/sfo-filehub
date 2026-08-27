---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-25
---

# 集成测试补齐：filehub v1 接口 + filehub-cli 命令行

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment
- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 本次交付以「生成/补齐集成测试」为主，属于有界单项目测试工作：服务端
    只新增/补充 `server/tests/` 集成用例，不改变服务端生产行为；
  - filehub-cli 新增真实二进制级命令行集成测试（`cli/tests/`），并补一条
    `--help`/`--version` 退出码对齐修复——该行为已由 `cli/README.md` 与
    `design/cli.md` 冻结为「退出码 0（含 --help）」，当前实现实测返回 1，
    属于把实现对齐到既有公开 CLI 契约的最小分支修复，不新增/变更命令面；
  - 命中 security/runtime-integration/contract-protocol 触发规则的均为
    **测试用例内容**（认证与授权负例、续期重试、错误契约断言），不构成
    生产代码、数据、依赖、发布面或兼容性的实质变更，不满足 high-risk 的
    「经确认的实质后果」门槛；按有界标准任务执行。
- Proposal and tier confirmation:
  - 用户于 2026-08-25 回复「确认」前，已展示完整测试用例清单（接口 A–G/J、
    CLI 命令行 K1–K22、CLI↔真实服务端 H1–H5）并说明推荐层级 standard；
  - 用户 2026-08-25 回复「确认」，按所展示清单与推荐层级 standard 定稿；
    本提案置为 `status: approved`；若后续用户改为 trivial/high-risk，则按
    确认结果执行对应流程。

## Background and Goal
- 现状：`server/tests/api_integration.rs` 已覆盖一批真实 HTTP 集成用例；
  `cli/tests/api_integration.rs` 是「进程内直接调用 handler + MockServer」，
  不是真实命令行的集成测试；admin-web 集成测试为契约桩；部分 v1 契约边界
  （token 属性修改不重签、revoke、project_scope 隔离、版本名校验矩阵、
  非 gzip 新契约、下载响应头、错误体结构等）在集成层缺失。
- 目标：按 `docs/modules/filehub.md` 与 `docs/api/v1-contract.md` 需求补齐
  集成测试，覆盖「服务端真实 HTTP 契约缺失断言 + filehub-cli 真实二进制
  命令行 + CLI↔真实 filehub-server 端到端」三层。

## Scope
### In scope
1. 服务端接口集成测试（`server/tests/api_integration.rs`，变更 fh-it-server-api）
   - 补齐 A4 refresh 续期、B7 删除级联、C4 协作者 write/admin 矩阵、C5 移除
     即时生效、D1/D2/D4/D6/D7/D8 token 契约（含 expires 边界、不重签、
     revoke、Specified 隔离、凭据身份隔离）、E2 版本名校验矩阵、E5 重复锁
     幂等、F5 预检超限、F8 非 gzip 不透明入库、F10 版本不存在 404、
     G2 Content-Disposition、G5 空版本 404、J1 错误体结构、J2 未知路由；
   - 已覆盖用例（A1–A3/A5、B1–B6、C1–C3、D3/D5、E1/E3/E4/E6、F1–F4/F6/F7/F9、
     G1/G3/G4/G6）不重复生成。
2. filehub-cli 命令行集成测试（新建 `cli/tests/cmd_integration.rs`，
   变更 fh-it-cli-cmd）
   - 通过 `env!("CARGO_BIN_EXE_filehub")` 启动真实二进制，覆盖 K1–K22：
     帮助/版本/无参、参数互斥、stdin 非终端、凭据来源优先级、stdin 登录、
     凭据文件生命周期与 0600、损坏配置、target 语法、401 续期一次、只读
     token、404/422/409 映射、网络失败、不安全归档、pull 完整性失败、
     输出契约、凭据不泄露、退出码全矩阵。
3. CLI↔真实服务端端到端（新建 `cli/tests/e2e_cli_server.rs`，
   变更 fh-it-cli-e2e）
   - 进程内装配真实 `filehub-server`（复用 server crate 公开装配接口），
     以子进程调用 CLI：login → new-version → push（文件+目录）→ versions
     （text/json）→ lock-version → 重复 push 409 → pull SHA 校验 →
     delete-app → logout，验证退出码与落盘内容。
4. 最小契约对齐修复（`cli/src/cli/mod.rs`，变更 fh-it-help-exit-code）
   - `CliArgs::try_parse()` 失败分支对 clap `DisplayHelp`/`DisplayVersion`
     返回退出码 0（其余解析错误仍 1），使 K1 与冻结文档一致。
5. 统一入口注册（`harness/scripts/test-run.py` MODULE_SUITES）与
   `docs/changes/042-integration-tests.md`、`completion-report.md`。
### Out of scope
- 不改服务端与 CLI 其它生产逻辑、不改 v1 契约、不改 admin-web；
- 不新建 `testplan.yaml`（standard 按 unified-test-entry 规则默认不走任务
  testplan），但新测试挂到模块级 canonical suite 保持 `all all` 可达；
- 不重做单元/DV 层已覆盖的纯逻辑验证。
### Boundary with neighboring modules
- 服务端集成复用 `server/tests/common` 装配方式；CLI 测试的 MockServer 仅
  服务契约形状，端到端以真实 server 为准；CLI 新增 dev-dependency 仅限
  `filehub-server`（path 依赖），不影响运行时依赖图与发布产物。

## Requirement Review
- 需求合理：接口集成、真实命令行集成与端到端三层互补，能暴露 handler 级
  测试看不到的命令面/退出码/凭据文件问题；已确认清单与需求文档逐一对应。
- 已确认取舍：K1 按冻结文档断言 `--help`/`--version` 退出码 0，因此附带
  最小生产修复；如用户希望保持当前退出码 1，需同时修订 README 与
  design/cli.md，本次默认采用「对齐既有冻结文档」方向。

## Proposal Items
| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|---|---|---|---|---|---|---|
| P-01 | fh-it-server-api | 补齐 v1 接口集成层缺口用例 | 仅 server/tests，不改服务端逻辑 | 以新增断言补缺口，已覆盖不重复 | 新增接口用例全部通过 | 不重写已覆盖场景 |
| P-02 | fh-it-cli-cmd | 真实二进制级 CLI 命令行集成 K1–K22 | 仅 cli/tests；使用 MockServer 契约形状 | 命令行层驱逐 handler 进程内盲区 | 新命令用例全部通过 | 不覆盖 CLI 交互 TTY 自动化 |
| P-03 | fh-it-cli-e2e | CLI↔真实 filehub-server 端到端 H1–H5 | cli/tests + dev-dependency | 真服务装配提升跨模块真实性 | 端到端用例全部通过 | 不启动外部部署进程 |
| P-04 | fh-it-help-exit-code | `--help`/`--version` 退出码 0 对齐冻结文档 | 仅 run_cli 解析失败分支 | 最小 1 分支修复换取 K1 可断言 | K1 用例通过，`--help` 实测 0 | 不改其它 clap 错误语义 |
| P-05 | fh-it-suite-entry | test-run.py 注册新测试入口 | 仅 harness/scripts/test-run.py 套件表 | 模块级 all/all all 可达 | `test-run.py filehub integration` 可执行新用例 | 不新增 ad hoc 命令 |

## Success Criteria
- 可见结果：新增服务端接口用例、CLI 命令行用例与端到端用例全部通过；
  `filehub --help`/`--version` 退出码实测为 0；
  `harness/scripts/test-run.py filehub integration` 能执行到全部新测试。
- 必需证据：`docs/changes/042-integration-tests.md`（Status: complete）、
  `completion-report.md`、`lower-tier-check.py` pre-edit/completion 通过、
  变更清单与提案范围一致。
- 非目标：不验收服务端权限实现本身的全量安全审查，不生成 testplan.yaml。

## Risks
- CLI 帮助/版本退出码修复属于公开命令面行为，但方向是「对齐既有冻结文档
  （README/design 均为 0）」，兼容性影响最小；仍有脚本若依赖当前 1 则会
  变化，作为残余风险记录在变更记录。
- 新增安全/合约负例用例本身不改变生产行为；端到端测试启动真实 server 与
  CLI 子进程，测试代码需隔离临时数据目录与随机端口，避免影响本机环境。
