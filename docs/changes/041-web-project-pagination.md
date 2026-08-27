# admin-web 项目列表分页

- Status: complete
- Owner module: filehub（filehub-web / 002-web 前端）
- Task manifest: docs/versions/v0.1/modules/filehub/041-web-project-pagination/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/041-web-project-pagination/proposal.md
- Affected paths: admin-web/src/api/contract.ts、admin-web/src/api/client.ts、
  admin-web/src/pages/ProjectsPage.tsx、admin-web/src/i18n/messages.ts、
  admin-web/src/styles.css、admin-web/tests/unit/client.test.ts、
  admin-web/tests/unit/components/ProjectsPage.test.tsx、
  admin-web/tests/integration/contract.test.ts、docs/api/v1-contract.md
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- `contract.ts` 新增 `ProjectPage { items, total }`；`client.ts` 抽出
  `rawResponse` 并新增 `listProjectsPage(bearer, {limit, offset})`：请求携带
  `?limit/offset`，从 `X-Total-Count` 响应头解析总量（缺失/非数字回退
  items.length）；既有 `listProjects` 与其他方法保持原语义；
- `ProjectsPage` 增加分页状态（当前页、总量），加载
  `limit=10&offset=(page-1)*10`；表格下方新增「上一页/下一页」与
  「第 x / y 页 · 共 n 个项目」；count-badge 显示总量；新建后跳末页、
  删除导致空页时回退一页、首末页按钮禁用；
- i18n 新增三个双语文案；styles.css 增加 `.pager`/`.pager-info` 轻量样式；
- 契约文档在消费对齐处注明 admin-web 项目列表已走分页参数与总量头；
- 测试：client 单测（参数拼接/总量头/回退）、契约桩分页行为、新增
  ProjectsPage 组件测试（首屏、翻页、边界禁用、新建跳末页、删除空页回退）。

## Risk Screen

- Public contract, protocol, or CLI change: no（客户端只消费 040 已冻结的既有
  分页契约；文档仅补消费对齐说明，不改路由/响应形状）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no（vite 构建产物照常生成）
- Material UI, accessibility, localization, or navigation workflow change: yes
  （项目列表增加分页导航与中英文案；按钮为既有 `Btn` 组件，具备可见文本标签）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no（admin-web 内部收敛；
  不触碰 server/CLI）

## Verification

- Targeted check:
  - `npx tsc --noEmit -p tsconfig.json`：通过；
  - `npm run test:unit`：12 个测试文件 52/52 通过（含新增
    ProjectsPage 5 例与 client 分页 3 例）；
  - `npm run test:integration`：8/8 通过（含新增分页契约用例）；
  - `npm run build`：tsc + vite 构建通过。
- Result: pass
- Residual risk or follow-up: TokensPage 项目选择器与 ProjectDetailPage 仍用
  首屏 `listProjects`（超 100 条场景未迁移，属既有 non-goal）；跨域独立部署
  时需服务端 CORS expose `X-Total-Count`（当前 nginx 同源代理不需要）。
