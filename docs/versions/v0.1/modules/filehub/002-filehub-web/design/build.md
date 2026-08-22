---
task_manifest: task.yaml
status: approved
---

# build 子模块设计（工程与静态交付）

## 职责

`admin-web/` 工程配置与构建：Vite + React + TypeScript，产出独立静态站点 `admin-web/dist`；API base URL 通过 `VITE_API_BASE_URL` 注入，开发环境可在 vite proxy 指向服务后台。

## 结构决策

- `package.json`：锁定运行时与开发依赖版本（`package-lock.json` 一并交付），scripts：`build`（`tsc -b && vite build`）、`dev`、`preview`。
- `vite.config.ts`：React 插件、`build.outDir` 默认 `dist`、基路径 `./`（支持任意静态托管路径）。
- `tsconfig.json`/`tsconfig.node.json`：严格模式；`src/**` 为运行时源。
- 环境变量：`VITE_API_BASE_URL`（默认 `http://127.0.0.1:8080`），`ApiClient` 启动时规范化。

## 部署契约

- 交付物：`admin-web/dist/` 静态文件（index.html + js/css 资源），不依赖服务后台静态托管；
- 部署方将 dist 挂到静态站点并把 `VITE_API_BASE_URL` 指向 001 服务后台；CORS 由服务端配置放行；
- 无服务端路由改写需求（SPA 使用 hash 或浏览器 history 由部署容量决定，首版用标准 history 并在文档注明）。

## 不变项

- 构建可复现：依赖锁定、无网络动态依赖；
- 产物内不含凭据/密钥；
- 生产构建产物可被验收阶段直接整链路验证。

- Consumer: 部署环境与浏览器（change_id: fh-web-login、fh-web-project-versions、fh-web-token-manage、fh-web-members 公共基础）
- Compatibility: new
