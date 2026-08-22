---
task_manifest: task.yaml
status: approved
---

# admin-web 项目详情页版本表格对齐与 SHA-256 操作优化

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment
- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 本次修改仅影响 `admin-web` 项目详情页的展示与交互，不改变 API、持久化数据、服务端、CLI 或依赖图。
  - 需求涉及 UI 表格布局、完整哈希文本展示、复制按钮及对应可访问名称，属于明确的单模块前端行为修改；按 standard 流程保留变更记录并进行独立缺陷发现。
- Proposal and tier confirmation: 用户于 2026-08-21 回复“确认”，采纳本提案与 standard 层级；`workflow_tier` 已写为 `standard`，本提案置为 `status: approved`。

## Background and Goal
- 当前项目详情页为每个版本分别渲染一张表格，列宽由该版本内的内容独立计算，导致多个版本之间相同列的起始位置不一致。
- SHA-256 当前只展示前 16 个字符加省略号，无法直接核对完整摘要，也没有针对摘要的复制操作。
- 目标是在保持现有版本管理和下载流程不变的前提下，让所有版本表格使用一致的列布局，完整显示 SHA-256，并提供明确可用的复制按钮。

## Scope
### In scope
- 为项目详情页每个版本的应用表格定义稳定、可复用的列宽布局，使不同版本之间的应用、大小、SHA-256、更新时间和操作列对齐。
- SHA-256 单元格显示 API 返回的完整字符串，并通过必要的换行或横向滚动保证长文本可见，不再截断。
- 在 SHA-256 旁增加复制按钮，调用浏览器剪贴板 API，复制成功后使用现有国际化的“已复制”反馈，并提供按钮的可访问名称/提示。
- 补充中英文复制相关文案与针对完整哈希、复制行为和表格结构的前端测试；不新增依赖。

### Out of scope
- 不修改版本或应用 API 契约、服务端存储的 SHA-256 值、上传/下载逻辑、权限和锁定逻辑。
- 不改变其他页面的表格布局，不引入通用表格组件或第三方剪贴板库。
- 不承诺在浏览器禁用剪贴板权限时绕过浏览器安全策略；该情况下保留完整文本供用户手动复制。

### Boundary with neighboring modules
- 仅属于 `filehub` 模块的 `filehub-web` 交付面，变更限定在 `admin-web/src/` 与对应前端测试；不触及 `filehub-server`、`filehub-cli` 或 v1 API 契约。

## Requirement Review
- 需求合理：列对齐是当前多版本卡片布局的可读性问题，完整哈希与复制是发布产物核验的直接操作需求。
- 选择在每张版本表中使用相同的列定义和固定/最小表宽，而不是把所有版本合并为一张表，以保留现有版本卡片、锁定和上传操作的结构。
- 完整 SHA-256 会增加单元格宽度；通过统一列宽、等宽字体、必要的断行和容器横向滚动平衡核验可读性与窄屏适配。
- 复制反馈采用短暂的成功状态，不改变原有下载/删除操作状态；复制失败只保留可见完整文本，避免新增全局错误流程。

## Proposal Items
| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-web-version-table | 详情页所有版本表格使用一致的列定义，保证相同列在版本卡片之间对齐 | 仅改 `ProjectDetailPage.tsx` 与 `styles.css` 的版本表格结构/样式 | 统一列宽可能在窄屏产生容器内横向滚动，以换取跨版本稳定对齐 | 测试验证多个版本表头/列定义一致；`npm run build` 通过 | 不改其他页面和业务 API |
| P-002 | fh-web-version-table | SHA-256 显示完整值，并提供复制按钮及成功反馈 | 复用现有 `Icon`、`Btn` 和 i18n 机制 | 剪贴板受浏览器权限限制，失败时仍可手动复制完整文本 | 测试验证完整哈希存在、点击后调用剪贴板并显示反馈 | 不改变服务端摘要或新增第三方依赖 |

## Success Criteria
- Concrete user-visible or system-visible result:
  - 项目详情页中至少两个版本的表格具有一致列宽和列顺序，所有版本的对应列垂直对齐。
  - 每个应用的 SHA-256 完整显示，不出现固定的 16 字符截断和省略号；旁边有复制按钮，成功后显示“已复制”反馈。
  - 中文和英文界面均有正确的复制按钮可访问名称/反馈文案，现有下载、删除、锁定和上传行为不变。
- Required evidence:
  - `npm run build` 通过。
  - `npm run test:unit` 通过，并覆盖完整 SHA-256、复制调用、反馈状态以及多个版本表格列结构。
  - 完成前对长哈希、空版本、锁定版本、窄屏表格容器和剪贴板不可用场景做独立缺陷发现。
- Explicit non-goals:
  - 不修改后端/CLI/API，不新增依赖，不重构其他页面，不改变版本业务流程。

## Risks
- 完整哈希可能使表格在小屏幕上变宽；通过版本表格容器的横向滚动和必要的文本断行控制影响，避免页面整体溢出。
- 剪贴板 API 可能因权限或非安全上下文不可用；完整文本必须始终可见，复制按钮应安全失败而不影响页面其余操作。
- 新增按钮会扩大操作列附近的视觉密度；使用现有 ghost/small button 样式和 aria-label 保持一致性与可访问性。
