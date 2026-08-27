# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/047-project-pagination-all-consumers.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - CLI（`cli/src/apiclient/mod.rs`）：`list_projects` 改为 `?limit=500&offset=`
    分页循环，用 `X-Total-Count` 驱动翻页，header 缺失/非法回退单页；公开
    签名与方法名不变。`resolve_project` 与 login 校验自动获得全量语义，第
    101+ 项目按名解析不再误报「不存在」。
  - admin-web（`api/client.ts`、`ProjectDetailPage.tsx`、`TokensPage.tsx`）：
    新增 `getProject`（消费 040 直查端点）与 `listAllProjects`（500/页循环，
    空页/达总量即停，页大小钳制 1..500）；详情页不再从首屏列表按 id 查找
    （404 展示文案保留），Token 页 Specified 勾选与 scope 名称展示覆盖全部
    可见项目。
  - 测试与契约：CLI mock 支持 query 分页与 `x-total-count`（`tok-paged`
    520 项目夹具），新增跨页解析 + 缺失名拒绝用例；admin client 单测新增
    直查/全量拉取/无总量头回退/页大小钳制；TokensPage 新增跨页 Specified
    选择用例；契约桩新增 `GET /projects/{id}` 与分页断言；v1-contract.md
    消费对齐补 CLI/详情页/Token 页说明。
- Handoff:
  - `cargo test -p filehub-cli --test api_integration -- --test-threads=1`：
    16/16；`cargo test -p filehub-cli --test cmd_integration -- --test-threads=1`：
    14/14；
  - `npm run test:unit`：57/57；`npm run test:integration`：9/9；
  - `npm run build`：tsc + vite 构建通过；
  - `lower-tier-check.py --profile pre-edit` 通过（任务开始基线已捕获）。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-cli-project-pagination | `list_projects` 分页拉全量（`X-Total-Count` 驱动、缺失回退单页），`resolve_project` 第 101+ 项目可解析 | proposal.md P-001 | cli/src/apiclient/mod.rs 分页循环 + `get_json_page`；mock 分页夹具；api_integration 16/16、cmd_integration 14/14 | 匹配 | pass |
| fh-web-project-pagination-consumers | `ApiClient.getProject` 与详情页直查；`listAllProjects` 与 TokensPage 全量勾选/展示 | proposal.md P-002 | admin-web client 两方法、两页面改造；契约桩直查发回正确项目 | 匹配 | pass |
| fh-pagination-consumer-tests | CLI mock 分页 + 跨页用例；admin client/组件/契约测试适配补用例 | proposal.md P-003 | cli/api_integration 新用例；client 单测 4 例；TokensPage 跨页 1 例；契约桩新增断言 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | CLI 分页循环边界（`page_len==0`、`projects.len() >= total`、offset 饱和加法）；`listAllProjects` 空页终止与 `items.length >= total`；详情页直查错误分类 | 反向推演：总量头缺失旧服务端必须等价于旧单页语义（不能整页丢弃或死循环）；服务端 limit 上限 500 与 admin 页大小 500 同时变化时的响应；详情页 404 必须仍显示既有「页面不存在」而非裸服务端文案 | mock 无总量头 fixture 场景由既有 15 个 api_integration 用例隐式覆盖；详情页测试改为 `getProject` 后仍断言版本表/按钮零回归 | pass |
| boundaries-and-failure-paths | 第 101+ 项目名（520 项目第 2 页）、不存在项目名仍退 5、限流 9999→500 钳制、`limit=1` 多页累计、契约桩 404 直查 | 边界推演：服务端返回空页但 total 未清零（循环必须终止）；offset 递增用 page 长度而非固定 500（防服务端收缩）；无 header/非数字 header 均回退单页；TokensPage 空项目列表不得卡死（total=0 首页后终止） | 全部边界由新增用例覆盖：跨页 520→第 2 页命中、missing 拒绝、9999 钳制 500、limit=1 两页累计；契约桩 9999 404 | pass |
| regression-and-side-effects | 既有 15 例 cli api_integration、14 例 cmd_integration、54 例 admin 单测、8 例契约测试逐一回归；`listProjects`/`listProjectsPage` 等既有 API 未改签名 | 排查：mock 增加 `x-total-count` 后旧 CLI 路径（无 query 假设）是否破坏——route 仍可处理无 query 请求；admin 组件桩 `endsWith` 不再匹配带 query URL 的隐患已在 TokensPage 桩修复并回归；ProjectDetailPage 三个组件用例改为 `getProject` 后断言不变 | 全部既有用例通过且新增用例彼此独立；唯一测试桩注意点（query 后缀匹配）已记录并修复 | pass |

## Verification

- Targeted check:
  - `cargo test -p filehub-cli --test api_integration -- --test-threads=1`：
    16/16 通过（含新增跨页解析用例）；
  - `cargo test -p filehub-cli --test cmd_integration -- --test-threads=1`：
    14/14 通过；
  - `npm run test:unit`：12 个测试文件 57/57 通过；
  - `npm run test:integration`：9/9 通过；
  - `npm run build`：tsc + vite 构建通过；
  - `lower-tier-check.py --profile pre-edit` 通过（任务开始基线已捕获）。
- Result: pass
- Exception reason: not-applicable（目标验证全部通过，无豁免）。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `listAllProjects`/CLI `list_projects` 均为分页拉全量；服务端 040 无按名/搜索过滤接口 | 项目数极大时 CLI 解析与 Token 页加载的请求次数随可见项目数线性增长（最多 500/请求）；当前建议量级内可用，后续若出现千级项目建议服务端增加按名过滤/搜索契约后收敛 | no |
| F-2 | low | admin-web 组件测试 stub 原本用 `endsWith("/api/v1/projects")` 匹配列表请求 | 带 `?limit/offset` 的 URL 不命中原判断，三个既有用例一度 404；已在测试桩改用前缀匹配并补充显式 cleanup，属于测试装置调整，产品代码零影响 | no |
| F-3 | low | 机械变更清单包含 `third_party/sfo-account/src/account_server.rs`（基线 11:57 快照 vs 12:09 当前内容，差异为 `ip_only` 实现） | 任务窗口内共享工作树其他在制工作并发更新了该 untracked 文件，非本任务交付；基线/变更清单为机械生成证据，本任务 scope 与 change record 均不包含该路径 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-001~P-003 全部落地且相互一致：CLI 跨页按名解析通过（520 项目第
  2 页命中断言成立），详情页直查消除首屏截断，Token 页全量项目勾选覆盖第二页，
  无总量头回退路径由既有与新增用例共同守护；admin-web 57 单测、9 契约、
  cli 16 api_integration + 14 cmd_integration 全部通过，构建通过，
  F-1/F-2 为低危非阻塞记录。
