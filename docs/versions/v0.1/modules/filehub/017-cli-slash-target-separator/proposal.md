---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-22
---

## Approval Record

- approver: user
- approval_date: 2026-08-22
- user_statement: 确认（按提案显示内容、standard 层级执行，全目标形态统一改为 `/` 分隔）

# filehub-cli 目标串分隔符改为 `/`（server/project/version[/name]）

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 变更 CLI 公共命令面的目标串分隔符（`:` -> `/`），属于公共命令面影响，
    不满足 trivial 的「无公共契约/CLI 影响」条件；
  - 变化有界且集中在 `cli` 模块：不改服务端 API、数据库、凭据、依赖与部署面，
    不构成 high-risk；
  - 按仓库默认，有界单项目命令面调整走 standard：变更记录 + 定向验证 +
    独立缺陷发现后完成报告。
- Proposal and tier confirmation:
  - 本提案需当前用户明确确认后才能执行。用户可选择按此提案确认（standard）、
    以替换层级确认，或要求修订提案；
  - 确认后将 `workflow_tier` 从 `pending` 写为确认层级并把本提案置为
    `status: approved`（2026-08-22 当前用户「确认」，按 standard 执行）。

## Background and Goal

016 任务将命令面统一为 `<server:...>` 冒号分隔目标串（如
`127.0.0.1:8080:test:1.1.0:logs`）。用户希望分隔符改为斜杠：

- `push`/`pull`/`delete-app`：`server/project/version/name`
- `new-version`/`lock-version`：`server/project/version`
- `versions`：`server/project`

server 段仍为 `host[:port]`（端口/IPv6 的冒号保留），例如
`127.0.0.1:8080/test/1.1.0/logs`。斜杠分隔同时消除 016 记录的非阻塞歧义
（带端口 server 漏写字段时冒号右切分无法报错）：新解析按 `/` 直接分段，
段数与字段数严格对应，缺段会明确报「目标必须为 ...」。

## Scope

### In scope

1. `cli/src/cli/args.rs`：`parse_server_project` /
   `parse_server_project_version` / `parse_server_project_version_name` 改为按
   `/` 切分，严格校验段数与空段；clap 帮助文案同步改为
   `server/project/version/name` 形态；
2. 命令行为：`push`/`pull`/`versions`/`new-version`/`lock-version`/
   `delete-app` 全部按新分隔符解析，`host:port` 冒号不受影响；
3. `cli/tests/`：既有目标用例迁移到 `/` 形态，并新增正反例
   （`127.0.0.1:8080/test/1.1.0/logs`、IPv6、无端口 host、缺段、空段、
   多余段拒绝）；
4. `cli/README.md` 与 `docs/modules/filehub.md` 目标形态说明同步为 `/`；
5. 按 standard 流程产出 `docs/changes/017-cli-slash-target-separator.md` 与
   任务包内 `completion-report.md`。

### Out of scope

- 不修改服务端代码与 `docs/api/v1-contract.md`；
- 不改 login/logout 形态、退出码、凭据机制与归档/校验逻辑；
- 不复用/兼容旧 `:` 分隔输入（016 为刚确认的新命令面，无历史存量需要兼容）；
- 不改 016 及更早任务包的历史文档（参考资料不变，本任务只新增交付记录）。

### Boundary with neighboring modules

仅 `cli` 模块及其 README、模块文档受影响；`filehub-server`、admin-web 不动。

## Requirement Review

需求合理：斜杠分隔让 server（含端口冒号）与目标字段边界清晰，`host:port` 不再
需要从右侧猜测切分位置，也顺手解决带端口 server 缺段时静默错拆的问题。
代价：project/version/name 不能再包含 `/`（与当前命名习惯一致，无冲突）；
新版命令面尚无存量脚本，无需兼容旧分隔符。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-cli-slash-target | 目标串分隔符统一为 `/`：`server/project`、`server/project/version`、`server/project/version/name`；server 段保留 host:port 冒号 | cli args/handler 解析 + 帮助文案 + 测试 + README + 模块文档 | 字段不可含 `/`；旧 `:` 输入不再接受 | 解析正反例 `127.0.0.1:8080/test/1.1.0/logs`、IPv6、无端口、缺段/空段/多余段；`cargo test -p filehub-cli` 全量通过；`push/pull --help` 显示 `/` 形态 | 不改服务端、不变更退出码、不兼容旧分隔符 |

## Success Criteria

- 用户可见结果：
  - `filehub new-version 127.0.0.1:8080/test/1.1.0`；
  - `filehub push 127.0.0.1:8080/test/1.1.0/logs "C:\\Program Files (x86)\\bucky-vpn\\data\\logs"`;
  - `filehub pull 127.0.0.1:8080/test/1.1.0/logs 输出文件`；
  - `filehub versions 127.0.0.1:8080/test` 等均按 `/` 形态工作。
- 必需证据：
  - `cargo test -p filehub-cli` 全量通过（含新分隔符解析正反例）；
  - `rustfmt --edition 2024 --check` 覆盖本次改动源文件；
  - 真实 filehub-server 端到端验证 push -> versions -> pull（SHA 一致）；
  - standard 变更记录与完成报告。
- 明确非目标：不兼容旧 `:` 目标串、不改服务端、login/logout 形态不变。

## Risks

- 命令面再变更（低）：016 刚切换命令名与目标形态，本任务只改分隔符；仓库内
  无存量脚本依赖 `:` 形态，README 同步更新。
- 字段含 `/`（低）：project/version/name 含 `/` 将无法表达；与现命名约定一致，
  解析会明确报缺段/多余段错误，不会静默错拆。

## Open Questions（已确认）

- 无未决问题；用户「确认」采纳「全部目标形态一起改为 `/`」的理解。
