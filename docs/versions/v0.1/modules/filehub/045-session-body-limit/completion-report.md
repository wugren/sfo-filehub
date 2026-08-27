# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/045-session-body-limit.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `third_party/sfo-account/src/account_server.rs`：`read_login_body()` 泛化为
    共享 `read_bounded_json_body(req, route)`（Content-Length 预检 +
    `take_http_body()` 流式累积，累计超过 65536 字节立即拒绝），登录侧错误文案
    保持「login request body ...」不变；`/account/get_account_info_of_session`
    handler 由无上限 `body_json()` 改为经 `read_session_info_body` 的 64 KiB
    有界读取 + `serde_json` 解析，超限错误沿用 `AccountErrorCode::InvalidParam`；
  - `docker/nginx.conf`：新增 `location = /account/get_account_info_of_session`
    精确匹配块，`client_max_body_size 64k`，与 `^~ /account/` 保持相同反代头；
  - `server/tests/api_integration.rs`：新增
    `get_account_info_of_session_rejects_request_body_over_64k`，覆盖 64 KiB 边界
    合法 session 解签、固定长度 65 KiB Content-Length 预检、无 Content-Length 的
    流式 chunked 实际流量超限三条路径。
- Handoff: `cargo check -p filehub-server` 通过；
  `--test api_integration request_body_over_64k` 2/2 通过（含 038 登录回归）；
  `--test api_integration session` 4/4 通过（含登录会话流与 refresh 回归）；
  nginx 模板未在本机执行 `nginx -t`（环境未安装 nginx），交付说明已标注需 Docker
  冒烟验证。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-session-body-limit | session 信息请求体最大 64 KiB：Content-Length > 65536 或实际累计读取 > 65536 均拒绝（InvalidParam），恰好 64 KiB 合法 session JSON 仍进入解签 | proposal.md P-001 | `account_server.rs` 共享有界读取 + `read_session_info_body`；集成用例边界/固定长度/chunked 三条路径覆盖并为成功/拒绝断言 | 匹配 | pass |
| fh-session-body-limit-nginx | docker nginx 对 `location = /account/get_account_info_of_session` 设 `client_max_body_size 64k`，保留相同反代头 | proposal.md P-002 | `docker/nginx.conf` 精确匹配块含 64k 与全部 proxy_set_header；`/api/v1/` 与全局 0 未改动 | 匹配 | pass |
| fh-session-body-limit-tests | HTTP 级回归：固定长度 65 KiB 与流式 chunked 超限 session 信息请求均返回非 0 err，64 KiB 边界合法 session 不受限 | proposal.md P-003 | `server/tests/api_integration.rs` 新增用例；定向测试 2/2 与 4/4 通过 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `read_bounded_json_body()` 的预检分支、流式累积顺序（先判超限再 extend）、`is_data()` 后 `into_data()`、`saturating_add`、错误码/文案映射；session handler 的 serde 解析路径 | 反向推演：Content-Length 为小值但实际多读（wire 语义 + 流式上限兜底）；单 chunk 超过 64 KiB（先判断再累积，不越界）；chunked 无 Content-Length（实际流量上限兜底）；trailer 帧拒绝；恰好 65536 允许 | 无绕过：超限三条路径（声明确认、无声明 chunked、声明小于实际）均被拒绝，不销毁有限内存预算 | pass |
| boundaries-and-failure-paths | 65536/65537 边界、Content-Length 头缺失/解析失败、成功响应与错误响应字段差异、nginx 侧 64k | 检查 off-by-one：边界用例精确构造 65536 字节合法 session JSON 并断言 err==0 解签成功；65537 起始的超限两形态均拒绝；Content-Length 解析失败后仍受实际流量上限；坏 JSON/缺字段回退既有 InvalidParam 解析错误路径 | 边界口径统一为“≤ 65536 字节允许”；无 off-by-one；成功响应仅解析 err 字段，错误响应含 msg，字段契约未变 | pass |
| regression-and-side-effects | 登录 handler 与 038 用例断言、`api_login_session_and_token_flow`、refresh 会话用例、nginx `^~ /account/` 与精确匹配优先级、Cargo 依赖与锁文件 | 排查是否误伤其它 body_json 路由（仅 session 信息路由使用 body，`get_account_info`/`refresh_session` 只读 Authorization 头未触）；`take_http_body` 改动是否影响上传流式 ingest（未改）；nginx 精确匹配优先级高于前缀；无新增直接依赖、无 Cargo.lock 变更 | 038 登录超限用例原断言通过（登录文案不变）；`session` 过滤 4/4 通过；上传路径与其它路由零改动；nginx 无本机 `nginx -t` 验证（环境未安装，记录于 F-1） | pass |

## Verification

- Targeted check: `cargo check -p filehub-server` 通过；
  `cargo test -p filehub-server --test api_integration request_body_over_64k` 2/2
  通过；`cargo test -p filehub-server --test api_integration session` 4/4 通过
- Result: pass
- Exception reason: 无（未跑全量 api_integration/unit：工作树存在 025-044 等在制
  未收尾内容，全量朝绿由在制任务负责；本次 session-info/登录/会话定向验证完整）

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本环境无 nginx，`nginx -t` 未执行 | nginx 模板为标准指令，仍需 Docker 构建/部署冒烟验证精确匹配块与反代头（同 038 残余项） | no |
| F-2 | low | 共享 `read_bounded_json_body` 以 route 字符串拼入错误/ trailer 文案 | 文案随 route 变化是预期设计；登录侧实际输出保持「login request body ...」不变，客户不应依赖文案 | no |
| F-3 | low | session 信息坏 JSON 的错误文案来自 `serde_json` 解析路径 | 错误码与响应结构不变，契约未约束文案；与 038 的坏 JSON 文案变化同类 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001/P-002/P-003 全部落地：Content-Length 预检与实际流量有界读取
  （64 KiB）覆盖 session 信息接口，nginx 精确匹配路由加 64k 上游限制，固定长度/
  chunked/边界三条 HTTP 回归路径全绿，登录与会话流不回归；独立缺陷发现覆盖逻辑、
  边界/失败路径与回归副作用，F-1~F-3 均为非阻塞低危记录。
