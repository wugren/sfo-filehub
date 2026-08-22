---
task_manifest: task.yaml
status: approved
---

# admin-web 管理后台布局美化

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment
- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 属纯前端视觉/布局重构，不涉及公共 API 契约、持久化数据、依赖图、构建产物或发布/部署面变更，不构成 high-risk。
  - 但改动覆盖 admin-web 全部页面结构、样式与表单/表格展示，命中 `ui-workflow` 触发器；为保证对比度、焦点态、响应式与空态/错误态等展示层不回归，按标准流程执行，保留一份变更记录并在完成后独立做缺陷发现。
- Proposal and tier confirmation:
  - 用户于 2026-08-20 回复“确认”，采纳 standard 层级；`workflow_tier` 已写为 `standard`，本提案置为 `status: approved`。

## Background and Goal
- 现状：`admin-web/` 功能已完整，但布局停留在朴素的功能页形态：顶部简单黑色条、通栏表格、无层次感的卡片与按钮，登录页与数据页视觉一致性弱，窄屏表格溢出，页面标题/说明/空态缺少统一的排版节奏。
- 目标：在不改变任何页面文案、路由、接口调用、会话与 token 展示语义的前提下，按 GitHub 官方 UI（Primer 设计语言）的风格对管理后台做整体布局与视觉优化：深色页头、中性浅色画布、白底 Box 卡片、列表式表格、GitHub 风格按钮/Label 徽标与焦点态，形成一致的排版系统、清晰的信息层级和可用的响应式行为。
- 修订记录（2026-08-20）：用户确认视觉方向按 GitHub 风格优化；原"克制中性浅色 + 蓝色强调"方向更新为 GitHub Primer 风格（保留顶部导航结构，不引入侧边栏）。

## Scope
### In scope
- 应用外壳与导航布局：
  - 页头改为 GitHub 风格深色吸顶栏（类似登录后 GitHub 页头）：品牌标记 + 主导航（项目、Token）带激活态，右侧用户信息 + 登出按钮统一样式。
  - 内容主区按 GitHub 容器风格设统一最大宽度（1280px）、留白与区块间距。
- 页面级排版：
  - 登录页参考 GitHub 登录卡片：浅色画布 + 白底边框 Box、品牌标题、紧凑表单。
  - 项目/项目详情/协作者/Token 各页统一“页头（标题 + 说明）→ 操作卡片 → 数据表格”的视觉节奏。
- 组件级样式：
  - 采用 GitHub Primer 设计语言子集，纯 CSS 实现，不引入组件库：
    - Box 卡片：白底、`1px solid #d0d7de` 边框、6px 圆角与轻微阴影；部分卡片带浅灰头栏。
    - 按钮：GitHub 风格默认灰按钮（`#f6f8fa` 底、`#1f2328` 文本、`#d0d7de` 边框、hover 加深）、主按钮（`#1f883d` 绿色）与危险按钮（红色边框/文本），统一 14px/中等字重。
    - 表格：GitHub 列表式表格（无内边框，行下分割线 + hover 浅灰背景，圆角容器），外层提供横向滚动容器，窄屏不破版。
    - Label 徽标：private/public/角色等用 GitHub 风格小圆角标签（细边框 + 淡底色）。
    - 表单：输入框 focus 时蓝色边框 + `3px rgba(9,105,218,0.3)` 外发光，复选框矩阵保持网格布局。
    - 空态：GitHub 风格居中空态（浅灰占位 + 提示文案，替代散落纯文本）。
    - 错误横幅：GitHub 风格红底浅色错误区，保留 `role="alert"` 语义。
  - 字体栈对齐 GitHub：`-apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", Helvetica, Arial, sans-serif`。
- 视觉细节：
  - 统一 Primer 色板：画布 `#ffffff`、浅画布 `#f6f8fa`、边框 `#d0d7de`、正文 `#1f2328`、次要文本 `#59636e`、强调 `#0969da`、危险 `#cf222e`、成功 `#1a7f37`，以 CSS 变量集中管理。
- 保持项目文档与可访问语义：
  - 不改动可访问名称/角色/文案与 API 行为；HTML 语义（header/nav/main/table/form/label/button）保持或增强。
- 验证：
  - `npm run build`（tsc + vite）、`npm run test:unit`、`npm run test:integration` 通过；必要时补充/微调单元测试断言。逐页在浏览器做视觉与交互抽查（登录、项目、详情、协作者、Token 创建与轮换展示区、空态/错误态、窄屏）。
### Out of scope
- 不改任何 React 业务逻辑、API 客户端、路由、会话或 token 语义（一次性 JWT 展示、confirm 交互、错误处理均保持不变）。
- 不新增 npm 依赖、不引入 UI 组件库/CSS 框架（保持纯 CSS + 现有 JSX 的轻量改版）。
- 不做深色模式、i18n、PWA 或换肤等扩展。
- 不修改服务端与 CLI。
### Boundary with neighboring modules
- 仅归属 filehub-web（`admin-web/`）展示层；`docs/api/v1-contract.md` 与 001/003 交付不受影响。

## Requirement Review
- 需求合理：GitHub/Primer 是成熟、无障碍友好的设计语言，按此风格改造不改变产品流程，仅提升可用性与视觉一致性，属于有界单模块展示层重构。
- 权衡：
  - 以纯 CSS + 少量 JSX 结构调整实现 Primer 风格子集，避免引入组件库依赖带来的构建图与回归风险；Color 与 Space 用 CSS 变量集中定义，便于后续微调。
  - 保留全部文案与可访问标签，使现有单元测试（按 label/role/名称查询）继续稳定；如某处结构导致断言失效，只微调测试，不改业务行为。
  - 吸顶深色页头与窄屏横向滚动属于低风险布局能力，不改变路由与状态；深色页头沿用现有"深色顶栏"结构，只是按 GitHub 配色精修。
- 备选：引入 Primer React / Tailwind 可获得逐像素官方组件，但会改变依赖/构建图并扩大回归面，本提案选择不动依赖树、用现有 JSX + 样式表实现。

## Proposal Items
| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-web-layout-polish | admin-web 全部页面按 GitHub/Primer 风格统一布局与视觉：深色页头、Box 卡片、GitHub 按钮、列表式表格、Label 徽标、空态/错误态、焦点态与窄屏适配 | 仅改 `admin-web/src/`（样式与页面/组件结构微调）；不改业务逻辑、路由、文案、API 语义 | 纯 CSS 实现 Primer 子集，不引入 UI 库，保持依赖树与构建产物形态不变 | `npm run build`、`test:unit`、`test:integration` 通过；逐页浏览器抽查与 GitHub 风格对照 | 不改深色模式/i18n/换肤，不改服务端与 CLI，不新增 npm 依赖 |

## Success Criteria
- Concrete user-visible or system-visible result:
  - 登录页、项目列表、项目详情、协作者管理、Token 管理五类页面呈现 GitHub Primer 风格的统一视觉语言：深色页头 + 浅色画布 + 白底 Box + 列表式表格 + GitHub 按钮/Label 徽标，窄屏不横向破版。
  - 页面文案、路由、接口行为、一次性 JWT 展示与错误处理与现状完全一致。
- Required evidence:
  - `npm run build` 通过（tsc 无类型错误 + vite 构建成功）。
  - `npm run test:unit` 与 `npm run test:integration` 通过；如结构改动影响断言，保留相应断言调整记录。
  - 浏览器抽查记录（截图或描述）覆盖：登录页、项目列表、项目详情、协作者、Token 创建/编辑、空态与错误态、窄屏宽度，并与 GitHub Primer 视觉特征（灰底画布、白底 Box、行分割线表格、focus ring、Label 徽标）逐项对照。
- Explicit non-goals:
  - 不改变业务语义、不新增依赖、不改深色模式/多语言/换肤、不触碰服务端与 CLI。

## Risks
- 视觉重构理论上可能影响可访问性；通过保留语义标签、`:focus-visible` 高对比焦点、文本对比度检查与浏览器抽查来覆盖。
- "GitHub 风格"是设计语言级目标，不追求逐像素复刻官方组件；以 Primer 色板、排版、Box/按钮/表格/徽标模式为验收基准，具体像素差异不作为验收阻塞项。
- 结构调整可能波及单元测试断言；确认后先跑测试基线，再以最小断言微调跟上。
- 吸顶导航与横向滚动容器在极端窄屏（<360px）可能出现堆叠；抽查时以常见手机宽度（375px 起）为验收基线，更小宽度不承诺像素级适配。
