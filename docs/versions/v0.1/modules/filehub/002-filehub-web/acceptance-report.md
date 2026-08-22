# filehub-web 验收报告（第二轮，返回修复后）

Risk profile: ./risk-profile.yaml

## Findings

| ID | Severity | Owning Stage | Correctness Category | Evidence | Problem | Blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-001 | none | none | overall | 关闭记录：`src/pages/TokensPage.tsx` `scopeError` 校验 + `tests/unit/tokens-scope.test.ts` 回归通过 | 首轮发现的「指定项目」空列表缺陷已修复并在本轮复核确认无残留 | no |
| F-002 | none | none | overall | 关闭记录：`src/pages/LoginPage.tsx` 按 `next` 导航 + `tests/unit/components/LoginPage.test.tsx` 用 `next=/tokens` 回归通过 | 首轮发现的登录成功丢失原始目标页缺陷已修复并在本轮复核确认无残留 | no |
| F-003 | none | none | overall | 关闭记录：`tests/unit/session-retry.test.ts` 通过；vitest 配置移至 `admin-web/tests/vitest.config.ts` 并登记测试阶段路径 | 首轮发现的 401 重试覆盖缺口与测试配置归属问题已修复并复核 | no |
| F-004 | none | none | overall | 收据链重建记录：`lifecycle.json` 因任务中补齐 task.yaml 规范字段经官方检查器与 task-transition 重建验证 | 无残留问题；本轮复核确认收据链与当前 task.yaml 绑定一致 | no |

## Object and Scope

- Task manifest: task.yaml
- Review date: 2026-08-20
- In-scope implementation: admin-web React 管理后台（登录/会话、项目/版本/下载、token 管理、协作者管理、独立构建）
- Review mode: independent falsification; conclusion selected after findings and category review，首轮 findings 经 `return --to implementation` 修复后本轮独立复核

## Requirement Coverage

| change_id | Requirement or Boundary | Source | Implementation Evidence | Finding | Status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-web-login | 登录后进入原始目标页；会话仅内存/sessionStorage；登出仅本地 | `proposal.md` P-01 | `src/pages/LoginPage.tsx`（next 导航）、`src/api/session.ts`、`tests/unit/components/LoginPage.test.tsx`、`session.test.ts` | F-002 已修复并回归通过 | pass |
| fh-web-project-versions | 可见项目/版本列表/下载带 Bearer（POST 更新语义/latest） | `proposal.md` P-02 | `src/pages/ProjectsPage.tsx`、`src/pages/ProjectDetailPage.tsx`、`src/api/client.ts`、契约桩 `contract.test.ts` | 集成契约与错误语义通过 | pass |
| fh-web-token-manage | 创建/列表（无过期列）/修改/轮换/撤销，明文一次 | `proposal.md` P-03 | `src/pages/TokensPage.tsx`（含 scopeError）、`src/api/client.ts`、`tests/unit/tokens-scope.test.ts`、`contract.test.ts` | F-001 已修复并回归通过 | pass |
| fh-web-members | 按 user_id 查看/添加/改级/移除，owner 不可管理 | `proposal.md` P-04 | `src/pages/MembersPage.tsx`、`src/api/client.ts`、`contract.test.ts` upsert/owner 403 断言 | 需求边界与错误语义通过 | pass |

## Independent Defect Discovery

| Category | Applicable Scope | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|------------------|--------------------|-------------------|----------------------------------|--------|
| requirement-and-behavior | 提案 P-01~P-04 全部页面行为与边界 | `proposal.md` 各 Proposal Item、修复后的 `src/pages/*`、LoginPage next 测试、契约桩断言 | 重查 next 跳转、无过期列、POST 语义、下载 Bearer、user_id 输入与越权呈现 | 首轮 F-001/F-002 已修复；其余需求项与实现一致，未发现新缺陷 | pass |
| logic-and-control-flow | LoginPage 认证跳转与 withAuthRetry 重试控制流程 | `LoginPage.tsx` effect/onSubmit、`session.ts withAuthRetry`、`session-retry.test.ts` | 构造 /tokens 未登录→登录成功路径与 401 重试/失败路径 | 无缺陷；next 导航与恰一次重试均有测试锁定 | pass |
| boundary-and-input | TokensPage 项目范围表单与 project_scope 序列化 | `TokensPage.tsx scopeError`、`contract.ts encodeProjectScope`、`tokens-scope.test.ts`、server tokens list 语义 | 空指定列表、All/Specified 形状、非法 id 输入路径 | 无缺陷；空指定范围在 UI 层拦截并单测覆盖 | pass |
| state-and-data-integrity | 会话持久化与 JWT 一次性展示 | `session.ts` 存储/恢复/登出、`TokensPage` issued 卡片、`session.test.ts` | 刷新恢复、登出清除、刷新失败清理、明文关闭后不可再读 | 无缺陷；sessionStorage 键完整清除，issued 状态仅组件内短时持有 | pass |
| error-handling-and-recovery | 两套响应格式与下载/续期失败 | `client.ts raw/envelope/fromV1`、`session.ts refreshOnce`、`session-retry.test.ts` | err!=0 仍 HTTP 200、403/404/422、网络失败、refresh 失败传播 | 无缺陷；错误分类与失败回退全部有断言 | pass |
| resource-lifetime-and-cleanup | fetch AbortController 与 ObjectURL | `client.ts raw/download`、`util/download.ts saveBlob` | 超时清理、下载成功后 revokeObjectURL、失败不泄漏 URL | 无缺陷；finally 清理与延迟 revoke 正常 | pass |
| concurrency-and-ordering | 前端运行时并发/顺序声明 | 设计 `## State and Ownership`、`session.ts` 续期策略 | 设计无并发声明；检查 refresh 串行化与同步存储 | not-applicable: 前端单线程，401 refresh 由 withAuthRetry 串行等待，无共享并发可变状态（设计证据：design.md State and Ownership 与 design/session.md 续期策略） | not-applicable |
| interface-and-compatibility | v1 契约、两套响应格式与 POST 更新语义 | `docs/api/v1-contract.md`、`client.ts`、契约桩断言 | project_scope JSON 形状、POST 更新语义、列表无 expires_at、下载附件头 | 无缺陷；契约事实由集成测试锁定 | pass |
| security-and-capacity | 凭据存储面、JWT 明文与越权边界 | `session.ts`（无 localStorage）、`client.ts` Authorization、`ProtectedRoute` | 检查凭据明文写入面、JWT 落库/日志、越权本地放行 | 无缺陷；凭据仅内存/sessionStorage，JWT 仅签发响应展示，权限全部服务端判定；下载匿名/带 Bearer 双向断言 | pass |
| test-adequacy | 测试集能否暴露已交付行为的缺陷 | `tests/unit`（30 例）、`tests/integration`（7 例）、DV 构建校验、`testing.md` gap 记录、`tests/vitest.config.ts` 登记 | 寻找未覆盖分支与可逃逸缺陷；验证 F-001/F-002 回归用例 | 无缺陷；withAuthRetry 与空 scope 分支已补测，极端 UI 交互（剪贴板/确认框）按 per-branch 原因记录为 manual | pass |

## Document Consistency

| Document | Source | Implementation Consistency | Finding | Status |
|----------|--------|----------------------------|---------|--------|
| design | `design.md` + `design/*.md` | 实现按设计子模块落地，接口签名与 Key Flows 一致；login 导航与 scope 校验未改变设计边界 | 无设计/实现不一致 | pass |
| testing | `testing.md` + `testplan.yaml` | 测试入口、步骤与文档一致；新增回归用例已记录并在 unified entrypoint 运行通过 | 无文档/实现不一致 | pass |

## Result Summary

- Overall result: accepted
- Outcome: 首轮发现的 2 个实现缺陷与 1 个测试缺口已通过返回实现/测试流程修复并回归通过；本轮独立复核未发现残留缺陷
- Blocking issues: 无
- Next action: 完成生命周期收尾（accepted completion 与任务索引移除）

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 独立缺陷搜索覆盖全部十个类别；四个 change_id 需求均通过，测试（30 单测 + 7 集成 + DV 构建）与契约桩证据充分，未遗留阻塞问题；首轮 return 修复链与收据重建均经官方检查器验证。
