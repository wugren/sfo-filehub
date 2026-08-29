# Docker 管理页面同源 API 修复完成报告

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: trivial
- Change record: not-applicable

## Delivery Summary

- Outcome: GitHub Actions 生成 Docker `web-dist` 时显式注入 `VITE_API_BASE_URL=/`；dist 验证在 CI/同源构建模式下同时校验构建环境、登录路由和 loopback 地址残留，正式页面请求将使用当前页面 origin 下的 `/account/login`。
- Handoff: 代码与本地产物验证已完成；当前已经发布的 GHCR tag 不会自动变化，需由后续受现有发布门禁授权的 CI 重新构建并发布镜像后，线上 Docker 用户才能获得修复。

## Proposal Consistency

| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-docker-web-same-origin-api | CI 的 Docker 前端产物使用同源 API，并机械拒绝 loopback 回归；开发默认值、nginx、服务端及既有远端 tag 不变 | proposal.md 的 P-001、P-002、Scope 与 Success Criteria | `.github/workflows/build.yml` 为 `Build and verify dist` 设置 `/`；`verify-dist.mjs` 检查 CI 环境和真实 JS；Actions 契约新增构建及 artifact 链断言；默认与同源构建、两条负向探针均已执行 | 实现覆盖已批准需求与边界，未修改开发环境配置、nginx、服务端或远端发布状态 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `.github/workflows/build.yml` 的 `test-web` 到 `web-dist` 上传及 `build-image` 下载链；`session.ts` 的 base 规范化；当前同源 dist | 以默认 `.env.local` 构建错误产物后分别模拟 `GITHUB_ACTIONS=true` 和伪设 `VITE_API_BASE_URL=/`，挑战“只检查 workflow 文本”与“只检查环境变量”的薄弱路径 | 第一条因 CI 未注入 `/` 非零失败，第二条因产物仍含 `http://127.0.0.1:8080` 非零失败；真实同源构建无 loopback 且保留 `/account/login`，未发现逻辑缺陷 | pass |
| boundaries-and-failure-paths | `verify-dist.mjs` 的环境分支、JS 资产读取、登录路由与 loopback 错误路径；本地 `.env.local` 和 GitHub Actions 固定环境 | 分别运行不带环境变量的普通 DV 与 `VITE_API_BASE_URL=/` 的 Docker DV，检查开发路径兼容和正式构建失败边界；检查缺少同源变量与错误产物的错误信息和退出码 | 普通开发 DV 保持通过；Docker DV 通过；两种错误路径均返回 1 且指出具体原因，未把开发 loopback 默认行为误改为正式发布行为 | pass |
| regression-and-side-effects | GitHub Actions unit/DV/integration 契约、Docker nginx/config DV、admin-web 单元/集成测试、任务差异与起始工作树基线 | 反查 Cargo lock 分发、发布授权、version/latest 同镜像、artifact 名称、checkout SHA、全部 workflow `run` Bash 语法、nginx 内部 8080 契约以及既有脏文件隔离 | Actions 6/7/7、Docker DV 3、Web 单元 57 与集成 9 全部通过；任务仅新增同源构建和验证，不触碰 `Cargo.lock`、`edit-guard.py`、本地 YAML/DB 或运行时反代契约 | pass |

## Verification

- Targeted check: `python3 tests/github_actions_build_contract.py --suite unit|dv|integration`；`python3 tests/docker_config_contract.py --suite dv`；`npm run test:unit`；`npm run test:integration`；默认及 `VITE_API_BASE_URL=/ npm run test:dv`；错误 dist 的 `GITHUB_ACTIONS=true node tests/dv/verify-dist.mjs` 与 `VITE_API_BASE_URL=/ node tests/dv/verify-dist.mjs` 负向探针；任务范围 `git diff --check`
- Result: passed
- Exception reason: not-applicable；本机未安装 `actionlint` 且无 Docker CLI，已由 PyYAML workflow 解析、全部 `run` 脚本 `bash -n`、artifact 契约及真实前端产物构建覆盖本次局部改动；未声称完成本地镜像或远端发布验证。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | proposal.md Out of scope、当前仅有仓库工作树差异且未执行远端发布 | 既有 GHCR 镜像仍包含旧前端产物，必须由后续授权 CI 重新构建/发布才能让已部署用户取得修复；这属于已批准的发布边界，不是当前代码缺陷 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: CI 注入、真实 dist 防回归、静态 artifact 链契约、正反构建验证和独立缺陷搜索均通过；交付满足批准的同源登录目标，且没有扩大到开发默认值、服务端、nginx 或远端发布操作。
