---
task_manifest: task.yaml
status: approved
---

# 049-project-name-cli-addressable：拒绝 CLI 无法寻址的项目名（评审第 7 项中低危）

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Proposal and tier confirmation: 用户 2026-08-26 回复「确认」，采用 standard
  层级与上述最小拒绝集范围继续执行。
- Tier rationale / triggered boundaries:
  - 需求明确，改动收敛在 filehub 项目的 projects 子模块创建入口、单元回归与
    v1 契约文档，属于有界单项目 bugfix；
  - 不满足 trivial：本次变更会收紧公开 v1 API（`POST /api/v1/projects`）的
    合法输入集（新增拒绝集并返回 422），对应评审第 7 项的服务端输入校验/
    健壮性缺陷修复，trivial 明确排除 public contract/protocol 与
    security/robustness 影响；
  - 未触发 high-risk：不改 schema/迁移、依赖图、发布/回滚/部署或错误类型；
    错误语义沿用现有 422 `invalid_input`；仓库处于 greenfield 阶段，未发现
    需要使用迁移或数据清洗的存量非法项目行（本地测试库如存在坏名也不影响
    新库约束，项目删除按 id 走既有 API/前端）。

## Background and Goal

- 现象（评审第 7 项，中低危，用户 2026-08-26 指示「不能创建CLI无法寻址的
  项目名」）：
  - 服务端 `SqliteProjectService::create`（`server/src/projects/service.rs:128`）
    只校验 `name.trim().is_empty()`，随后把原始 `name` 直接落库；
  - CLI `parse_target`（`cli/src/cli/args.rs:120`）把 `<server>/<project>` 按
    `/` 严格分段、对每个字段做 `trim()`，因此包含 `/` 或带首尾空白的项目名
    无法被任何项目型 CLI 命令精确寻址。
- 目标：服务端创建入口拒绝这两类名称（422 且不落库），使「能创建」与
  「CLI 可精确寻址」保持一致；CLI 解析本身已严格，无需改动。

## Scope

### In scope

- `server/src/projects/service.rs`（change_id `fh-project-name-cli-addressable-validate`）：
  - 在权限判定与 INSERT 之前对项目名做最小拒绝集校验，抽成
    `validate_project_name`：
    - trim 后为空：仍按现有语义拒绝（`project name required`，422）；
    - 原始 name 含 `/`：拒绝（422），错误消息说明不得包含 `/`；
    - `name != name.trim()`（存在首尾空白）：拒绝（422），错误消息说明不得
      含首尾空白；
  - 通过校验后仍按原始 name 落库（只拒绝、不静默 trim 或改写）。
- `server/tests/unit/projects.rs`（change_id `fh-project-name-cli-addressable-tests`）：
  - 新增回归用例 `create_rejects_cli_unaddressable_names`：
    - 拒绝集：`a/b`、`/x`、`x/`、`  `（纯空白，既有语义）、` demo`、
      `demo `、` demo `；
    - 逐项断言 `err.kind == ProjectErrorKind::InvalidInput`，且 projects 表
      行数不变（无一落库）；
    - 放行集：`demo`、`demo-1`、`demo_1`、`demo 1`（内部空格）、`Demo`、
      `项目`——这些名称经 CLI 分段/trim 语义可精确寻址，保持可创建。
- `docs/api/v1-contract.md`（change_id `fh-project-name-cli-addressable-contract`）：
  - `POST /api/v1/projects` 一行注明：项目名 trim 后非空、不得包含 `/`、
    不得含首尾空白，非法返回 422（与 CLI `<server>/<project>` 精确寻址格式
    一致）。

### Out of scope（非目标）

- 不采用 admin-web 已有的完整命名格式 `^[a-z0-9][a-z0-9_-]*$`：评审与用户
  口径只要求「CLI 可寻址」，大写、Unicode、内部空格等名称 CLI 仍能精确寻址，
  不做额外格式收紧。
- 不改 CLI 的 `parse_target` 与任何命令、不改版本/app 名校验（040 已覆盖
  ）、不新增 API 端点或错误码。
- 不做存量数据迁移/清理：greenfield 无已知存量非法行；既有坏名（若有）仍可
  经 web/API 按项目 id 删除，不在本任务范围。
- 不修改测试基建、不运行仓库级格式化、不触碰 005/048 等在制未提交任务的
  改动。

## Requirement Review

- 需求合理：服务端创建校验与 CLI 目标寻址协议是同一套命名空间约束的两端，
  服务端放行而 CLI 永远无法寻址会制造不可操作项目，应在创建入口消除。
- 方向选择：采用「最小拒绝集」而非「统一改为 web 的 NAME_RE」——以 CLI
  分段/trim 机制为准，只拒绝 `/` 与首尾空白，避免超出用户口径的格式限制；
  采用「拒绝」而非「自动 trim 后存储」，因为用户明确要求「不能创建」，且
  静默改写名称会改变客户端所见输入、并可能让不同原始名称撞名。
- 材料风险/权衡：拒绝集以当前 CLI 目标解析（`/` 分段 + trim）为唯一依据；
  若未来 CLI 目标语法变化或 web 要求更严格命名，需要另行修订契约。
- 待确认问题：无。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-project-name-cli-addressable-validate | `create` 拒绝含 `/` 或存在首尾空白的项目名，返回 422 且不落库 | 仅 `SqliteProjectService::create` 创建入口；名称不静默改写 | 最小拒绝集，不限定大小写/Unicode/内部空格 | 单元用例覆盖拒绝集与放行集、projects 行数不变 | 不迁移存量、不改存储语义 |
| P-002 | fh-project-name-cli-addressable-tests | 新增 CLI 可寻址性回归用例 | `server/tests/unit/projects.rs` | 回归由既有单元测试层覆盖 | 定向 `cargo test -p filehub-server --test unit_tests` 通过 | 不引入集成测试或新基建 |
| P-003 | fh-project-name-cli-addressable-contract | v1 契约写明项目名拒绝集与 422 | `docs/api/v1-contract.md` | 文档与实现保持一致 | 契约表含 `/`/首尾空白拒绝说明 | 不新增端点或错误码 |

## Success Criteria

- 直接 API `POST /api/v1/projects`（绕过 web 前端）提交含 `/` 或首尾空白的
  项目名返回 422 `invalid_input`，且不产生 projects 行；
- `demo`、内部空格、大写、Unicode 等 CLI 可精确寻址的名称仍可正常创建；
- 空白名仍按既有的 422 `project name required` 拒绝；
- `cargo test -p filehub-server --test unit_tests` 受影响回归全部通过
  （新增用例 + 既有项目用例不回归），`cargo clippy -p filehub-server --tests`
  无本任务新增告警；
- 契约文档与实现一致；按 standard 流程产出
  `docs/changes/049-project-name-cli-addressable.md` 与任务包
  `completion-report.md`（中文正文），经 `lower-tier-check.py` completion
  profile 校验通过。

## Risks

- 最小拒绝集边界：大写、Unicode、内部空格等不在拒绝集内，CLI 可寻址但可能
  与某些客户端输入习惯不一致；如需进一步收敛，须另行提出契约变更。
- 存量坏名：greenfield 阶段无已知存量非法行，本任务不做数据清洗；若某个
  本地库在修复前已存在坏名，该项目仍可经 web/API 按 id 删除。
- 在制工作树：仓库存在多个在制未提交任务改动，本任务只修改提案列出的三个
  文件，验证以受影响子模块定向为准，不做仓库级格式化。
