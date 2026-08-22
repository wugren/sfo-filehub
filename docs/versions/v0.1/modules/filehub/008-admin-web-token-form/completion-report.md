# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/008-admin-web-token-form.md

## Delivery Summary
- Outcome: Token 创建/编辑表单的有效期由“不过期/设置日期 + 日期输入”改为 GitHub 式五档预设（1周=7 天、1个月=30 天且为创建默认、半年=183 天、1年=365 天、不过期），保存时换算出 UTC ISO `expires_at`（不过期则不携带）；新建权限默认勾选 metadata:read、artifacts:read、artifacts:write；权限区新增 全选/全不选 复选框。移除自定义日期输入及其失效文案键，编辑模式默认“不过期”沿用旧语义（不修改 expires_at）。
- Handoff: `npm run build`（tsc + vite）、`npm run test:unit`（33/33，含新增预设/默认权限断言）、`npm run test:integration`（7/7）、`npm run test:dv` 全部通过；Playwright（zh-CN + 本地 mock API）8/8 项交互断言通过，创建表单截图存于 /tmp/pw/shots/token-form-008.png。

## Proposal Consistency
| change_id | Requirement or Boundary | Proposal Source | Delivery Evidence | Finding | Status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-web-token-form-expiry-scopes | 五档有效期预设（1周/1个月/半年/1年/不过期，创建默认 1个月，移除日期输入）；新建默认勾选 metadata:read、artifacts:read、artifacts:write；权限区提供全选/全不选；提交契约不变、不新增依赖、不改服务端 | proposal.md P-001 与 Scope/Success Criteria | `TokensPage.tsx`（EXPIRY_PRESETS/EXPIRY_PRESET_DAYS/expiresAtForPreset/DEFAULT_SCOPES、预设单选、全选/全不选）、`messages.ts`（新增 6 键、删除 3 键）、`styles.css`（segmented 换行 + scope-toggle-row）、`tokens-scope.test.ts`（新增 3 组断言）；服务端与 `api/contract.ts` 未改动 | 匹配 | pass |

## Independent Defect Discovery
| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | TokensPage diff（预设换算、保存分支、初始化、编辑分支）与 changelog | 检查创建/编辑两条提交路径的 `expires_at` 兜底：确认 never→null→不携带字段；编辑默认 never 与旧“不过期”语义一致（保留原过期时间）；编辑 scopes 沿用 init 而非默认三项；`projectScope`/重签警告链路未被改动 | 未发现逻辑缺陷；携带/省略 expires_at 与新旧行为一致 | pass |
| boundaries-and-failure-paths | 全选/全不选与“至少一个权限”校验、服务端 365 天上限、i18n 死键、窄屏布局 | 全不选后保存应被 scopesRequired 拦截（校验条件未变）；最大预设 365 天 ≤ MAX_EXPIRY；`rg` 确认 setDate/invalidDate/dateRange 无残留引用；segmented 增加 flex-wrap 防窄屏挤压 | 未发现越界路径；四档时长与服务端上限有安全余量 | pass |
| regression-and-side-effects | build/单测/集成/dv 输出、i18n 类型、依赖文件 | 检查 tsc 是否捕获被删文案键的残留引用、测试是否仍按中文断言通过、dist 是否重新构建、package.json/lock 与 server/CLI 是否被误改 | build、单测 33、集成 7、dv 通过；尚无相关变更 | pass |

## Verification
- Targeted check: `cd admin-web && npm run build && npm run test:unit && npm run test:integration && npm run test:dv`；另用 Playwright（chromium headless + 本地 mock API，zh-CN）对 Token 创建表单做交互断言
- Result: passed
- 验证明细：构建成功；单测 33/33、集成 7/7、dv dist 校验通过；Playwright 8/8（预设文案与顺序、默认 1个月、默认三项权限、全选六项、全不选清空、1年提交 ~365 天 ISO、不过期不携带 expires_at）。
- Exception reason: not-applicable

## Findings
| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | info | 代码评审与提案边界 | 编辑模式“不过期”不显式清除既有过期时间（旧语义继承）；如需清除需服务端/契约扩展 `expires_at: null`，已记入变更记录残余项，非本次需求范围 | no |

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 交付贴合已批准提案 P-001 及用户拍板的三项决策（默认 30 天、183 天半年、移除日期输入）；提交契约、服务端与既有流程未改动，构建、单测、集成、dist 与浏览器交互断言全部通过，仅剩余编辑清除过期时间的 info 级后续项。
