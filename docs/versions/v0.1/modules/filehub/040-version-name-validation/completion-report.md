# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/040-version-name-validation.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - 服务端：`SqliteVersionService::validate_version` 在 `create_version` 事务
    与权限检查前校验版本名——trim 后为空或字面值 `latest` 拒绝；trim 前原始输入
    含 `/`、`?`、`#` 或 Unicode 控制字符拒绝，返回 422 `invalid_input` 且不
    落库；不限制格式与长度，`latest` 读取/下载语义不变；
  - 回归：`server/tests/unit/versions.rs` 新增
    `create_version_rejects_reserved_and_route_unsafe_names`，覆盖拒绝集
    （含 trim 后命中 `latest`、纯空白、`/`、`?`、`#`、C0/DEL/C1 控制字符）与
    放行集（`1.0`、`Latest`、含空格与预发布风格名称）；
  - 契约：`docs/api/v1-contract.md` 注明 `POST .../versions` 的拒绝集与 422。
- Handoff: `cargo test -p filehub-server --test unit_tests` 48 项全通过
  （47 项既有 + 1 项新增）；`dv_tests` 2 项通过；`api_integration`
  `upload_security_boundaries` 单独运行通过；clippy 无本任务新增告警。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-version-unsafe-validate | `create_version` 拒绝保留字 `latest` 与 `/`、`?`、`#`、控制字符，422 且不落库 | proposal.md P-001 | service.rs `validate_version` + `create_version` 前置校验；单元用例断言 422 次数与 versions 表 0 行 | 匹配 | pass |
| fh-version-unsafe-tests | 新增保留字/不安全字符回归用例 | proposal.md P-002 | `create_version_rejects_reserved_and_route_unsafe_names` 通过；既有版本生命周期/输入校验用例不回归 | 匹配 | pass |
| fh-version-unsafe-contract | v1 契约文档写明拒绝集与 422 | proposal.md P-003 | `docs/api/v1-contract.md` POST /versions 行含 `latest`/`/`/`?`/`#`/控制字符拒绝说明 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `validate_version` 的 trim 前字符扫描与 trim 后空值/`latest` 判定、`create_version` 调用点、错误消息构造 | 反向推演 trim 绕过：边缘控制字符（U+0085 尾部、换行/制表符）、纯空白、`latest` 前后空白、`Latest` 大小写变体、路径/查询/片段分隔符与 C1 区控制字符；开发期实测发现并修复 U+0085 被 `trim()` 剥除的绕过路径（改为扫描原始输入） | 无绕过：扫描原始输入后所有控制字符均拒绝；`latest` 判定基于 trim 后值，前后空白不会漏判；放行集行为与提案一致 | pass |
| boundaries-and-failure-paths | 校验顺序（事务与权限检查之前）、拒绝路径是否落库、报错字符在错误 JSON 中的安全表示、`latest` 读取路由未动 | 拒绝后查 `versions` 表计数为 0；错误消息对控制字符用 `{:?}` 转义，不引入不可控字符；`get(None)`/`latest` 语义与 `http.rs` 无改动；超长/任意格式名称不进入拒绝集（按用户口径） | 校验在任何副作用前完成；无落库、无新错误码、无路由改动 | pass |
| regression-and-side-effects | 既有版本生命周期、权限分支、输入校验、上传/下载用例；契约文档与实现一致性 | 检查 `publish_app`/`lock`/`delete_app` 未替换为空校验（路径参数查询语义保持 404）；对比既有 `version_and_app_input_validation_branches` 约束无冲突；确认仅改提案列出文件 | unit 48 + dv 2 全通过；clippy 无新增告警；本任务未触碰其它在制改动 | pass |

## Verification

- Targeted check: `cargo test -p filehub-server --test unit_tests`（48/48）、
  `cargo test -p filehub-server --test dv_tests`（2/2）、
  `cargo test -p filehub-server --test api_integration upload_security_boundaries`
  （单独 1/1）
- Result: pass
- Exception reason: 全量并行 `api_integration` 下 `upload_security_boundaries`
  出现 409 干扰（`server/tests/api_integration.rs:143` 创建版本返回 409），
  单独运行通过；根因为另一在制任务新增的 `project_list_pagination_and_single_get`
  集成用例与本用例共享测试服务器/数据库状态，与本任务改动无关，未修改该在制
  任务代码。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 单元用例放行 `1.0`、`Latest`、含空格名称 | 按用户口径保持最小拒绝集，空格与大小写变体仍可作版本名；如需进一步收敛需另行契约变更 | no |
| F-2 | low | `validate_version` 扫描 trim 前原始输入 | trim 仍会规范化边缘普通空白（space），但控制字符一律拒绝；语义已在注释与用例中记录 | no |
| F-3 | low | `api_integration` 并行全量 1/7 失败、单独运行通过 | 并行干扰来自另一在制任务的集成测试共享状态（项目/版本重名 409），非本任务代码缺陷；该任务收敛后可全量复跑 | no |
| F-4 | low | 拒绝集仅字面值 `latest` | 与路由 `version == "latest"` 的特殊语义一致；`Latest`/`LATEST` 可创建并可精确查询，不产生关键字歧义 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001～P-003 全部落地：`create_version` 在校验通过前不产生任何
  versions 行，拒绝集（`latest`、`/`、`?`、`#`、控制字符）与放行集均经独立
  对抗用例覆盖（含真实发现的 trim 绕过路径）；unit 48 项与 dv 2 项全绿、
  clippy 无新增告警、契约文档同步；F-1～F-4 均为非阻塞低危记录。
