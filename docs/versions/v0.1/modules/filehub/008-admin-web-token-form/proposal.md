---
task_manifest: task.yaml
status: approved
---

# admin-web Token 创建表单：GitHub 式有效期预设 + 权限默认与全选/全不选

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment
- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 本次改动限定在 admin-web 单个前端模块的 Token 创建/编辑表单，不涉及公共 API 契约、持久化数据、依赖/构建图、服务端/CLI 或发布面变更；预设有效期换算后仍落在服务端既有限制（`expires_at` 最长 1 年、可不过期）内，不构成 high-risk。
  - 但该改动属于表单交互（UI workflow）变更：替换有效期控件、改变权限默认值与新增全选/全不选交互，按 trivial 定义不满足“无可感知 UI 工作流影响”的条件；沿用本仓库 006/007 admin-web 任务的 standard 先例（变更记录 + 完成后独立缺陷发现）。
- Proposal and tier confirmation:
  - 用户于 2026-08-21 回复确认并拍板三项决策：① 创建默认有效期改为 30 天（非 006 现状的“不过期”）；② 半年按 183 天计算；③ 移除自定义日期输入，只保留五档预设。`workflow_tier` 已写为 `standard`，本提案置为 `status: approved`。

## Background and Goal
- 现状：Token 创建/编辑表单中，有效期只有“不过期 / 设置日期”两个选项，日期还需手填（最长 1 年）；权限复选框必选其中之一，无默认选择，也没有批量选择能力。
- 目标：参照 GitHub 创建 Token 的体验——
  - 有效期改为直接指定时长预设：`1周`、`1个月`、`半年`、`1年`、`不过期`，提交时换算为对应 `expires_at` 时间戳；`不过期` 不传 `expires_at`。创建默认选中 `1个月`（30 天），移除原“设置日期”自定义输入。
  - 新建 Token 时权限默认勾选 `metadata:read`、`artifacts:read`、`artifacts:write`；编辑时保持该 Token 已选权限不变。
  - 权限区提供 `全选` 与 `全不选` 两个复选框，一键切换全部权限。

## Scope
### In scope
- `admin-web/src/pages/TokensPage.tsx` 中 TokenFormModal 的交互调整：
  - 有效期控件：以预设单选（GitHub 风格）替换现有“不过期/设置日期”分段控件与日期输入；预设包括 1周（7 天）、1个月（30 天）、半年（183 天）、1年（365 天）、不过期；保存时按当前时间计算 ISO 时间戳。
  - 权限默认值：创建模式默认选中 `metadata:read`、`artifacts:read`、`artifacts:write`；编辑模式沿用 Token 当前 scopes。
  - 权限区新增 `全选` / `全不选` 复选框：全选=勾选 `ALL_SCOPES` 全部六项，全不选=清空；保留“至少选择一个权限”校验与文案。
- `admin-web/src/i18n/messages.ts`：补充有效期预设与全选/全不选的中英文文案；清理因日期控件移除而不复存在的文案键（若仍有引用则保留）。
- `admin-web/tests/`：为新增的预设换算/全选逻辑补充最小单元测试（沿用现有 node 环境 vitest 风格，提取纯函数便于断言）；保留现有测试文件不破坏的断言。
- 验证：`npm run build`、`npm run test:unit`、`npm run test:integration` 全部通过。
### Out of scope
- 不改服务端 `server/`：有效期仍受 `MAX_EXPIRY`（365 天）约束，`expires_at` 契约不变；本次预设（最长 365 天）均在其内。
- 不改 Token 列表展示、创建/编辑/轮换/撤销流程、项目范围选择、JWT 一次性展示与错误处理语义。
- 不新增 npm 依赖、不引入组件库、不改路由与会话逻辑。
- `expires_at` 展示：Token 表格当前不展示过期时间，本次不新增该列。
### Boundary with neighboring modules
- 仅归属 filehub-web（`admin-web/`）表单展示层；`docs/api/v1-contract.md`、服务端与 CLI 不受影响。

## Requirement Review
- 需求合理：GitHub 式有效期预设与权限默认/批量选择是成熟的可用性模式，能显著减少创建 Token 的摩擦与误配置；且与现有服务端能力完全兼容。
- 权衡与决定性选择：
  - 有效期以保存时刻换算时间戳，同一选择再次编辑会按新时间重签（与旧“设置日期”行为一致，符合现有编辑重签警告语义）。
  - 创建默认有效期：用户已确认默认 `1个月`（30 天，同 GitHub 默认）。
  - 权限默认值仅影响新建表单初始勾选，不影响服务端默认与既有 Token。
  - 半年按 183 天计算（用户已确认；多出的 3 天在服务端 365 天上限内安全冗余）。
  - 删除日期选择控件（用户已确认）；`invalidDate`/`dateRange` 等文案键若无引用则一并清理，保持 i18n 无死键。

## Proposal Items
| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-web-token-form-expiry-scopes | Token 创建/编辑表单：有效期改为 GitHub 式预设（1周/1个月/半年/1年/不过期）；新建时默认勾选 metadata:read、artifacts:read、artifacts:write；权限区新增全选/全不选复选框 | 仅改 admin-web 中 TokensPage 表单与 i18n 文案；提交的 `expires_at`/`scopes` 契约不变；编辑模式默认值沿用 Token 原权限 | 预设有效期以保存时刻换算出 ISO 时间戳；默认有效期保持现状“不过期”；半年按 183 天 | `npm run build`/`test:unit`/`test:integration` 通过；手工检查复选框默认值、全选/全不选、五档有效期提交值 | 不改服务端/CLI/API 契约，不新增依赖，不改 Token 列表与流程语义 |

## Success Criteria
- Concrete user-visible or system-visible result:
  - 新建 Token 表单中，有效期区域显示 `1周`、`1个月`、`半年`、`1年`、`不过期` 五档直接可选的预设项（不再要求手填日期）。
  - 新打开创建表单时，有效期默认选中 `1个月`（30 天）。
  - 新打开创建表单时，`metadata:read`、`artifacts:read`、`artifacts:write` 已默认勾选；权限区提供 `全选` 与 `全不选` 复选框且行为正确。
  - 选择 1周/1个月/半年/1年 时提交的 `expires_at` 分别为约 7/30/183/365 天后的 UTC ISO 时间；选择不过期时不传 `expires_at`；请求中 `scopes` 与勾选一致。
- Required evidence:
  - `npm run build` 通过（tsc 无类型错误 + vite 构建成功）。
  - `npm run test:unit` 与 `npm run test:integration` 通过，包含新增的预设换算与全选/全不选单元测试。
  - 浏览器/构建产物抽查：创建表单默认权限三选、全选/全不选切换、五档有效期文案可见且保存请求字段正确。
- Explicit non-goals:
  - 不修改服务端校验与 `expires_at` 上限；不改 Token 表格展示列；不改项目范围选择、轮换/撤销、JWT 一次性展示与错误处理；不新增依赖。

## Risks
- 表单交互变更可能影响既有工具/浏览器测试对“设置日期”控件的假设；通过保留提交契约不变与跑全量前端测试覆盖。
- 预设有效期在保存时计算，表单停留时间长会让实际过期时刻略晚于打开时刻，属于可接受的既有语义（与日期控件一致）。
- 若未来服务端收紧有效期上限（低于 365 天），预设 1年 会触发服务端拒绝；当前 `MAX_EXPIRY = 365` 天，无需联动。
