# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/049-project-name-cli-addressable.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - 服务端：`SqliteProjectService::validate_project_name` 在
    `create` 的 owner 判定之后、权限判定与 INSERT 之前完成校验——trim 后为空
    仍按既有 `project name required`（422）拒绝；原始 name 含 `/` 时拒绝；
    `name != name.trim()`（首尾空白）时拒绝。校验通过后仍按原始 name 落库，
    不做静默 trim 或改写；错误经既有 `InvalidInput → 422` 映射返回。
  - 回归：`server/tests/unit/projects.rs` 新增
    `create_rejects_cli_unaddressable_names`，拒绝集（`a/b`、`/x`、`x/`、
    纯空白、` demo`、`demo `、` demo `）逐项断言
    `ProjectErrorKind::InvalidInput` 且 projects 表 0 行新增；放行集
    （`demo`、`demo-1`、`demo_1`、`demo 1`、`Demo`、`项目`）全部可创建且
    名称原样存储。
  - 契约：`docs/api/v1-contract.md` 的 `POST /api/v1/projects` 行注明
    项目名约束与 422（与 CLI `<server>/<project>` 精确寻址语义一致）。
- Handoff: `cargo test -p filehub-server --test unit_tests` 58 项全通过
  （57 项既有 + 1 项新增）；`cargo clippy -p filehub-server --tests` 无本次
  改动新增告警；未做仓库级 rustfmt（在制工作树其它任务的未格式化 hunk 保留
  原样，本次新增代码格式清洁）。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-project-name-cli-addressable-validate | `create` 拒绝含 `/` 或存在首尾空白的项目名，返回 422 且不落库 | proposal.md P-001 | service.rs `validate_project_name` + `create` 前置调用；单元用例断言 422 次数、projects 行数 0 | 匹配 | pass |
| fh-project-name-cli-addressable-tests | 新增 CLI 可寻址性回归用例 | proposal.md P-002 | `create_rejects_cli_unaddressable_names` 通过；既有项目/版本/账号用例不回归 | 匹配 | pass |
| fh-project-name-cli-addressable-contract | v1 契约写明拒绝集与 422 | proposal.md P-003 | `docs/api/v1-contract.md` POST /projects 行含 `/`/首尾空白拒绝说明 | 匹配 | pass |
## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `validate_project_name` 的 trim 判定与 `/` 字符扫描、`create` 调用顺序、`http.rs` 的 `InvalidInput → 422` 映射、`cli/src/cli/args.rs:120` parse_target（`/` 严格分段 + 每字段 trim）、`apiclient/mod.rs` 按名精确匹配 | 代入纯空白/全角空格/NBSP 等 trim 可剥除输入、`/` 位于开头/中间/结尾、带首尾空白的同名伪装（`demo ` 与 `demo`）、大写/Unicode/内部空格放行集、空串 | 无绕过：trim 后为空仍拒绝；`/` 三位置均拒绝；首尾空白统一拒绝，无法伪装成已存在名；放行集与 CLI 可寻址语义一致，未引入额外格式限制 | pass |
| boundaries-and-failure-paths | 校验发生在权限判定与 INSERT 之前、拒绝路径是否落库、错误消息在错误 JSON 中的表示、`name` 存储是否被改写 | 拒绝后查询 projects 表计数为 0；`InvalidInput` 分支与 422 错误码既有映射未改动；通过校验的名称原样落库（断言 `created.name == good`）；重复名 409、权限 403 语义未受影响 | 无部分写入/无静默改写；错误仅固定文案，不含用户控制的控制字符；无 schema/迁移/新错误码 | pass |
| regression-and-side-effects | 既有项目创建/删除/级联/分页/协作者/token 范围用例、版本与账号用例、契约文档与实现一致性 | 检查本任务是否触碰 CLI/web/测试基建、是否引入 clippy 新告警、`rustfmt --check` 的差异是否都属于本任务、任务包 scope_paths 与实际 diff 是否一致 | 单元 58 项全绿；clippy 无本次新增告警（projects/service.rs:100/277 与 projects.rs:561 的告警为其它在制改动遗留）；rustfmt 仅报告其它在制任务 hunk；实际改动仅提案三文件 + 任务文档 | pass |

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests`（58/58 全通过，
  含新增 `create_rejects_cli_unaddressable_names`）；`cargo clippy -p
  filehub-server --tests --message-format short`（无本次改动新增告警）；
  `rustfmt --edition 2024 --check` 复查新增代码段（文件级 check 仅报告其它
  在制任务 hunk，本次提交不进行仓库级格式化）
- Result: pass
- Exception reason: not-applicable（无需例外；在制工作树格式化差异按共享工作树
  规则保留，不归本任务）

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 放行集含 `Demo`、`项目`、`demo 1` | 按确认口径保持最小拒绝集；大写/Unicode/内部空格经 CLI 分段/trim 后可精确寻址，如需进一步收敛须另有契约变更 | no |
| F-2 | low | `rustfmt --check` 在 service.rs 导入块与 projects.rs 分页/scope 用例处报差异 | 差异来自其它在制任务未格式化 hunk，非本次改动；本任务新增代码段格式清洁，未做仓库级格式化以保护共享工作树 | no |
| F-3 | low | clippy 输出 projects/service.rs:100、277 与 projects.rs:561 告警 | 均为既有在制改动遗留，本任务未触碰相关行；无 049 引入的告警 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001～P-003 全部落地：`create` 在落库前拒绝含 `/` 或首尾空白的
  项目名（422、projects 0 行），放行集保持 CLI 可精确寻址；单元 58 项全绿，
  clippy 无本任务新增告警，契约文档同步；F-1～F-3 均为非阻塞低危记录。
