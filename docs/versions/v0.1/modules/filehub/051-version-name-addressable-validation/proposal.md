---
task_manifest: task.yaml
status: approved
---

# 版本名可寻址性与下载响应头安全修复（评审第 5 项，中危）

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 需求明确，改动局限在 filehub-server 的 versions 创建校验、下载响应头构造
    与其单元回归 + v1 契约文档，属于有界单项目 bugfix；
  - 不满足 trivial：本次变更再次收紧了公开 v1 API 中版本号的合法输入集
    （拒绝整段 `.`/`..` 与 `"`、`\`），并直接修复下载 `Content-Disposition`
    quoted-string 可被截断/歧义的中危缺陷，trivial 明确排除 public contract
    与 security 影响；
  - 未触发 high-risk：不改 schema/迁移、依赖图、发布/回滚/部署、错误类型
    或路由；错误语义沿用现有 422 `invalid_input`；greenfield 阶段无存量非法
    版本行需要数据清洗。

## Background and Goal

- 现象（评审第 5 项，中危）：`create_version` 的版本名校验（040 任务落地，
  `server/src/versions/service.rs` 的最小拒绝集）仍放行整段 `.` 与 `..`。
  浏览器的标准 URL 解析会把 `versions/./download` 归一化为
  `versions/download`、把 `versions/../download` 归一化到上一级路径，因此这类
  版本能通过 API 创建，却无法通过 admin-web 或常规 HTTP 客户端按名称访问。
- 现象（同项）：版本名还允许双引号 `"` 与反斜杠 `\`，而下载响应在
  `server/src/contract/mod.rs:244` 的 `set_download_headers` 中直接把版本名
  插入 `attachment; filename="{name}.tar.gz"` 的 quoted-string。`"` 会提前
  终止 filename 参数、`\` 会制造转义歧义，造成文件名参数截断或参数歧义。
- 目标：在版本创建入口拒绝不可寻址的整段 `.`/`..` 与 `"`、`\`；同时对下载
  响应头做防御性转义，保证即使未来有其它调用方传入不安全文件名，生成的
  Content-Disposition 头也始终是合法 quoted-string。

## Scope

### In scope

- `server/src/versions/service.rs`：`validate_version` 在既有拒绝集
  （trim 后为空/`latest`；原始输入含 `/`、`?`、`#`、控制字符）基础上新增：
  - trim 后整段等于 `.` 或 `..` 时拒绝（URL 路径归一化后不可寻址）；
  - 原始输入含 `"` 或 `\` 时拒绝（保持与控制字符同样的“扫描 trim 前原始输入”
    策略，避免边缘空白剥除后漏判）。
  `create_version` 在校验前返回 `VersionError::invalid_input`（HTTP 422），
  不落库；`1.0.0`、`1.0`、`Latest`、含空格、含其它点段（如 `1.0.0.1`、
  `1.0.0-rc1`）等不在拒绝集内的名称仍放行。
- `server/src/contract/mod.rs`：`set_download_headers` 在拼接 quoted-string
  前按 RFC 9110 quoted-string 规则转义 `\`（`\\`）与 `"`（`\"`），并移除
  HeaderValue 无法承载的控制字符；正常名称（如 `{id}-1.0.0-ui`）生成的
  头与现契约逐字不变，既有集成断言不回归。
- `server/tests/unit/versions.rs`：扩展既有回归用例，覆盖 `.`、`..`、`"`、
  `\` 拒绝且不落库，放行集（`1.0.0`、`1.0.0.1`、`..rel` 等）保持可创建。
- `server/src/contract/mod.rs`（`#[cfg(test)]`）：新增 `set_download_headers`
  最小响应实现单测，断言对含 `"`/`\`/控制字符的输入生成的 Content-Disposition
  为合法 quoted-string（转义后值）且 `HeaderValue::from_str` 成功。
- `docs/api/v1-contract.md`：`POST .../versions` 行补上 `.`/`..` 整段与
  `"`、`\` 拒绝说明；下载行注明响应头对 filename 作 quoted-string 转义。

### Out of scope

- 不限制版本名格式与长度：只扩展拒绝集，不引入 semver、长度上限或严格字符集。
- 不改读取/发布/锁定等路径参数里的非创建版本名校验语义：非法名称无法创建，
  greenfield 中不存在存量非法行，此类查询按现有语义返回 404。
- 不改 `validate_app` / app 名规则：下载路由用 `?app=` 查询参数寻址，`.`/`..`
  仍可寻址；app 名原本就不允许 `"`/`\`/控制字符。如后续要连带收紧 app 名，
  需另行确认。
- 不改 admin-web / CLI：服务端 422 会透传展示，前端无需改动。
- 不改 schema/迁移、路由、错误类型；不触碰其它在制未提交任务改动；不运行
  仓库级格式化。

## Requirement Review

- 需求合理：`validate_version` 声称是路由/下载响应头安全的权威拒绝集，却漏掉
  URL 归一化特例（整段 `.`/`..`）与 quoted-string 元字符（`"`、`\`），与
  040 任务的目标直接冲突；评审第 5 项准确指出了“能创建但不可寻址/响应头可被
  截断”的两个真实后果。
- 方向选择：创建入口做权威拒绝（与服务端单一权威校验原则一致）；`.`/`..`
  只按“trim 后整段”拒绝，避免误伤 `1.0.0` 等含点版本；`"`/`\` 按原始输入
  全量拒绝，堵住 trim 边缘绕过；下载响应头再做 quoted-string 转义作为
  defense-in-depth，修复“直接把版本名插入”这一具体缺陷点。
- 材料风险/权衡：防御性转义对正常名称输出逐字不变（既有下载契约断言保持），
  对已不可能被创建的非法名称仅改变行为；拒绝集扩大会让极少数现命名空间收紧，
  但 greenfield 阶段无存量数据，且该集合恰好是 URL/HTTP 层的不安全字符。
- 待确认问题：无。app 名是否同样拒绝 `.`/`..` 属范围外边界，提案中明确列为
  non-goal，如需一并处理请另行告知。

## Approval Record

- approver: 用户
- approval_date: 2026-08-26
- user_statement: 用户 2026-08-26 对「版本名可寻址性与下载响应头安全修复
  （评审第 5 项，中危）」提案回复「确认」，最终 tier 为 standard。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-version-addressable-validate | `create_version` 拒绝整段 `.`/`..` 与含 `"`/`\` 的版本名，422 且不落库 | 仅版本创建入口；读取/`latest` 语义不变 | 拒绝集扩展，不限制格式/长度 | 单元回归覆盖新增拒绝集与放行集 | 不改路径参数查询语义 |
| P-002 | fh-version-disposition-sanitize | `set_download_headers` 对 filename 作 RFC 9110 quoted-string 转义（`\`→`\\`、`"`→`\"`、移除控制字符） | 仅下载响应头构造 | 正常名称输出逐字不变，非法名称不会被原样拼入头 | 响应头单测断言转义值与 HeaderValue 可解析 | 不使用 `filename*`、不改变正常头格式 |
| P-003 | fh-version-addressable-tests | 新增/扩展单元回归覆盖拒绝集、放行集与响应头转义 | server 单元测试 | 回归由既有单元层覆盖 | 定向 cargo test 通过，既有用例不回归 | 不引入集成测试或新基建 |
| P-004 | fh-version-addressable-contract | v1 契约文档写明新增拒绝集与响应头转义 | docs/api/v1-contract.md | 文档与实现保持一致 | 契约表含对应说明且与实现一致 | 不新增端点或错误码 |

## Success Criteria

- `POST /versions` 对 trim 后为 `.`/`..` 的版本名及含 `"`/`\` 的版本名返回
  422 `invalid_input`，且不产生 versions 行；
- `1.0.0`、`1.0`、`Latest`、`1.0.0.1`、`..rel`、含空格的名称等不在拒绝集
  内的版本可正常创建、查询、发布与下载（既有生命周期用例保持通过）；
- `set_download_headers` 对含 `"`/`\` 的输入生成合法 quoted-string 响应头，
  对正常名称生成的 `attachment; filename="{id}-{version}-{app}.tar.gz"` 与
  现契约逐字一致；
- `server` 定向测试通过（受影响子模块单元用例全绿），clippy 无本任务新增告警；
- 按 standard 流程产出 `docs/changes/051-version-name-addressable-validation.md`
  与任务包 `completion-report.md`（中文正文），并经 `lower-tier-check.py`
  completion profile 校验通过。

## Risks

- 最小拒绝集边界：空格、Unicode 等未列入字符仍允许进入版本名，其路由/下载
  行为依赖既有 URL 编码与 HeaderValue 承载；本次不额外收紧或验证，如后续发现
  新的破坏字符需扩展拒绝集。
- app 名 `.`/`..` 不在本次范围：下载以查询参数寻址仍可访问；PUT/DELETE 的
  app 路径段若由浏览器前端构造同样受 URL 归一化影响，本次不改，留作范围外
  边界记录。
- 在制工作树：仓库存在大量未提交的在制任务改动，本任务只修改提案列出文件，
  验证范围以受影响子模块定向为准。
