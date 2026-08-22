---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-21
---

## Approval Record

- approver: user
- approval_date: 2026-08-21
- user_statement: 确认（按提案显示内容、standard 层级执行；Open Questions 三项答复：
  1 旧命令直接移除、2 pull 使用精确文件路径、3 其余命令统一 server:... 目标形态）

# filehub-cli 命令面改为 push / pull，全部目标统一 <server:...> 形态

Risk profile: not-created（standard 层级不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
  - 确认记录：2026-08-21 当前用户回复「确认」并按 Open Questions 答复将三条
    决议写入确认范围：1) 旧命令 `publish`/`download` 直接移除不留别名；
    2) `pull` 的 `<path>` 为精确输出文件路径；3) `versions`/`new-version`/
    `lock-version`/`delete-app` 同步统一为 `<server:...>` 目标形态。
- Tier rationale / triggered boundaries:
  - 本次改动 CLI 公共命令面：`publish` 换名 `push`、新增 `pull`，并把六条命令的
    参数统一为 `<server:...>` 前缀目标串，不满足 trivial 的「无公共契约/CLI
    影响」条件；
  - 变化有界且集中在 `cli` 模块：不改服务端 API、数据库 schema、凭据格式、
    依赖图与部署面，不构成 high-risk 的 material consequence；
  - 按仓库默认，有界单项目重构/功能工作走 standard：变更记录 + 定向验证 +
    独立缺陷发现后完成报告。

## Background and Goal

当前命令面把服务器、项目、版本、应用名拆散：`publish` 用
`<路径> <project>:<version> [--app] [SERVER]`，`download` 用
`<project>[:<version>] -o <目录> [--app] [SERVER]`，其余命令也各自带独立
`[SERVER]` 位置参数。用户希望命令面统一为直观的 push/pull 模型，且所有目标都
写成 `<server:...>` 前缀串：

- `filehub push <server:project:version:name> <path>`（替代 `publish`）；
- `filehub pull <server:project:version:name> <path>`（替代 `download`，
  `<path>` 为精确输出文件路径）；
- `filehub versions <server:project> [-o <path>] [--format text|json]`；
- `filehub new-version <server:project:version>`；
- `filehub lock-version <server:project:version>`；
- `filehub delete-app <server:project:version:name>`。

目标：push/pull 成为发布与拉取主命令，应用名 `name` 成为目标串必填段（不再出现
缺省 `default`），服务器不再作为独立位置参数；login/logout 保持现有形态。

## Scope

### In scope

1. `publish` 改名为 `push`：`push <server:project:version:name> <path>`；
   `<path>` 是本地待发布文件或目录（仍统一打包 `.tar.gz`），目标串解析出
   server/project/version/name 后走现有 v1 API（resolve project +
   `PUT .../versions/{version}/apps/{name}`），成功输出
   `push succeeded: project:version:name (sha256 ...)`；
2. 新增 `pull`：`pull <server:project:version:name> <path>`，走现有 v1 API
   （resolve project + GET version 元数据 + 按 app 下载），SHA-256 校验后
   **原子写入 `<path>` 这个精确文件路径**，成功输出 `pull succeeded: <path>`；
3. 其余命令统一 `<server:...>` 目标形态：
   - `versions <server:project>`（server 从目标解析，不再接位置参数）；
   - `new-version` / `lock-version` 为 `<server:project:version>`；
   - `delete-app` 为 `<server:project:version:name>`；
4. 旧命令 `publish` 与 `download` **直接移除**，不保留别名（含 args 类型、
   handler、README 命令表与测试中的引用）；
5. 新增统一目标解析与校验（从右侧固定切分，见 P-003）；
6. 同步更新 `cli/src/`（args、命令分发、全部 handler、解析辅助）、
   `cli/tests/`（单元/集成/dv 用例全部迁移并补充解析正反例）、
   `cli/README.md` 命令表、`docs/modules/filehub.md` 命令面描述；
7. 按 standard 流程产出 `docs/changes/016-cli-push-pull-commands.md` 与
   任务包内 `completion-report.md`。

### Out of scope

- 不修改服务端代码与 `docs/api/v1-contract.md`（v1 API 契约不变）；
- 不改变登录/凭据机制（login/logout 仍为 `[SERVER]` 形态）、退出码、
  归档打包/安全净化逻辑本身；
- 不实现 `pull` 自动解压；`pull` 只保存 `.tar.gz` 归档（沿用现有 download
  完整性校验与原子落盘语义，只是目标变为精确文件路径）。

### Boundary with neighboring modules

仅 `cli` 模块及其 README、命令面描述文档受影响；`filehub-server` 与 admin-web
不动。上传/下载仍复用 `cli/src/apiclient/` 与 `cli/src/archive/` 既有传输与
完整性校验逻辑。

## Requirement Review

需求合理：push/pull 是文件集散客户端的直观语义；目标串内嵌服务器与 name 消除
`--app` 与独立 SERVER 位置参数，六条命令统一形态降低学习成本；`pull` 用精确
文件路径符合「拉取到指定位置」的直觉。

选择的方向与权衡：
- 服务器身份沿用既有的 `host[:port]` 归一化与凭据 key 语义，目标中的 server 段
  直接交给现有凭据与传输层；
- 目标串从右侧固定切分：`name` 取最后一段、`version` 倒数第二段、`project`
  倒数第三段、剩余部分整体拼接为 server。`127.0.0.1:8080:test:1.1.0:logs`
  与 IPv6 含冒号 server 均可正确解析；代价是 project/version/name 不能含
  冒号（与现契约一致）；
- 旧命令直接移除（用户确认），不保留别名，换取最小命令面；兼容代价以变更记录
  明示。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-cli-push | `publish` 改名为 `push`（push server:project:version:name + 本地路径），沿用打包/校验/发布链路；旧 `publish` 移除 | cli 命令面 + handler + 测试 + README | 命令面破坏性改名换取统一语义 | `push --help` 新形态；集成/单元用例以新参数发布成功；help 不再含 publish | 不改服务端 API、不改退出码 |
| P-002 | fh-cli-pull | 新增 `pull server:project:version:name` + 本地输出路径：按 app 下载、SHA-256 校验、原子写入精确输出文件；旧 `download` 移除 | cli 命令面 + handler + 测试 + README | 精确文件路径语义，用户传入目录报输入错误 | `pull --help` 新形态；dv/integration 用新参数拉取且字节/SHA 一致；目录目标被拒 | 不自动解压、不做断点续传 |
| P-003 | fh-cli-unified-target | 新增 server:... 目标解析族：server:project / server:project:version / server:project:version:name，右侧固定切分 + 空段/冒号校验 | cli args + 单元测试 | 从右切分换取 server 含端口/IPv6 兼容 | 解析正反例覆盖 `127.0.0.1:8080:test:1.1.0:logs`、`::1:8080:...`、缺段与非法输入 | 不引入 glob、不支持明文凭据入参数 |
| P-004 | fh-cli-management-targets | `versions`/`new-version`/`lock-version`/`delete-app` 统一 server:... 目标形态并移除独立 server 位置参数 | cli 命令面 + handler + 测试 + README | 行为等价，纯参数承载位置变化；旧脚本需改写 | 各 handler 用例以新目标形态通过（含 409/404/403 错误映射不变） | 不改命令名与既有输出格式 |

## Success Criteria

- 用户可见结果：
  - `filehub push 127.0.0.1:8080:test:1.1.0:logs "C:\\Program Files (x86)\\bucky-vpn\\data\\logs"`
    发布成功且 app 名为 `logs`（不再出现 `default`）；
  - `filehub pull 127.0.0.1:8080:test:1.1.0:logs <输出文件>` 下载成功，输出文件
    SHA-256 与服务端一致；
  - `filehub versions 127.0.0.1:8080:test` 等六条命令均按 `<server:...>` 形态
    工作；`filehub --help` 不再列出 publish/download。
- 必需证据：
  - `cargo test -p filehub-cli` 全量通过（迁移后的单元/集成/dv 用例 + 新增目标
    解析反例）；
  - `cargo fmt --check`（本次改动源文件）通过；
  - standard 变更记录与完成报告记录命令面变化、目标解析规则、精确路径语义与
    验证结果。
- 明确非目标：不改服务端、不加自动解压、login/logout 形态不变。

## Risks

- 命令面破坏性变更（中）：`publish`/`download` 及其余命令旧脚本一次性失效；
  用户已确认直接移除，不留别名；变更记录明示旧命令映射与改写示例。
- 目标串解析歧义（低）：从右侧固定切分依赖 project/version/name 不含冒号的
  契约；若未来允许含冒号版本号需重新设计。
- pull 目标语义（低）：`<path>` 为精确文件路径，用户传入目录名会得到明确输入
  错误，不会静默覆盖目录。

## Open Questions（已确认）

1. 旧命令 `publish`/`download` 是否保留兼容别名？——**已确认：直接移除**。
2. `pull <...> <path>` 的 `<path>` 是精确输出文件路径还是输出目录？——
   **已确认：精确文件路径**。
3. `versions`/`new-version`/`lock-version`/`delete-app` 是否统一成
   `server:...` 目标形态？——**已确认：统一**。
