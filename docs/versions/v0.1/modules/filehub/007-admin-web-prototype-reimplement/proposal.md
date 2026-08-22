---
task_manifest: task.yaml
status: approved
---

# admin-web 按交互原型重新实现（原型交互/版式 + GitHub 浅色系 + 中/英语言切换 + 原型字体）

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment
- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 本次按原型重做 admin-web 全部页面结构、交互与展示层（侧栏导航、项目详情页签、Modal/Confirm、JWT 一次性展示、下载状态等），并新增中/英显示语言选择，命中 ui-workflow/可访问性/本地化触发器，不满足 trivial 条件。
  - 范围收敛在单个前端模块：不改 v1 API 契约、会话/token 安全语义、数据模型、服务端/CLI 与静态站交付形态；语言机制为页面内轻量实现（不引入第三方 i18n 包与组件库），不构成 high-risk；按 standard 执行（一份变更记录 + 完成后独立缺陷发现与 completion 校验）。
- Proposal and tier confirmation:
  - 用户方向修订记录：
    1. 2026-08-21 第一轮：文案中英文一起采用；整体 GitHub 浅色系；交互不要修改（当时理解为保留现状）。
    2. 2026-08-21 第二轮：交互需要与原型一致；仅颜色调整为 GitHub 浅色系；字体按原型。
    3. 2026-08-21 第三轮（最终）：文案不是中英并置，而是可以选择显示语言，支持中文和英文。
  - 本提案据此最终修订：交互/版式/字体按原型；颜色为 GitHub 浅色系；文案提供中文/英文可切换显示。
  - 用户于 2026-08-21 回复"确认"，采纳 standard 层级；workflow_tier 已写为 standard，本提案已置为 status: approved。

## Background and Goal
- 现状：admin-web/ 功能完整（登录/session、项目、版本下载、协作者、token 管理），006 采用 GitHub Primer 浅色风格与顶部深色页头，协作者为独立路由，项目创建/删除等为行内表单与 window.confirm，文案固定中文。
- 原型：docs/versions/v0.1/modules/filehub/002-filehub-web/prototype/ 是设计交互原型：左侧固定导航栏（Projects / API Tokens）、项目详情页内 Versions / Collaborators 页签、创建项目/添加协作者/Token 表单为 Modal、删除/改可见性/轮换/撤销为 Confirm 对话框、JWT 一次性展示弹窗、下载按钮状态、Badge/空态/错误态体系，字体为 Inter + JetBrains Mono（基础字号 15px），文案为英文。
- 目标：按原型重建 admin-web 的交互与版式（真实数据继续走 v1 API，不落地 mock）；配色改为 GitHub 浅色系；字体按原型；全部可见文案支持中文/英文两种显示语言并由用户选择，不采用中英并置。

## Scope
### In scope
- 交互与版式（严格对齐原型）：
  - 登录页：按原型结构为居中卡片（品牌标识 + 引导语 + 用户名/密码 + 错误横幅 + loading），继续真实登录与会话逻辑。
  - 左侧固定侧栏：品牌标识 + Projects / API Tokens 导航（激活态）+ 底部当前用户（头像首字母、用户名、id）+ Sign out；内容区在侧栏右侧。
  - 项目页：页头（标题 + 计数徽标）+ New Project 按钮；表格 Name / Visibility / Owner / Actions；行点击进详情；新建项目、可见性切换、删除均为 Modal/Confirm；空态与错误态按原型。
  - 项目详情页：面包屑（Projects / 项目名）、项目名 + 可见性 Badge + owner、Make private/public 与 Delete 操作（Confirm）；Versions / Collaborators 页签。
    - Versions 页签：版本表格（latest 标记、size、SHA-256 截断、发布时间、下载按钮 Downloading…/Done 状态），下载走真实 fetch -> blob。
    - Collaborators 页签：owner 隐式 admin 提示、Add collaborator Modal（数字 user_id + 角色）、行内角色选择、移除 Confirm；服务端 403 以错误横幅呈现。
    - 路由：/projects/:id 承载详情页；/projects/:id/members 重定向到该详情页并默认落在 Collaborators 页签（兼容既有链接）。
  - Token 页：页头 + New Token 按钮；表格 Name / Project Scope / Permissions / Created / Actions（Edit / Rotate / Revoke）；创建/编辑双列 Modal（名称、All/Specific projects、有效期最长 1 年、权限 scope 复选框、编辑重签提示）；Rotate/Revoke Confirm；JWT 一次性展示 Modal（警告提示、可复制明文、Done）。
- 颜色：GitHub Primer 浅色系（原型深色令牌整体映射）：
  - 画布 #f6f8fa、卡片/侧栏 #ffffff、边框 #d0d7de、正文 #1f2328、次要文本 #59636e、强调/激活蓝 #0969da、成功/主按钮绿 #1f883d、危险 #cf222e、角色徽标沿用浅底细边框风格；
  - primary 激活态、focus ring、Badge 变体（public/private/scope/role）、表格 hover 等全部改为浅色等价物。
- 语言显示（i18n，轻量自实现）：
  - 支持中文（zh）与英文（en）两种显示语言，直觉切换，不采用同屏中英并置；
  - 语言入口放在侧栏底部用户区附近（如 中/EN 切换按钮或 select），对登录前页面也在右上角提供切换；
  - 全部可见文案（页面标题/表头/按钮/表单标签/空态/错误提示/面包屑/Confirm/JWT 提示/下载状态等）纳入语言字典；技术字段（user_id、owner、visibility、scope、SHA-256、.tar.gz、latest 等）保留原文不翻译；
  - 切换即时生效并持久化语言偏好（localStorage 只存语言，不存任何凭据）；初始语言跟随浏览器语言（zh 开头为中文，否则英文）；
  - 实现方式：React Context + 类型化字典，不引入第三方 i18n 库与 npm 依赖。
- 字体：按原型——Inter（400/500/600）为无衬线栈首选项、JetBrains Mono（400/500/600）为等宽栈首选项，基础字号 15px；加载方式随原型采用 Google Fonts @import，并保留系统回退栈（受限网络环境以回退渲染，不阻塞）。
- 工程与验证：保留 Vite + React + TS、现有 api/ 会话/客户端/错误层与 util 下载实现；页面/组件/样式按原型重建（纯 CSS + 内联 SVG 复刻 lucide 图标，不引入组件库）；跑 npm run build、test:unit、test:integration、test:dv，浏览器逐页对照原型抽查并验证中/英切换。

### Out of scope
- 不修改 v1 API 契约、服务端（server/）、CLI（cli/）与 admin-web/dist 交付形态；不实现任何服务端能力。
- 不使用原型 mock 数据与原型 npm 依赖（MUI/Radix/Emotion/Tailwind 等）：admin-web 零新增 npm 依赖（含零第三方 i18n 包）。
- 不做深色模式/主题切换、中英并置文案、PWA、换肤或组件库选型；不修改原型目录。

### Boundary with neighboring modules
- 仅归属 filehub-web（admin-web/）展示层；docs/api/v1-contract.md、001/003 交付不受影响。
- 页面不做本地权限判断，仅按服务端 401/403 呈现错误态；token JWT 明文仍在签发响应一次性展示、前端不落库；语言偏好仅存语言标识，不涉及凭据与用户数据。

## Requirement Review
- 需求合理：用户最终明确了"交互按原型、颜色 GitHub 浅色系、字体按原型、显示语言中/英可切换"的组合；原型覆盖全部目标页面与状态，作为交互/版式基准可执行。
- 关键取舍：
  - 语言机制采用自研轻量 Context + 字典，而非 i18n 框架：满足中/英完整切换且不引入依赖、不改变路由与交互；切换即时生效并通过 localStorage 记忆。
  - 颜色映射：以 GitHub Primer 浅色令牌替换原型深色令牌，保持组件形态、间距、圆角与交互不变；激活/主操作使用 GitHub 蓝/绿色约定（导航激活蓝、主按钮绿、危险红）。
  - 依赖：原型工程自带大量 UI 依赖，若整体引入会扩大构建图与供应链面；采用"原型结构/样式语义 + 纯 CSS + 内联 SVG（lucide 路径复刻）"落地，验收以交互与视觉对照为准。
  - 字体加载：跟随原型引用 Google Fonts 并带系统回退；若部署环境无法访问 Google Fonts，由回退栈保证可读性（自托管字体列为后续可选，不入本次范围）。
  - 路由兼容：协作者从独立页收敛进详情页页签（原型交互），保留 /projects/:id/members 重定向，避免既有链接失效。
- 备选：中英并置或仅单语言——均与用户"可选择显示语言"的明确指示冲突，予以排除。

## Proposal Items
| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-web-prototype-reimplement | admin-web 按原型重建交互/版式/组件/字体（侧栏、详情页签、Modal/Confirm、JWT 一次性展示、下载状态、Badge/空态/错误态），颜色整体采用 GitHub 浅色系 | 仅改 admin-web/src/（页面/组件/样式）与必要测试断言；不改 api 层语义、会话/token 语义、依赖与交付形态 | 纯 CSS + 内联 SVG 复刻，不引入原型依赖；Google Fonts 跟随原型并保留系统回退 | npm run build、test:unit、test:integration、test:dv 通过；浏览器逐页对照原型（交互/版式/字体）一致且为 GitHub 浅色配色 | 不动服务端/CLI/契约、不加依赖、不落地 mock、不做深色模式/并置文案/PWA |
| P-002 | fh-web-i18n-zh-en | 全部可见文案支持中文/英文两种显示语言：语言入口（登录前右上角 + 侧栏用户区）、即时切换、浏览器语言默认、localStorage 记忆、类型化字典覆盖全部页面/组件文案 | 仅改 admin-web/src/（i18n 上下文、字典、页面取词）与相应测试；不存凭据、不引第三方 i18n 包 | 自研轻量 i18n 而非框架，保持零新增依赖；技术字段不翻译 | 中/英两套语言下逐页核对文案完整；切换即时生效、刷新后保持；构建与测试全通过 | 不做多语言扩展框架/远程语言包/中英并置，不改 API 语义 |

## Success Criteria
- Concrete user-visible or system-visible result:
  - 五类页面（登录、项目列表、项目详情两页签、Token 管理）的布局与交互与原型一致：左侧导航、行点击进详情、Modal 表单、Confirm 对话框、下载状态、JWT 一次性展示、空态/错误态与 Badge 体系；
  - 整体配色为 GitHub 浅色系（浅色画布、白底卡片/侧栏、GitHub 蓝/绿/红色与浅色徽标），字体呈现为 Inter/JetBrains Mono（含回退）；
  - 页面文案可在中文/英文间切换：登录前与登录后均有入口、切换即时生效、刷新后保持、默认跟随浏览器语言，两套语言下各页文案完整无缺漏；
  - 真实 API 功能（登录/401 续期、项目增删改可见性、版本下载、协作者增改删、token 创建/编辑/轮换/撤销）保持可用。
- Required evidence:
  - npm run build（tsc + vite）通过；npm run test:unit 与 npm run test:integration 通过；npm run test:dv（dist 校验）通过。
  - 浏览器抽查（截图或逐项清单）覆盖：登录（含语言切换入口）、项目列表与新建/删除/可见性、详情页 Versions/Collaborators 页签、Token 创建/编辑/轮换/撤销与 JWT 展示、空态/错误态、窄屏；中/英语言各抽查一遍并核对切换即时性、刷新持久化与浏览器默认值。
  - 必要的测试断言调整记录（若页面结构与语言取词导致既有测试选择器失效，只调测试不改业务行为）。
- Explicit non-goals:
  - 不改变 API/认证/token 安全语义；不新增 npm 依赖；不落地原型 mock 数据；不做深色模式/中英并置/PWA/服务端与 CLI 改动；不改原型目录。

## Risks
- 原型为深色设计，浅色映射存在观感差异：以 GitHub Primer 令牌与组件形态对照为基准，像素级差异不阻塞，验收锁定"浅色系 + 原型结构/交互"。
- 自研 i18n 的字典遗漏风险：以全部页面逐项核对清单 + 两语言抽查覆盖；字段技术词不翻译以减少歧义。
- 页面结构重构可能波及既有测试断言（现有测试集中于 api/session 层，页面级断言少）：先跑测试基线，仅做最小断言调整并记录。
- Google Fonts 在部分网络环境不可达：以系统回退保证渲染；如部署侧要求离线自托管，作为后续项单独处理。
- 语言切换入口在登录前/后两个位置，需保证小屏不溢出：采用紧凑控件并在窄屏验收。
