---
task_manifest: task.yaml
status: approved
---

# admin-web SHA-256 复制按钮与版本操作列对齐修正

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment
- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 本次修改仅影响 `admin-web` 项目详情页中的按钮展示和操作列布局，不改变 API、数据、服务端、CLI 或依赖图。
  - 需求涉及 UI 操作可用性与对齐行为，属于已完成版本表格功能的局部前端修正；按 standard 流程执行针对性测试和独立缺陷发现。
- Proposal and tier confirmation: 用户于 2026-08-21 回复“确认”，采纳本提案与 standard 层级；`workflow_tier` 已写为 `standard`，本提案置为 `status: approved`。

## Background and Goal
- 当前 SHA-256 复制按钮同时显示图标和文字，用户希望仅保留紧跟 hash 文本的复制图标。
- 锁定版本没有删除按钮，未锁定版本有删除按钮，当前操作列的右对齐方式会导致两个版本的下载按钮位置不一致。
- 目标是缩小复制控件并保持下载按钮在锁定/未锁定版本之间的水平位置一致。

## Scope
### In scope
- SHA-256 单元格保留复制图标按钮，移除按钮内的可见“复制/已复制”文字；保留 `aria-label`、`title` 和成功状态，确保可访问性与反馈不变。
- 将版本文件操作区改为稳定的双槽布局，为删除按钮在锁定版本中保留同等宽度的占位槽，使下载按钮与未锁定版本对齐。
- 增加针对复制按钮可见结构和锁定/未锁定操作槽位的单元测试；不新增依赖。

### Out of scope
- 不修改 SHA-256 值、复制逻辑、版本锁定逻辑、删除逻辑、下载 API 或其他页面。
- 不改变复制成功反馈语义，不移除按钮的辅助功能名称。
- 不进行整体表格或页面布局重构。

### Boundary with neighboring modules
- 仅属于 `filehub` 模块的 `filehub-web` 交付面，变更限定在详情页 JSX、对应 CSS 和单元测试；不触及 `filehub-server`、`filehub-cli` 或 v1 API 契约。

## Requirement Review
- 需求合理：复制按钮的主要动作可由图标表达，保留无障碍名称即可减少 hash 单元格的视觉干扰；操作区预留删除槽位可以解决锁定状态下下载按钮漂移。
- 采用固定双槽布局而不是依赖按钮数量自动排列，保证锁定和未锁定版本的下载按钮具有同一列起点，同时保留删除按钮的现有交互。
- 锁定版本的占位元素不接受焦点、不参与语义操作，仅用于维持视觉布局；真实删除按钮仍只在未锁定版本出现。

## Proposal Items
| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-web-version-actions | SHA-256 复制控件只显示图标，并紧跟 hash 文本；保留可访问名称和复制成功状态 | 仅改详情页 SHA 单元格 JSX/CSS | 图标按钮依赖 aria-label/title 提供语义，不再显示可见文字 | 测试验证按钮只有图标、保留 `aria-label`，复制行为继续通过 | 不移除复制功能或反馈 |
| P-002 | fh-web-version-actions | 锁定与未锁定版本的下载按钮位置保持一致 | 仅改详情页文件操作区 JSX/CSS | 锁定版本保留不可见布局槽位，操作区占用固定宽度 | 测试验证两种状态的操作区均有稳定下载/删除槽位；构建通过 | 不改变锁定/删除权限和下载调用 |

## Success Criteria
- Concrete user-visible or system-visible result:
  - SHA-256 文本后只显示一个复制图标，不显示“复制/已复制”文字；鼠标悬停和辅助技术仍可识别复制动作及成功状态。
  - 锁定版本和未锁定版本的文件下载按钮在同一操作列位置对齐；未锁定版本仍显示删除按钮，锁定版本仍不可删除。
- Required evidence:
  - `npm run build` 通过。
  - `npm run test:unit` 通过，并覆盖复制按钮结构和锁定/未锁定操作区。
  - `npm run test:integration` 与 `npm run test:dv` 通过。
  - 独立检查复制可访问性、锁定状态边界和操作列布局副作用。
- Explicit non-goals:
  - 不改 API、服务端、CLI、版本数据和其他页面。

## Risks
- 仅图标按钮如果缺少可访问名称会降低可用性；实现中必须保留 `aria-label` 与 `title`，并通过测试校验。
- 占位槽位如果仍可获得焦点或被读屏器识别会产生噪声；使用 `aria-hidden` 的非交互元素，不渲染 disabled button。
