# admin-web 项目详情页版本表格对齐与 SHA-256 操作优化

- Status: complete
- Owner module: filehub（filehub-web / admin-web）
- Task manifest: docs/versions/v0.1/modules/filehub/012-align-version-columns-sha256/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/012-align-version-columns-sha256/proposal.md
- Affected paths: admin-web/src/pages/ProjectDetailPage.tsx, admin-web/src/styles.css, admin-web/src/i18n/messages.ts, admin-web/tests/unit/components/ProjectDetailPage.test.tsx
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

为每个版本表格增加相同的 `colgroup` 和固定最小表宽，使用容器内横向滚动承载长内容；SHA-256 改为完整值并在同一单元格提供复制按钮。复制成功状态只保留在当前版本/应用行，不改变下载、删除、锁定和上传行为。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no（仅限详情页局部展示与复制操作，保留语义按钮和中英文文案）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `npm run build`, `npm run test:unit`, `npm run test:integration`, `npm run test:dv`
- Result: passed
- Residual risk or follow-up: 真实浏览器的剪贴板权限仍受浏览器安全上下文控制；权限不可用时页面保留完整 SHA-256 文本，用户可手动复制。测试运行中保留既有 React Router future flag 与 `act(...)` 警告，未新增失败。
