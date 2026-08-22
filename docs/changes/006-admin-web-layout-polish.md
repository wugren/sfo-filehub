# admin-web GitHub 风格布局优化

- Status: complete
- Owner module: filehub（filehub-web / admin-web）
- Task manifest: docs/versions/v0.1/modules/filehub/006-admin-web-layout-polish/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/006-admin-web-layout-polish/proposal.md
- Affected paths: admin-web/src/styles.css, admin-web/src/App.tsx, admin-web/src/pages/*.tsx, admin-web/src/components/*.tsx
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

按 GitHub Primer 设计语言子集用纯 CSS + 少量 JSX 结构调整实现布局美化：

- `styles.css` 全面重构：Primer 色板（画布/浅画布/边框/正文/次要文本/强调/危险/成功）收进 CSS 变量；深色吸顶页头；白底 Box 卡片；GitHub 风格按钮（默认灰、绿色主按钮、红色危险按钮与 focus ring）；列表式表格（行分割线 + hover）与横向滚动容器；Label 徽标（public/private/角色）；居中空态；GitHub 风格错误横幅与登录卡片；字体栈对齐 GitHub。
- JSX 最小调整：`App.tsx` 导航改用 `NavLink` 提供激活态并增加内联 SVG 品牌标记；各页面给危险/主操作按钮与表格容器/空态补充稳定的类名与结构；不改变任何文案、路由、接口调用、confirm 交互、一次性 JWT 展示与错误处理语义。
- 不新增 npm 依赖，不改变依赖图与构建产物形态。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no（仅视觉展示层调整；保留全部语义标签、文案与焦点可见性，并经浏览器抽查与单测覆盖）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `npm run build`（tsc --noEmit + vite build）、`npm run test:unit`、`npm run test:integration`；浏览器抽查登录/项目/详情/协作者/Token/空态/错误态/窄屏。
- Result: passed
- 验证明细：`npm run build` 成功（tsc 无错误 + vite 构建 dist）；`test:unit` 30/30、`test:integration` 7/7、`dv verify` dist 校验通过；Playwright 对 9 个页面/状态截图，18 项布局断言全部通过（深色吸顶页头、白色 Box、绿色主按钮/红色危险按钮、行分割线表格与容器内滚动、Label 徽标、输入焦点环、登录卡片居中、错误横幅、390px 窄屏无页面横向溢出）。
- Residual risk or follow-up: 本沙箱首次安装的 admin-web `node_modules` 含 Windows 平台残留二进制（挂载文件系统上无法删除，`npm ci` 会报 EIO）；已在仓库根放置与本机平台匹配的 `node_modules`（git 忽略）并给 `admin-web/node_modules/.bin` 建符号链接，本地 `npm run build`/测试可正常执行。后续在其他平台/真实文件系统重新 `npm ci` 后可清理残留。单元测试存在一条既有的 `act(...)` 提示（登录异步测试，布局改动前即存在，不影响断言）。任务执行期间 `.gitignore` 被外部并发追加 `target` 一行（非本次任务修改，已保留；该行有利于后续基线不再包含构建产物）。
