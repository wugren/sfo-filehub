# admin-web 按交互原型重新实现（原型交互/版式 + GitHub 浅色系 + 中/英语言切换 + 原型字体）

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/007-admin-web-prototype-reimplement/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/007-admin-web-prototype-reimplement/proposal.md
- Affected paths: admin-web/src/**
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 按原型（docs/versions/v0.1/modules/filehub/002-filehub-web/prototype/）重建 admin-web 展示层：左侧固定侧栏导航（Projects / API Tokens）、项目详情页 Versions / Collaborators 页签、新建/删除/可见性/协作者/Token 操作收敛为 Modal 与 Confirm 对话框、JWT 一次性展示弹窗、下载按钮 downloading/done 状态、Badge/空态/错误态体系。
- 颜色整体映射为 GitHub Primer 浅色系（画布 #f6f8fa、白底卡片/侧栏 #ffffff、边框 #d0d7de、正文 #1f2328、激活蓝 #0969da、主按钮绿 #1f883d、危险红 #cf222e），组件形态/间距/交互语义保持原型；字体按原型 Inter + JetBrains Mono（含系统回退，Google Fonts @import 跟随原型）。
- 文案提供中文/英文两种显示语言：自研轻量 i18n（React Context + 类型化字典），登录前右上角与侧栏用户区提供切换入口，默认跟随浏览器语言，偏好存 localStorage（仅语言标识，不存凭据），切换即时生效；技术字段（user_id、scope、SHA-256 等）不翻译。
- 数据与行为继续走真实 v1 API + session：登录/401 续期一次/本地登出、项目 CRUD 与可见性、版本列表与 fetch->blob 下载、协作者增改删、token 创建/编辑(重签)/轮换/撤销与 JWT 一次性展示；不落地原型 mock 数据。
- 协作者从独立路由收敛为详情页页签；/projects/:id/members 保留并重定向到详情页 Collaborators 页签。
- 实现约束：零新增 npm 依赖（纯 CSS + 内联 SVG 复刻 lucide 图标），不动 admin-web/src/api/ 与 util/download.ts 的契约与传输语义。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no（零新增 npm 依赖；admin-web/dist 仍为同一 Vite 静态站点交付形态）
- Material UI, accessibility, localization, or navigation workflow change: yes（本任务即批准的展示层重建与中/英语言切换目标；证据见提案 P-001/P-002 与 completion-report，按 standard 层流程执行）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: cd admin-web && npm run build && npm run test:unit && npm run test:integration && npm run test:dv
- Result: passed
- Residual risk or follow-up:
  - Google Fonts 在受限网络环境不可达时以系统回退栈渲染；离线自托管字体作为后续可选。
  - 自研字典的中/英文案颗粒度以"全部可见 UI 标签均有对应语言"为验收口径。
