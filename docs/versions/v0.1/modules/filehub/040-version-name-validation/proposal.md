---
task_manifest: task.yaml
status: approved
---

# 版本名保留字与不安全字符校验（不限制格式）

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard

## Approval Record

- approver: 用户
- approval_date: 2026-08-25
- user_statement: 用户 2026-08-25 对修订后的「最小拒绝集（保留字 latest 与
  `/`、`?`、`#`、控制字符，不限制格式）”提案回复「确认」，最终 tier 为
  standard。
- Tier rationale / triggered boundaries:
  - 需求明确，改动局限在 filehub-server 的 versions 子模块创建入口与服务端
    契约文档 + 单元回归，属于有界单项目 bugfix；
  - 不满足 trivial：本次变更收紧了公开 v1 API 中版本号的合法输入集（拒绝
    保留字与不安全字符），并直接对应评审第 6 项的路由安全/健壮性缺陷修复，
    trivial 明确排除 public contract 与 security 影响；
  - 未触发 high-risk：不改 schema/迁移、依赖图、发布/回滚/部署、错误类型
    或路由；错误语义沿用现有 422 `invalid_input`；仓库处于 greenfield 阶段，
    无存量非法版本行需要数据清洗。
- Proposal and tier confirmation: 用户 2026-08-25 回复「确认」，采用 standard
  层级与上述最小拒绝集范围继续执行。

## Background and Goal

- 现象（评审第 6 项，中危）：`create_version` 只拒绝空字符串
  （`server/src/versions/service.rs`），而读取/下载路由把字面值 `latest`
  永久解释为“最新版本”（`server/src/versions/http.rs`），因此可以成功创建
  名为 `latest` 的版本却永远无法按名称精确查询；包含 `/`、`?`、`#` 或控制
  字符的版本名还可能造成路由不可达或下载 `Content-Disposition` 响应头生成
  失败。
- 目标（用户 2026-08-25 修订后的口径）：不限制版本命名格式，只做最小
  拒绝集——版本名（trim 后）不得是保留字 `latest`（与路由特殊语义一致的字面
  值），且不得包含 `/`、`?`、`#` 或任意控制字符；其余字符与格式保持可用。
- 范围说明：用户明确「不要限死格式了」，因此不再强制 `x.x.x`，也不设置
  长度上限；读取/下载路由的 `latest` 关键字语义保持不变（该名称无法再被
  创建，不存在二义性）。

## Scope

### In scope

- `server/src/versions/service.rs`：新增版本名安全校验——trim 后：
  - 等于 `latest` 时拒绝；
  - 含 `/`、`?`、`#` 任一个字符时拒绝；
  - 含任意控制字符（Unicode `char::is_control()`，覆盖 C0/DEL/C1）时拒绝。
  `create_version` 在事务前拒绝并返回 `VersionError::invalid_input`
  （HTTP 422），错误消息与现有 “version is required” 风格一致。
- `server/tests/unit/versions.rs`：新增回归用例——`latest`、含 `/`/`?`/`#`、
  含控制字符、`latest` 带空白 trim 后命中保留字等均拒绝；`1.0.0`、`1.0`、
  `Latest`（非字面值保留字）、`1.0.0 beta`（空格不在拒绝集）等仍可创建，
  既有版本生命周期用例保持通过。
- `docs/api/v1-contract.md`：`POST .../versions` 一行注明版本名不得为
  `latest`、不得包含 `/`、`?`、`#` 或控制字符，非法返回 422。

### Out of scope

- 不限制版本名格式：不强制 `x.x.x`，不校验字段数/数字/字母、不设置长度上限，
  不引入 semver 依赖。
- 不校验读取/发布/锁定等路径参数里的非创建版本名。非法名称无法被创建，
  greenfield 中不存在存量非法行，此类查询按现有语义返回 404，无需迁移。
- 不改 admin-web / CLI 代码：前端提示语无需变更，服务端 422 错误会透传展示。
- 不修改其它版本方法语义、错误类型、路由、schema/迁移；不触碰 005 等
  在制未提交任务改动；不运行仓库级格式化。

## Requirement Review

- 需求合理：`latest` 关键字冲突与 `/`、`?`、`#`、控制字符导致的路由/响应头
  风险均可通过“创建时最小拒绝集”消除，符合用户明确的修订口径，且比严格
  `x.x.x` 格式对现有/未来任意命名方式的侵入更小。
- 方向选择：`create_version` 单一入口做服务端权威校验，改动最小；控制字符
  用标准 Unicode `char::is_control()` 而非手写 ASCII 表，避免遗漏 C1 区；
  `/`、`?`、`#` 三个字符与 `latest` 精确字面值匹配路由实际保留语义，避免
  过度拒绝。
- 材料风险/权衡：最少拒绝集意味着空格、Unicode 等其它字符仍可出现在版本名
  （经 URL 编码后由现有路由/Header 机制承载）；本次只消除已确认会破坏路由/
  响应头的字符与保留字。若未来希望收敛命名规则，需另行扩展契约。
- 待确认问题：无（上一版的长度上限与 `x.x.x` 格式已按用户指示移除）。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-version-unsafe-validate | `create_version` 拒绝保留字 `latest` 与 `/`、`?`、`#`、控制字符，422 且不落库 | 仅版本创建入口；`latest` 读取语义不变 | 最小拒绝集，不限制格式/长度 | 单元用例覆盖拒绝集与放行集 | 不设格式/长度限制、不改其它方法语义 |
| P-002 | fh-version-unsafe-tests | 新增保留字/不安全字符回归用例 | `server/tests/unit/versions.rs` | 回归由既有单元测试层覆盖 | 定向 cargo test 通过，既有用例不回归 | 不引入集成测试或新基建 |
| P-003 | fh-version-unsafe-contract | v1 契约文档写明拒绝集与 422 | `docs/api/v1-contract.md` | 文档与实现保持一致 | 契约表含 `latest`/不安全字符拒绝说明 | 不新增端点或错误码 |

## Success Criteria

- `POST /versions` 对 `latest`、包含 `/`、`?`、`#` 或控制字符的版本名返回
  422 `invalid_input`，且不产生 versions 行；
- `1.0.0`、`1.0`、`Latest`、含空格的名称等不在拒绝集内的版本可正常创建、查询、
  发布与下载（既有 `version_lifecycle_create_publish_update_lock_delete` 用例
  保持通过）；
- 读取/下载 `latest` 仍返回最近创建的版本；
- `server` 定向测试通过（受影响子模块用例全绿）；
- 按 standard 流程产出 `docs/changes/040-version-name-validation.md` 与
  任务包 `completion-report.md`（中文正文），并经 `lower-tier-check.py`
  completion profile 校验通过。

## Risks

- 最小拒绝集的边界：空格、Unicode 等未列入的字符仍允许进入版本名，其路由/
  下载行为依赖既有 URL 编码与 Header 机制，本次不额外验证或收紧；如后续发现
  新的破坏字符，需要扩展拒绝集。
- `latest` 仅按字面值（小写）拒绝，与路由特殊语义一致；其它大小写变体
  （如 `Latest`）可创建并精确查询，不产生关键字歧义。
- 在制工作树：仓库存在未提交的在制任务改动，本任务只修改提案列出文件，
  验证范围以受影响子模块定向为准。
