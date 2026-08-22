---
task_manifest: task.yaml
status: draft
---

# 服务端 CORS 预检放行修复

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment
- Proposed tier: standard
- Final tier: pending
- Tier rationale / triggered boundaries:
  - 属配置型缺陷修复，无公共契约、数据迁移、依赖或安全边界变更，不构成 high-risk。
  - 但该配置直接影响服务端 HTTP 运行时行为（CORS 预检响应），属于有界单项目 bugfix，按任务分类默认走 standard：
    修复后需保留一份 `docs/changes/<change>.md` 变更记录，并在完成后做独立缺陷发现与 completion 校验。
- Proposal and tier confirmation:
  - 用户确认本提案后，将 `workflow_tier` 从 `pending` 写为 `standard`，并把本提案置为 `status: approved`。
  - 若用户改为 trivial/high-risk，则按确认结果执行对应流程。

## Background and Goal
- 现象：浏览器访问 `http://localhost:5173`（admin-web 开发服务器）调用 `http://127.0.0.1:8080/account/login` 时，预检请求被服务端拒绝：
  `Response to preflight request doesn't pass access control check: No 'Access-Control-Allow-Origin' header is present`。
- 根因（已在运行中的服务上复现）：
  - `filehub-server.json` 与 `server/config.example.json` 都设置了 `allow_origins: ["*"]`、`allow_methods: ["GET","POST","PUT","DELETE","OPTIONS"]`，但 `allow_headers` 为空数组。
  - sfo-http 0.7.0 的 Actix 后端只有在 `allow_headers` 非空时才配置 `actix-cors` 的 `allowed_headers`；`Cors::default()` 的可放行请求头集合为空。
  - 浏览器登录预检携带 `Access-Control-Request-Headers: content-type`，actix-cors 因请求头不在白名单返回 400，且不附加 `Access-Control-Allow-Origin`，于是表现为“无 ACAO 头”。
- 目标：让服务端对带 `content-type`（及 `authorization`）的跨域预检返回标准 CORS 头，登录等 admin-web 请求恢复可用。

## Scope
### In scope
- 将仓库内的服务端运行配置 `filehub-server.json` 与配置示例 `server/config.example.json` 的 `allow_headers` 从 `[]` 改为 `["*"]`。
- 完成改动后对运行中的服务做预检/登录的定向验证，并保留变更记录与完成报告。
### Out of scope
- 不修改 `server/src/` 等 Rust 源码、不更换/补丁 `sfo-http` crate。
- 不修改 admin-web 前端代码、`VITE_API_BASE_URL` 或登录流程。
- 不改动 `allow_origins`、`allow_methods`、`support_credentials`、`max_age` 等其他 CORS 语义。
### Boundary with neighboring modules
- 服务端 CORS 放行属于 001-filehub-core-platform 的 HTTP 装配/配置职责；002-filehub-web 仅作为消费方通过浏览器完成验证。

## Requirement Review
- 需求合理：配置值 `allow_origins: ["*"]` 已表明“放行任意来源”的既有意图，`allow_headers` 缺失只是让预检在请求头校验处提前失败的配置缺陷；补上请求头放行符合原意图。
- 权衡：使用 `["*"]` 表示接受浏览器可能携带的任意请求头。由于 `allow_origins` 本身已是 `["*"]`、`support_credentials` 为 `false`，且会话通过 `Authorization` 头传递，这一选择与既有安全姿态一致，未扩大凭据暴露面。
- 备选：只放行 `["Content-Type", "Authorization"]` 更窄。考虑到服务端契约含 JSON body 与 Bearer 头，且 `sfo-http` 已支持 `"*"` 映射为 `allow_any_header()`，采用 `["*"]` 最小且可维护。

## Proposal Items
| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-cors-allow-headers | 两个服务端配置的 `allow_headers` 改为 `["*"]`，使带 content-type 的跨域预检通过 | 仅改两份 JSON 配置，不改源码与前端 | 放行任意请求头；沿用已有 allow_origins=* 的安全姿态，不启用 credentials | 运行中的服务对 `/account/login` 预检返回 200 且含 `Access-Control-Allow-Origin: *`；登录 POST 返回正常包装响应 | 不修改 Rust 源码、不补丁 sfo-http |

## Success Criteria
- Concrete user-visible or system-visible result:
  - `curl -X OPTIONS http://127.0.0.1:8080/account/login`（携带 `Origin: http://localhost:5173`、`Access-Control-Request-Method: POST`、`Access-Control-Request-Headers: content-type`）返回 HTTP 200，且带 `Access-Control-Allow-Origin` 与 `Access-Control-Allow-Headers`。
  - 浏览器登录页不再报 CORS 预检阻断。
- Required evidence:
  - 修复后的 OPTIONS 响应头明文（curl 输出）。
  - 登录 POST 在带 Origin 时仍返回 200 与 `{err:0,result:{session,...}}`。
- Explicit non-goals:
  - 不引入新的服务端 CORS 中间件代码，不改变其他 CORS 配置项。

## Risks
- 放行任意请求头属于轻微的安全面加宽，但与既有的 `allow_origins: ["*"]` 姿态一致且未启用凭据模式；部署方如需更窄约束，可按各自环境覆盖配置。
- 修改后需要重启当前 8080 服务才生效；本次任务负责完成仓库配置修复并提供验证命令，服务重启动作若由用户环境持有，会明确告知。
