# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/015-cli-server-identity-docker.md

## Delivery Summary

- Outcome: filehub-cli 的 server 地址统一按 `host[:port]` 身份匹配，参数不再要求
  协议头；登录/续期/logout/凭据查找按身份 key 执行，并对旧 `http://`/`https://`
  key 做剥协议兼容匹配（等价 Docker ConvertToHostname）。请求端点按 Docker 语义：
  先 `https://identity`，loopback（localhost、127.0.0.0/8、[::1]）在连接失败时
  降级 `http://identity`，非 loopback 不开放明文 HTTP。`filehub login
  http://127.0.0.1:8080` 后以 `filehub publish ... 127.0.0.1:8080` 执行不再报
  「未登录」。
- Handoff: 实现位于 `cli/src/apiclient/mod.rs`（`send_with_fallback` 与端点候选
  列表）、`cli/src/credential_store/mod.rs`（`normalize_server`、身份 key 与旧 key
  兼容匹配）、`cli/src/cli/args.rs`（SERVER 帮助文案）；测试位于
  `cli/tests/api_integration.rs` 与 `cli/tests/common/mod.rs`，新增「无协议登录落盘
  身份 key」「旧 http 凭据 + 无协议 publish」「无协议登录并发布全流程」定向用例。
  本机 `cargo test -p filehub-cli` 全量通过，`cargo check --workspace` 通过；无遗留
  阻塞问题。

## Proposal Consistency

| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-cli-server-identity | server 参数与本地凭据以 `host[:port]` 为身份；无协议/显式协议/尾随斜杠统一；旧带协议 key 兼容匹配 | proposal.md P-001 | `normalize_server` 去协议去路径；save/update/logout/current_credential 全部按身份；legacy_scheme_keys_are_matched_by_identity 与 legacy_http_credential_matches_no_scheme_publish 通过 | 交付与提案一致 | pass |
| fh-cli-https-first-fallback | 请求层 HTTPS 优先，loopback 在传输失败时降级 HTTP；非 loopback 仅 HTTPS | proposal.md P-002 | `FilehubClient::send_with_fallback` 按 `endpoint_bases` 顺序重试；`endpoint_candidates` 对 registry.example.com 只生成 HTTPS、对 127.0.0.1/localhost/[::1] 追加 HTTP；no_scheme_login_and_publish_workflow 通过 | 交付与提案一致 | pass |
| fh-cli-identity-regression | 全部 CLI 命令沿用同一语义并回归 | proposal.md P-003 | api_integration 14 项覆盖 login/publish/download/versions/new-version/lock/delete-app 与 401 续期；`cargo test -p filehub-cli` 全量通过 | 交付与提案一致 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | 审查 `normalize_server`、`credential_key`、`current_credential`、`endpoint_candidates` 与 `send_with_fallback` 全部分支 | 代入协议变体（http/https/无协议/尾斜杠）、旧 key 定位、非 loopback 候选、双重端点失败与 401 续期路径 | 身份归一化与旧 key 匹配正确；非 loopback 不产生 http 候选；连续端点失败返回 Transport 且不误降级；续期落盘复用原 key | pass |
| boundaries-and-failure-paths | 单元与集成正反例：endpoint_candidates 非 loopback、legacy key、no-scheme 登录+发布、旧 http 配置 publish | 挑战「https 失败后 http 是否快速生效」「下载流中途失败是否重试」「logout 是否清楚旧 key」「非 loopback 是否开放明文」 | mock 对 TLS ClientHello 快速关闭后落到 HTTP；下载流响应头已返回后不重试；logout 按身份清除全部匹配 key；127.0.0.1/localhost/[::1] 之外无 http 候选 | pass |
| regression-and-side-effects | 全量 `cargo test -p filehub-cli`（10 单元 + 14 集成 + 4 dv）与 `cargo check --workspace` | 检查既有 `http://` 测试写法、401 续期、409/403/404 错误分类、命令帮助文案与 003 设计文档 | 全部用例通过；既有 api/dv 用例在新语义下无需重写；`cargo check --workspace` 通过；未引入新依赖或服务端改动 | pass |

## Verification

- Targeted check: `rustfmt --edition 2024 --config skip_children=true --check`（本次
  实际改动的 5 个源/测试文件）、`cargo test -p filehub-cli`（10 单元 + 14
  api_integration + 4 dv 全量）、`cargo check --workspace`
- Result: passed
- Exception reason: not-applicable

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | no_scheme 用例中 mock 每次先收到 TLS ClientHello 再快速 400 关闭，随后走 HTTP 成功 | 对明文 HTTP 的 loopback 服务，每次命令会先产生一次 HTTPS 连接失败往返；这是 Docker「HTTPS 优先、HTTP 降级」的固有行为，成功路径对用户无感知 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 交付完整覆盖已确认提案 P-001/P-002/P-003：身份与传输分离、旧凭据兼容、
  HTTPS 优先与 loopback HTTP 降级均已实现并通过单元/集成/流程测试；独立缺陷发现
  三分类全 pass，唯一发现为预期的 HTTPS 首尝试往返（非缺陷），无阻塞项。
