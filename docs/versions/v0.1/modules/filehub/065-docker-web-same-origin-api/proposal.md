---
task_manifest: task.yaml
status: approved
---

# 修复 Docker 管理页面错误连接本机 API

Risk profile: not-created（确认 high-risk 后创建 `./risk-profile.yaml`）

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: trivial
- Tier rationale / triggered boundaries:
  - 根因已由当前代码路径确认：GitHub Actions 的 `test-web` job 直接执行 `npm run test:dv`，未注入 Docker 一体镜像要求的 `VITE_API_BASE_URL=/`；Vite 因而读取已跟踪的 `admin-web/.env.local`，把 `http://127.0.0.1:8080` 固化进上传的 `web-dist`，随后由 `build-image` 原样打入 GHCR 镜像；
  - 修改范围仅涉及同一 `filehub` 模块的前端 CI 构建参数和 dist 防回归检查，但结果直接改变正式 Docker produced artifact 及 release/deployment surface；根据 build/config/deployment trigger，这一物质影响触发 `high-risk`；
  - 不改变 API、nginx 反代、服务端监听、开发环境默认地址、持久化数据、安全边界或依赖图。
- Proposal and tier confirmation: 2026-08-29 用户回复“确认，trivial”，批准本提案并明确选择 `trivial`；已向用户说明该选择低于 produced artifact / release-deployment surface 默认建议层级。

## Approval Record

- approver: 用户
- approval_date: 2026-08-29
- user_statement: “确认，trivial”
- selected_tier: trivial
- explicit_tier_override: 用户明确将建议的 `high-risk` 降为 `trivial`；按用户选择执行轻量流程，并在完成报告中保留正式镜像需重新构建发布的残余风险。

## Background and Goal

Docker 一体镜像应由 nginx 同时提供管理页面，并将浏览器访问的 `/account/`、`/api/v1/` 请求反向代理到容器内 `127.0.0.1:8080`。当前发布流水线生成的前端产物却包含绝对地址 `http://127.0.0.1:8080`；远程用户浏览器会把该地址解释为用户自己的电脑，导致登录请求绕过容器 nginx 并失败。

目标是让 CI 发布到 GHCR 的管理页面固定使用同源 API base `/`，并在产物验证阶段机械拒绝再次包含 loopback API base，使登录请求实际到达页面所属 Docker 宿主地址及其 nginx 反代。

## Scope

### In scope

- 在 GitHub Actions 的 admin-web 发布产物构建步骤显式注入 `VITE_API_BASE_URL=/`；
- 保证 `web-dist` artifact 和后续 GHCR Docker 镜像复用同一份已验证的同源前端产物；
- 扩展 `admin-web/tests/dv/verify-dist.mjs`，验证构建后的 JavaScript 不包含 `http://127.0.0.1:8080`，并包含可证明登录/API URL 使用同源 base 的产物证据；
- 执行前端构建、dist 验证、工作流静态/契约检查，以及与 Docker 打包链路有关的目标验证；
- 在 acceptance 阶段独立检查 CI artifact 传递链、环境变量优先级、旧 loopback 字符串残留和本地构建路径一致性。

### Out of scope

- 不修改 `admin-web/.env.local` 和 `session.ts` 的开发模式默认 API 地址；
- 不修改 Docker nginx 的 `/account/`、`/api/v1/` 反代目标，也不修改容器内 server 固定监听的 `127.0.0.1:8080`；
- 不改变 HTTP API、认证流程、CORS、安全策略、配置 YAML、端口映射、数据库或持久化文件；
- 不重写或删除已经发布的 GHCR tag；修复进入后续重新构建/发布的镜像，是否覆盖既有 tag 由现有发布策略决定；
- 不触碰工作树中既有的 `Cargo.lock` 与 `harness/scripts/edit-guard.py` 改动。

### Boundary with neighboring modules

本任务只修正 filehub-web 的正式构建产物和 Docker 发布流水线。服务端仍由容器内 nginx 通过 `127.0.0.1:8080` 访问；浏览器只看到相对于当前页面 origin 的 `/account/*` 与 `/api/v1/*`。CLI、服务端业务实现和独立前端开发模式不变。

## Requirement Review

需求合理，且根因不是 nginx 反代规则，而是本地 `build-docker.sh` 与 GitHub Actions 使用了不同的前端构建参数。只修改 nginx 或 Docker 运行参数无法修复已经固化在 JavaScript 中的绝对 URL。

建议以“CI 显式注入 + dist 产物断言”同时闭环：前者修复行为，后者阻止以后因 workflow 重构或环境文件优先级变化而回归。保持 `.env.local` 与源码默认值不变，可以继续支持现有本地独立开发方式；代价是正式发布路径必须始终显式声明同源配置，因此需要把这一点变成机器检查。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-docker-web-same-origin-api | CI 构建供 Docker 使用的 `web-dist` 时显式设置 `VITE_API_BASE_URL=/`，使 `/account/login` 等请求与页面同源 | 仅改变正式 CI/Docker 前端产物；开发环境默认值、服务端和 nginx 内部 upstream 不变 | 发布构建必须显式维护该环境变量 | CI workflow 静态证据；以同源变量完成真实前端构建；dist 中无 `http://127.0.0.1:8080` 且登录路径仍存在 | 不改变独立部署前端的通用配置能力或 Docker 内部端口契约 |
| P-002 | fh-docker-web-same-origin-api | dist 验证必须拒绝带 loopback API base 的发布候选产物，并验证同源 API base | 检查只面向生成的 JS 产物，不绑定哈希文件名或压缩器内部变量名 | 产物文本检查依赖稳定的用户可见 URL/路由字面量，避免绑定实现细节 | 负向 fixture/构建或等价验证证明 loopback 会失败；正常同源构建通过 | 不扩展为通用安全扫描器，不检查 CLI 或服务端中的合法 loopback 文本 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - 使用修复后的 CI 产物构建 Docker 镜像后，从任意宿主地址打开管理页面，登录请求目标为当前页面 origin 下的 `/account/login`，不再访问浏览器所在设备的 `127.0.0.1:8080`；
  - nginx 继续把该同源请求转发到容器内 filehub-server。
- Required evidence:
  - admin-web 单元/集成测试及 `VITE_API_BASE_URL=/ npm run test:dv` 通过；
  - dist 验证能识别并拒绝包含 `http://127.0.0.1:8080` 的错误发布产物，同时不依赖构建 hash 文件名；
  - workflow/YAML 与 Docker 契约目标检查通过，确认上传和打包的是同一 `web-dist`；
  - 独立验收审查覆盖环境变量优先级、artifact 传递、浏览器 origin 语义、本地 `build-docker.sh` 一致性及发布/回滚边界。
- Explicit non-goals:
  - 不声称当前已发布的远端 GHCR 镜像自动改变；必须由后续 CI 重新构建并按现有授权发布；
  - 不改变开发服务器直接访问 `127.0.0.1:8080` 的行为。

## Risks

- 正式镜像行为依赖 Vite 编译时注入；若只验证 workflow 文本而不验证 dist，仍可能因环境加载或命令调整回归，因此必须检查真实产物。
- 使用 `/` 后，浏览器将请求发给页面 origin；这正是 Docker 一体镜像的目标，但独立静态站点若复用该 artifact，也必须具备相同 origin 的 API 反代。本任务明确把该 artifact 定义为 Docker 发布产物，不改变独立部署构建的可配置能力。
- 修复不会自动覆盖既有 GHCR tag。若需要替换已发布版本，需要遵循当前仓库的 tag/发布授权和回滚策略，不能在本任务中静默执行远端发布。
