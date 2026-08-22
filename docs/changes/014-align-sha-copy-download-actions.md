# admin-web SHA-256 复制按钮与版本操作列对齐修正

- Status: complete
- Owner module: filehub（filehub-web / admin-web）
- Task manifest: docs/versions/v0.1/modules/filehub/014-align-sha-copy-download-actions/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/014-align-sha-copy-download-actions/proposal.md
- Affected paths: admin-web/src/pages/ProjectDetailPage.tsx, admin-web/src/styles.css, admin-web/tests/unit/components/ProjectDetailPage.test.tsx
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

将 SHA-256 复制按钮收缩为仅图标但保留可访问名称；将下载/删除操作包装为固定双槽布局，锁定版本用非交互占位元素补齐删除槽位，保证下载按钮位置稳定。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no（局部按钮展示和操作列布局修正，保留复制语义与既有操作）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `npm run build`, `npm run test:unit`, `npm run test:integration`, `npm run test:dv`
- Result: passed
- Residual risk or follow-up: 复制按钮改为图标后依赖 `aria-label` 和 `title` 提供语义；既有 React Router future flag 与 `act(...)` 警告仍存在，但未新增失败。
