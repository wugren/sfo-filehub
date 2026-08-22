# admin-web Token 表单：GitHub 式有效期预设 + 权限默认与全选/全不选

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/008-admin-web-token-form/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/008-admin-web-token-form/proposal.md
- Affected paths: admin-web/src/pages/TokensPage.tsx、admin-web/src/i18n/messages.ts、admin-web/tests/unit/tokens-scope.test.ts
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- TokenFormModal 的有效期控件由“不过期/设置日期 + 日期输入”改为 GitHub 式五档预设单选：1周（7 天）、1个月（30 天，创建默认）、半年（183 天）、1年（365 天）、不过期；保存时按当前时间 + 天数换算出 UTC ISO `expires_at`，不过期则传 `null`（创建/更新均不携带该字段）。
- 权限默认值：创建模式默认勾选 `metadata:read`、`artifacts:read`、`artifacts:write`；编辑模式沿用 init 传入的 Token 原 scopes。
- 权限区新增两个复选框：`全选`（当且仅当全部选中时勾选；点击在“全选/清空”间切换）与 `全不选`（当且仅当零选中时勾选；点击清空）。保留“至少选择一个权限”校验。
- 移除自定义日期输入；`tokens.form.setDate`/`invalidDate`/`dateRange` 文案键确认无引用后删除，`expiresAtHint` 语义保留（编辑时 expires_at 留空 = 不修改）。
- 新增导出纯函数 `EXPIRY_PRESETS`、`expiresAtForPreset` 与 `DEFAULT_SCOPES`，在 tokens-scope.test.ts 中做确定性单元断言。
- 实现约束：零新增 npm 依赖，不改 api/contract.ts、服务端与 CLI；`expires_at`/`scopes` 提交契约不变。

## Risk Screen

- Public contract, protocol, or CLI change: no
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: yes（即本任务已批准的表单交互目标：有效期预设、权限默认与全选/全不选；证据见提案 P-001 与 completion-report，按 standard 层流程执行，公共契约与安全边界不变）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: cd admin-web && npm run build && npm run test:unit && npm run test:integration
- Result: passed
- Residual risk or follow-up:
  - 编辑模式下默认“不过期”仍沿用旧语义（不携带 `expires_at`，服务端保留原过期时间）；若需在编辑中显式清除过期时间，需另行扩展 `TokenUpdateInput` 传 `expires_at: null`，本次不在范围内。
  - 预设有效期以保存时刻换算，表单停留时间不会影响最终过期时刻的准确性（仅稍晚于打开时刻）。
  - 浏览器交互抽查（Playwright，zh-CN）：五档预设可见且默认 1个月、默认勾选三项权限、全选/全不选、1年提交 ~365 天后 ISO、不过期不携带 expires_at，8/8 通过；截图存于 /tmp/pw/shots/token-form-008.png。
