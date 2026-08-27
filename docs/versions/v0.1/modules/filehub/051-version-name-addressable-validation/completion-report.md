# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/051-version-name-addressable-validation.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - 服务端：`SqliteVersionService::validate_version` 新增拒绝 trim 后整段为
    `.`/`..` 的版本名（URL 归一化后不可寻址），并拒绝原始输入含 `"` 或 `\`
    的版本名（Content-Disposition quoted-string 元字符）；沿用 422
    `invalid_input`，校验在任何事务/权限副作用前完成且不落库；
  - 响应头：`contract::set_download_headers` 通过
    `escape_disposition_filename` 按 RFC 9110 quoted-string 规则转义
    `\`/`"` 并移除控制字符；正常名称的头输出逐字不变；
  - 回归：扩展版本名校验拒绝集/放行集用例；contract 新增 2 项响应头单测；
  - 契约：`docs/api/v1-contract.md` 同步 `POST /versions` 拒绝集与下载头
    转义说明。
- Handoff: unit_tests 62/62（含 lib contract 2 项）、版本名定向回归 1/1、
  下载头契约集成断言 1/1 全通过；clippy 无本任务新增告警。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-version-addressable-validate | `create_version` 拒绝整段 `.`/`..` 与含 `"`/`\` 的版本名，422 且不落库 | proposal.md P-001 | service.rs `validate_version` 新增判定并前置调用；单元用例断言拒绝集 + versions 表 0 行 | 匹配 | pass |
| fh-version-disposition-sanitize | `set_download_headers` 对 filename 作 RFC 9110 quoted-string 转义 | proposal.md P-002 | contract/mod.rs `escape_disposition_filename` + 2 项单测；下载契约集成断言逐字不变 | 匹配 | pass |
| fh-version-addressable-tests | 新增/扩展单元回归覆盖拒绝集、放行集与响应头转义 | proposal.md P-003 | `create_version_rejects_reserved_and_route_unsafe_names` 扩展；`contract::tests` 2 项通过 | 匹配 | pass |
| fh-version-addressable-contract | v1 契约文档写明新增拒绝集与响应头转义 | proposal.md P-004 | docs/api/v1-contract.md POST 行 + 下载行已更新 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `validate_version` 的 trim 后整段判定与 trim 前原始字符扫描、`create_version` 调用点、`escape_disposition_filename` 字符分支 | 反向推演：带空白的 ` . `/` .. `（trim 后命中拒绝）、引号/反斜杠嵌入名称、控制字符 + 引号组合、`1.0.0`/`..rel`/`1.0.0.1` 不应误伤；转义顺序 `\` 先于 `"`，无状态混淆 | 新增 ` . `/` .. ` 用例实测拒绝；引号/反斜杠用例实测拒绝；放行集全部可创建；转义结果与 RFC 9110 quoted-string 一致 | pass |
| boundaries-and-failure-paths | 校验在权限/事务前执行且拒绝不落库；URL 归一化语义（仅整段点号）、下载头生成失败路径；`publish_app`/`lock`/`delete_app` 未改动 | 拒绝后 `versions` 表计数为 0；整段点号是 URL 归一化唯一触发形态，`foo.`/`..foo` 不受影响；`set_download_headers` HeaderValue 构造不再可能被 `"`/`\` 破坏；控制字符在边界被丢弃 | 无绕过；无落库；读取/发布路径语义保持 404/既有行为；响应头恒为合法 quoted-string | pass |
| regression-and-side-effects | 工作树基线 diff：与 pre-edit 基线逐文件对比，仅含 4 个声明文件的意图 hunk；契约文档与实现一致性；既有生命周期/权限/上传用例 | 全量 unit 62/62（含 040 及后续在制任务用例）；集成下载头契约断言（`{pid}-1.0.0-ui`）逐字不变；cli 警告维持原状，无本任务新增项 | 无回归；未触碰其它在制改动；未引入仓库级格式化 | pass |

## Verification

- Targeted check: `cargo test -p filehub-server --lib --test unit_tests`
  （62/62，含 `contract::tests` 2 项与版本名定向回归）、`cargo test -p
  filehub-server --test api_integration
  opaque_bytes_upload_missing_version_download_headers_and_empty_version`（1/1）
- Result: pass
- Exception reason: 无。api_integration 仅单独运行下载头契约用例；全量集成
  含其它在制任务用例的状态共享（既有已知并行 409 干扰），不在本次变更范围。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 放行集含 `1.0.0.1`、`..rel`、空格、Unicode | 按用户既有口径保持最小拒绝集，其它潜在 URL/Header 不便字符未收紧；如需进一步收敛需另行契约变更 | no |
| F-2 | low | `escape_disposition_filename` 静默丢弃控制字符 | 仅作为防御层生效；合法创建路径中版本名已不含控制字符，行为只影响未来非法调用方 | no |
| F-3 | low | `validate_app` 未拒绝 app 名 `.`/`..` | 提案明确列为 non-goal（下载以 `?app=` 寻址可访问；PUT/DELETE 路径段由浏览器构造仍受归一化影响），未改动 | no |
| F-4 | low | clippy 在 contract/mod.rs、versions/service.rs 存在既有告警 | 均为在制/存量告警（large Err、expect 后 is_none 等），本任务 hunk 无新增 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001～P-004 全部落地：`validate_version` 拒绝 `.`/`..` 整段与
  `"`/`\`（含 trim 绕过边界），`set_download_headers` 生成恒合法 quoted-string，
  回归与集成证据全绿；排除 hunk 后确认未触碰其它在制改动；F-1～F-4 均非阻塞
  范围外/防御性记录。
