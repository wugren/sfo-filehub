# filehub-cli server 身份与传输按 Docker 语义（无协议头 + HTTPS 优先/HTTP 降级）

- Status: complete
- Owner module: filehub / filehub-cli
- Task manifest: docs/versions/v0.1/modules/filehub/015-cli-server-identity-docker/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/015-cli-server-identity-docker/proposal.md
- Affected paths: `cli/src/apiclient/mod.rs`、`cli/src/credential_store/mod.rs`、`cli/src/cli/args.rs`、`cli/tests/common/mod.rs`、`cli/tests/api_integration.rs`、003-filehub-cli 设计文档
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

1. 服务器身份统一为 `host[:port]`：`normalize_server` 去掉协议头与路径；登录/续期/logout/凭据查找全部按身份 key 处理。
2. 旧配置兼容：已有 `http://host:port` / `https://host:port` key 在精确 key 未命中时按剥协议身份匹配（等价 Docker `ConvertToHostname`），无需用户重新 login。
3. 传输候选端点：`FilehubClient` 内部维护 `https://identity` 优先、loopback 身份追加 `http://identity` 的候选列表；首次请求连接失败时降级重试，4xx/5xx 响应不降级；下载流在响应头已返回后不再降级重试。
4. 明文 HTTP 仅对 `localhost`/`127.0.0.0/8`/`::1` 开放，安全面与 Docker 默认一致；不新增 insecure-registries 配置。

## Risk Screen

- Public contract, protocol, or CLI change: yes — server 参数不再要求协议头、凭据 key 改为身份、请求层新增 HTTP 降级；已通过旧 key 兼容与 loopback-only 边界控制，且 003 CLI 提案确认按此语义执行。
- Persistent data, schema, or migration change: no — 配置文件格式不变，旧 key 只读兼容，不写迁移。
- Security, privacy, or trust-boundary change: no — 明文 HTTP 严格限制 loopback，凭据仍只存本机用户配置目录。
- Concurrency, lifecycle, or runtime integration change: no — 单请求候选重试，401 续期与下载部分字节不重试语义保持不变。
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-cli`（单元 10 + api_integration 14 + dv 4 全通过），含新增「无协议登录落盘身份 key」「旧 http 凭据 + 无协议 publish」「无协议登录并发布全流程」三项定向用例；`rustfmt --edition 2024 --config skip_children=true --check` 覆盖本次实际改动的 5 个源/测试文件；`cargo check --workspace` 通过。
- Result: passed
- Residual risk or follow-up: 非 loopback 明文 HTTP 与 insecure-registries 配置清单明确不在本任务范围；后续如需开放，需单独提案扩展安全边界。
