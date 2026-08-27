# 拒绝 CLI 无法寻址的项目名：`/` 与首尾空白（评审第 7 项中低危）

- Status: complete
- Owner module: filehub（filehub-server）
- Task manifest: docs/versions/v0.1/modules/filehub/049-project-name-cli-addressable/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/049-project-name-cli-addressable/proposal.md
- Affected paths: server/src/projects/service.rs；server/tests/unit/projects.rs；docs/api/v1-contract.md
- Explicit tier override: none
- Expanded high-risk packet: none
## Approach

- `SqliteProjectService::validate_project_name` 实现最小拒绝集：trim 后为空
  仍按既有语义拒绝（`project name required`，422）；原始 name 含 `/` 时拒绝
  （`/` 是 CLI `<server>/<project>` 目标串的字段分隔符）；`name != name.trim()`
  （带首尾空白）时拒绝——CLI 对每个目标字段先 trim 再精确匹配，这类名称
  永远无法被项目型 CLI 命令寻址。
- `create` 在校验处直接调用该函数（位于 owner 判定之后、权限判定与 INSERT
  之前），拒绝时返回 422 `invalid_input` 且不落库；通过校验后仍按原始 name
  落库，不做静默 trim 或改写。
- 按用户确认口径采用最小拒绝集：不引入 admin-web 的
  `^[a-z0-9][a-z0-9_-]*$` 格式限制；大写、Unicode、内部空格等名称经 CLI
  分段/trim 语义仍可精确寻址，保持可创建。
- 契约文档同步说明拒绝集与 422；单元回归覆盖拒绝集、放行集与“无一落库”。

## Risk Screen

- Public contract, protocol, or CLI change: yes —— `POST /api/v1/projects`
  输入集收紧（项目名含 `/` 或首尾空白时 422 `invalid_input`；422 错误码已有，
  非新增端点/错误码）。该变更属已确认提案范围。
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no —— 关闭既有输入校验缺口，
  不新增信任面；不改变认证/授权边界。
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests` 58/58 全通过
  （含新增 `create_rejects_cli_unaddressable_names`）；`cargo clippy -p
  filehub-server --tests --message-format short` 无本次改动新增告警；
  `rustfmt --edition 2024 --check` 复查新增代码段（文件级 check 仅报告其它
  在制任务 hunk，本次不进行仓库级格式化）
- Result: pass
- Residual risk or follow-up: 存量坏名不做迁移（greenfield 无已知存量行）——
  若本地库在修复前已有此类项目，仍可经 web/API 按 id 删除；最小拒绝集之外的
  名称（大写/Unicode/内部空格）按确认口径保持可用，如需收敛须另行契约变更。
