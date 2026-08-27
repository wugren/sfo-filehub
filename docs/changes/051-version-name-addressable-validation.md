# 版本名可寻址性与下载响应头 quoted-string 安全修复

- Status: complete
- Owner module: filehub（filehub-server）
- Task manifest: docs/versions/v0.1/modules/filehub/051-version-name-addressable-validation/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/051-version-name-addressable-validation/proposal.md
- Affected paths: server/src/versions/service.rs；server/src/contract/mod.rs；server/tests/unit/versions.rs；docs/api/v1-contract.md
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- `SqliteVersionService::validate_version` 在 040 最小拒绝集（trim 后为空/
  `latest`；原始输入含 `/`、`?`、`#`、控制字符）基础上新增：trim 后整段为
  `.` 或 `..` 时拒绝（浏览器标准 URL 解析会归一化 `./`/`../` 路径段，此类版本
  创建后不可按名称寻址）；原始输入含 `"` 或 `\` 时拒绝（二者是
  Content-Disposition quoted-string 元字符，直接拼入 filename 参数会截断或
  造成歧义）。字符类检查沿用 trim 前扫描策略；`.`/`..` 判定基于 trim 后整段，
  不误伤 `1.0.0`、`1.0.0.1`、`..rel` 等含点名称。
- `contract::set_download_headers` 增加 `escape_disposition_filename` 防御性
  转义：写入 `filename="..."` 前按 RFC 9110 quoted-string 规则把 `\` 转义为
  `\\`、`"` 转义为 `\"`，并移除控制字符。正常名称（如 `{id}-1.0.0-ui`）
  生成的响应头与现契约逐字一致；即使未来其它调用方传入不安全名称，也不会再
  生成可截断/歧义的非法响应头。
- 回归：扩展 `create_version_rejects_reserved_and_route_unsafe_names`
  拒绝集（`.`、`..`、` . `、` .. `、`"`、`\`）与放行集（`1.0.0.1`、
  `..rel`），断言拒绝不落库；contract 新增 `#[cfg(test)]` 响应头单测，覆盖
  quoted-string 元字符转义、控制字符移除与正常名称逐字不变。
- 契约文档同步：`POST /versions` 行补充新增拒绝集；下载行注明 filename
  按 RFC 9110 quoted-string 转义 `"`/`\` 并移除控制字符。

## Risk Screen

- Public contract, protocol, or CLI change: yes —— `POST /versions` 输入集
  继续收紧（拒绝 `.`/`..` 整段与 `"`、`\`，非法 422）；下载行文档同步说明
  响应头转义规则，正常下载头格式不变。属已确认提案范围。
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no —— 关闭评审第 5 项中危的
  可寻址性/响应头截断缺口，不新增信任面。
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server --lib contract::`
  （2 项 contract 响应头单测）、`cargo test -p filehub-server --test unit_tests`
  （62/62，含扩展的版本名校验回归）、`cargo test -p filehub-server --test
  api_integration opaque_bytes_upload_missing_version_download_headers_and_empty_version`
  （下载头契约集成断言 1/1）
- Result: pass
- Residual risk or follow-up: 版本名最小拒绝集边界（空格、Unicode 等仍放行）
  与 app 名 `.`/`..` 均按提案列为范围外边界；clippy 无本任务新增告警；工作树
  其余在制改动未触碰。
