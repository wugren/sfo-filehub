# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/006-admin-web-layout-polish.md

## Delivery Summary
- Outcome: admin-web 全部页面已按 GitHub Primer 风格完成布局与视觉统一：深色吸顶页头（品牌标记、NavLink 激活态、用户 chip 与登出）、浅色画布 + 白底 Box 卡片、GitHub 风格按钮（默认灰/绿色主/红色危险）、列表式表格（行分割线 + hover + 容器横向滚动）、private/public Label 徽标、居中登录卡片、GitHub 风格空态与错误横幅、输入焦点环，并修复 390px 窄屏横向溢出（移动端隐藏用户 chip 与品牌文本）。
- Handoff: `npm run build`、`npm run test:unit`、`npm run test:integration`、`npm run test:dv` 全部通过；Playwright 9 张截图与 18 项布局断言通过。剩余风险为本机 node_modules 平台残留二进制（挂载盘无法删除）、一条既有的 `act()` 测试提示，以及任务期间 `.gitignore` 被并发追加的 `target` 行（已保留，导致完成清单含构建产物证据噪声），均不影响交付物。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-web-layout-polish | admin-web 全部页面按 GitHub/Primer 风格统一布局与视觉，不改业务逻辑/路由/文案/API 语义，不新增 npm 依赖 | proposal.md P-001 与 Scope | `admin-web/src/styles.css`（Primer 色板、Box、按钮、表格、Label、空态/错误态、焦点态、响应式）；`App.tsx`/各页面仅补结构类名与按钮变体；package.json 与 package-lock.json 未改动；行为断言（路由跳转、会话、token 一次性展示、confirm 交互）由原测试全量覆盖 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `App.tsx`/`LoginPage`/`ProjectsPage`/`ProjectDetailPage`/`MembersPage`/`TokensPage` 与样式表基线 diff | 逐文件对比基线，检查是否有文案/语义/API 行为被误改；NavLink 是否影响受保护路由 | 未发现业务逻辑回归；文案逐字保留（含"← 返回项目列表"）；`NavLink` 仅作用于已登录后页头导航，受保护路由逻辑不变 | pass |
| boundaries-and-failure-paths | 空态/错误态/窄屏/未登录跳转路径 | 空项目/空 token 空态断言；登录失败错误横幅 `role="alert"`；390px 窄屏页面横向溢出与表格容器内滚动；未登录访问受保护页重定向仍由原测试覆盖 | 初始 390px 头部溢出 37px，已通过隐藏用户 chip（修正 CSS 优先级）与品牌文本修复，复查通过；其余路径无缺陷 | pass |
| regression-and-side-effects | 构建产物、单测/集成测试、jsdom 断言、dist 校验 | 检查 class/文案变更是否命中测试断言（按 label/role/names 查询）；构建产物是否含挂载点与非空资源；新增 CSS 类是否影响既有测试选择器 | `test:unit` 30/30、`test:integration` 7/7、`dv verify` 通过；无需修改测试断言 | pass |

## Verification
- Targeted check: `cd admin-web && npm run build && npm run test:unit && npm run test:integration && npm run test:dv`；另用 Playwright（chromium headless + 本地 mock API）对登录/项目/详情/协作者/Token/空态/错误态/窄屏截图并跑 18 项布局断言（命令脚本位于 /tmp/pw/verify.mjs，截图位于 /tmp/pw/shots/）
- Result: passed
- 验证明细：构建成功；单测 30、集成 7、dv 通过；18/18 布局断言通过。
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 本机文件系统 | admin-web `node_modules` 由 Windows 平台安装残留（`@esbuild/win32-x64/esbuild.exe`、`@rollup/rollup-win32-x64-msvc/rollup.win32-x64-msvc.node`）在挂载盘上无法删除，`npm ci` 触发 EIO；本任务在仓库根补装 Linux 依赖树并链接 `.bin` 完成验证 | no |
| F-2 | low | 单测输出 | `LoginPage.test.tsx` 存在既有的 `act(...)` 警告（异步登录状态更新），与本次布局改动无关，断言全部通过 | no |
| F-3 | low | `.gitignore` mtime 17:22:57 与基线快照差异 | 任务执行期间 `.gitignore` 被外部并发追加 `target` 一行（基线快照内容不含该行，非本次任务修改）；该改动有益且已保留，导致完成清单包含 `target`/`server/target` 构建产物路径（纯证据噪声），不影响本次 admin-web 交付范围 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 交付贴合已批准提案 P-001 的 GitHub Primer 风格范围；文案、路由、会话与 token 展示等业务语义保持不变；构建、单测、集成测试、dist 校验与浏览器布局断言全部通过，空态/错误态/窄屏边界已做独立缺陷搜索，仅剩余与本机依赖文件系统相关的非阻塞环境项。
