# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/038-login-body-limit.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - `third_party/sfo-account/src/account_server.rs` 登录 handler 由无上限
    `body_json()` 改为 `read_login_body()`：Content-Length 预检（声明 >
    65536 字节立即拒绝）+ `take_http_body()` 流式累积（实际读入超过 64 KiB
    立即拒绝，不再继续读取），超限错误沿用 `AccountErrorCode::InvalidParam`；
  - `third_party/sfo-account/Cargo.toml` 补充 `http-body-util`、`serde_json`
    直接依赖（已在 Cargo.lock 解析图中）；
  - `docker/nginx.conf` 新增 `location = /account/login` 精确匹配块，
    `client_max_body_size 64k`，与 `^~ /account/` 保持相同反代头；
  - `server/tests/api_integration.rs` 新增 `login_rejects_request_body_over_64k`：
    64 KiB 边界合法 JSON 仍进入账号校验、固定长度 65 KiB 走 Content-Length
    预检、无 Content-Length 的流式 chunked 走实际流量上限，均验证。
- Handoff: `cargo check -p filehub-server` 通过；
  `cargo test -p filehub-server --test api_integration login` 2/2 通过
  （新增用例 + 既有 `api_login_session_and_token_flow`）；nginx 模板未在本机
  执行 `nginx -t`（环境未安装 nginx），交付说明中已标注需 Docker 冒烟验证。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-login-body-limit | 登录请求体最大 64 KiB：Content-Length > 65536 或实际累计读取 > 65536 均拒绝（InvalidParam），恰好 64 KiB 仍进入账号校验 | proposal.md P-001 | `account_server.rs` 的 `read_login_body()`（预检 + 有界累积）；集成用例固定长度/边界/chunked 三条路径均覆盖并断言消息 | 匹配 | pass |
| fh-login-body-limit-nginx | docker nginx 对 `location = /account/login` 设 `client_max_body_size 64k`，保留相同反代头 | proposal.md P-002 | `docker/nginx.conf` 精确匹配块含 64k 与全部 proxy_set_header | 匹配 | pass |
| fh-login-body-limit-tests | HTTP 级回归：固定长度 65 KiB 与流式 chunked 超限登录均返回非 0 err | proposal.md P-003 | `server/tests/api_integration.rs` 新增用例；`--test api_integration login` 2/2 通过 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | `read_login_body()` 的预检分支、流式累积顺序（先判超限再 extend）、`is_data()` 后 `into_data()`、错误码映射；sfo-http `take_http_body` 的 actix 实现 | 反向推演：Content-Length 为小值但实际多读（wire 层按 Content-Length 截断，且实际流量上限兜底）；单 chunk 超过 64 KiB（先判断再累积，不越界）；`saturating_add` 覆盖 usize 溢出；trailer/非 data 帧返回 InvalidParam；恰好 65536 允许 | 无绕过：三条大于 65536 的路径（声明确认、无声明 chunked、声明小于实际——由 wire 语义与流式上限共同约束）均被拒绝，不销毁有限内存预算 | pass |
| boundaries-and-failure-paths | 65536/65537 边界、Content-Length 头缺失/解析失败、空 body、JSON 解析失败、同时在 nginx 侧的 64k | 检查 off-by-one：边界合法 JSON 用例精确构造 65536 字节并断言不被上限误拒；65537 起始的超限两形态均拒绝；Content-Length 解析失败忽略后仍受实际流量上限；空/坏 JSON 仍回退到既有 InvalidParam 解析错误路径 | 边界口径统一为“≤ 65536 字节允许”；无 off-by-one；坏 JSON 错误消息与改动前有措辞差异（由 serde_json 直接输出），不影响契约 | pass |
| regression-and-side-effects | 登录 handler 之外的 account 路由（`get_account_info_of_session`/`refresh_session`）、上传/下载路径、nginx 全局 0、Cargo.lock 中 http-body-util/serde_json 与 sfo-account | 排查是否误伤其它 body_json 路由（未改）；`take_http_body` 改动是否影响上传流式 ingest（未改）；nginx 精确匹配优先级高于 `^~ /account/`，上传位置不受 64k 影响；新增直接依赖未引入新包 | 既有 `api_login_session_and_token_flow` 通过；其余路由与上传路径零改动；nginx 无本机 `nginx -t` 验证（环境未安装，记录于 F-2） | pass |

## Verification

- Targeted check: `cargo check -p filehub-server` 通过；
  `cargo test -p filehub-server --test api_integration login` 2/2 通过
- Result: pass
- Exception reason: 无（未跑全量 api_integration：工作树存在 036 等在制未收尾
  用例，全量朝绿状态由在制任务负责；本次登录相关定向验证完整）

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `frame.into_data()` 的 Err 分支类型是退回原帧，不是 std error；已先 `is_data()` 校验后用 `unwrap_or_default()` | 若未来 frame 语义变化可能静默拿到空数据；当前 `is_data()` 后 `into_data()` 必然 Ok，无实际影响 | no |
| F-2 | low | 本环境无 nginx，`nginx -t` 未执行 | nginx 模板为标准指令，仍需 Docker 构建/部署冒烟验证精确匹配块与反代头 | no |
| F-3 | low | 超限登录与坏 JSON 解析错误的 `msg` 措辞由 sfo-http 解析文案变为读取/`serde_json` 文案 | 错误码与响应结构不变，客户端不应依赖文案；契约未约束文本 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001/P-002/P-003 全部落地：Content-Length 预检与实际流量有界读取
  （64 KiB）生效，nginx 登录精确匹配路由加 64k 上游限制，三条 HTTP 回归路径
  全绿，既有登录流不回归；独立缺陷发现覆盖逻辑、边界/失败路径与回归副作用，
  F-1~F-3 均为非阻塞低危记录。
