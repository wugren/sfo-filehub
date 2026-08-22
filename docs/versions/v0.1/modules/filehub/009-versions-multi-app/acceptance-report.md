---
task_manifest: task.yaml
status: approved
---

# 版本多应用（versions multi-app）验收报告

## Findings

| ID | Severity | Owning Stage | Correctness Category | Evidence | Problem | Blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-1 | none | none | test-adequacy | 首轮 review 发现 `validate_app`/`create_version` 空值分支与 422/单 app 缺省下载等参数域用例缺失；已返回 testing 补齐：`server/tests/unit/versions.rs` 新增输入校验用例、`server/tests/api_integration.rs` 新增边界用例，统一入口复跑 14 步全绿（run artifact 20260821T071924Z） | 缺口已闭合，无残留缺陷 | no |
| F-2 | none | none | state-and-data-integrity | `publish_app` 的 `version_apps.file_id` 全局 UNIQUE；服务层若复用同一 `FileRecord` 发布到第二个 app，惟一处突映射为 Db(500) | 该路径仅存在于服务层误用：`FileStore::ingest` 每次生成唯一 Uuid，HTTP/CLI/Web 与现有测试均不会复用 file_id；不构成可触达缺陷，不改服务契约 | no |
| F-3 | none | none | test-adequacy | `admin-web` 未新增 ProjectDetailPage 组件级渲染断言（锁定禁用/上传交互） | UI 情境已由 build + integration stub 覆盖后端契约，testing.md Unit Tests 行按 manual 记录理由，属于记录的残余情境而非缺陷 | no |

## Object and Scope

- Task manifest: task.yaml
- Review date: 2026-08-21
- In-scope implementation: `server/migrations/0006_versions.sql`、`server/src/model/record.rs`、`server/src/versions/{mod,service,http}.rs`、`server/src/storage/store.rs`、`cli/src/`、`admin-web/src/`、`docs/api/v1-contract.md`、`docs/modules/filehub.md`
- Review mode: independent falsification（独立证伪）；conclusion selected after findings and category review

## Requirement Coverage

| change_id | Requirement or Boundary | Source | Implementation Evidence | Finding | Status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-versions-multi-app-model | 版本显式创建（重复 409）、版本内多个具名 app 发布/更新/删除、不可逆锁定、单版本查询返回全部 app、不做存量兼容 | proposal.md P-001；design/versions.md | `versions/service.rs`（create_version/publish_app/delete_app/lock/list/get/referenced_file_ids）、`versions/mod.rs` trait、`record.rs` `AppRecord/VersionRecord/VersionPublish`、`0006_versions.sql`（`version_apps` + `locked_at`，无回填）；unit/dv 断言 | 无需求缺陷（F-2 为不可达残余场景） | pass |
| fh-versions-multi-app-api | v1 契约：创建/锁定端点、`PUT/DELETE .../apps/{app}`、`apps[]` 聚合响应、`download?app=` 缺省单 app 兼容/多 app 422 | proposal.md P-002；design/api.md | `versions/http.rs`（CreateVersionRequest/DownloadQuery/每条新路由）、`docs/api/v1-contract.md`；集成断言 201/200/404/409/422 | 无需求缺陷 | pass |
| fh-cli-multi-app | `new-version`/`lock-version`/`delete-app`、`publish/download --app`、versions 输出 app 与锁定状态 | proposal.md P-003；design/cli.md | `cli/src/cli/{args,mod,new_version_handler,lock_version_handler,delete_app_handler,publish_handler,download_handler,versions_handler}.rs`、`apiclient/{mod,contract}.rs`；dv/integration mock 断言 | 无需求缺陷 | pass |
| fh-web-multi-app | 版本详情：创建版本、按 app 上传/更新/删除、锁定与锁定标记、按 app 下载 | proposal.md P-004；design/web.md | `admin-web/src/pages/ProjectDetailPage.tsx`、`api/{client,contract}.ts`、`components/{icons,ui}.tsx`、`styles.css`、`i18n/messages.ts`；unit/integration/build 断言 | 无需求缺陷（F-3 为已记录的 manual 情境） | pass |

## Independent Defect Discovery

| Category | Applicable Scope | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|------------------|--------------------|-------------------|----------------------------------|--------|
| requirement-and-behavior | 六轮澄清需求 + 锁定/下载/不做存量兼容边界 | proposal.md、design/*.md、四个 change_id 实现与测试逐项对照 | 搜索缺失行为、非目标外行为与反例（如隐式建版本残留、解锁端点、存量回填） | 隐式建版本/解锁/回填均不存在；契约文档 v1 破缺说明齐备 | pass |
| logic-and-control-flow | 版本生命周期分支、latest 排序、下载选 app、事务 UPSERT | versions/service.rs 全量、http.rs 路由、aggregate_all/get 排序 | 挑战重复 409、锁定后写拒绝、空版本下载、多 app 缺省 422、`?app` 缺失/非法、latest 空版本、更新 created=true/false | 断言覆盖：重复创建 409、锁定写 409、422/404、latest、created 标志均通过 | pass |
| boundary-and-input | 空 version、app 字符白名单、query 缺省/非法、编码路径参数 | `validate_app`、`DownloadQuery`、`CreateVersionRequest`、http 参数解析 | 注入空/空白 version、含空格与 `/` 的 app、`?app=wrong`、多 app 无 query | F-1 暴露的参数域缺口已补齐（unit + integration 新增用例全绿） | pass |
| state-and-data-integrity | versions/version_apps 状态、锁定终态、引用集 | service.rs 事务与 UPSERT、store.rs discard/gc、referenced_file_ids | 挑战更新后旧文件回收、删除后引用移除、锁定与写入并发顺序、file_id 唯一 | 引用集断言、锁定终态断言通过；F-2 记录不可达的边缘语义 | pass |
| error-handling-and-recovery | publish 失败 discard、404/409/422 映射、CLI 退出码 | versions/http.rs api_err、cli classify_status/from、mock 错误路由 | 注入版本缺失/锁定/非法输入/权限拒绝与流损坏 | 404/409/422/403 与 CLI 退出码 2/3/4/5/7 均有断言 | pass |
| resource-lifetime-and-cleanup | 临时归档、SQLite 事务、下载流、清理守卫 | cli CleanupGuard、service.rs tx、store.rs 写入/删除、web timer | 检查成功/失败路径的临时文件与句柄释放 | CleanupGuard/disarm、tx commit/rollback、超时 timer 清理路径核对无泄漏 | pass |
| concurrency-and-ordering | 锁定与写操作原子性、重复创建冲突、事务隔离 | publish_app 事务内锁定检查、lock COALESCE、UNIQUE 约束 | 挑战并发 publish/delete 与锁定竞态、重复创建竞态 | 串行语义 + UNIQUE 兜底；并发注入未做（testing.md concurrency 行 manual 已记录） | pass |
| interface-and-compatibility | v1 契约破缺、三交付面同步、consumer 迁移 | docs/api/v1-contract.md、consumer-closure-check、negative fixture、CLI/Web 调用链 | 挑战旧符号残留、旧 POST multipart 语义残留、参数形状不一致 | removed-symbol-scan/external-negative/compile-closure 全过；契约文档与实现一致 | pass |
| security-and-capacity | 新端点鉴权、app 白名单、上传上限、凭据注入 | permissions checker、validate_app、files max_archive_bytes、CLI 凭据路径 | 挑战只读成员/token 新端点 403、Anonymous private 401、非法 app 注入路径、超限归档 | 权限拒绝断言、422 注入断言通过；无新暴露面 | pass |
| test-adequacy | 正常/边界/负向/错误/生命周期/兼容/跨模块与六类设计元素 | testing.md、testplan.yaml、run artifact（20260821T071924Z 全部 14 步 0 失败）、各测试断言 | 判断参数域、状态迁移、失败路径、并发等能否被现有测试暴露 | F-1/F-3 已按缺口修复或明确 manual 理由；统一入口 artifact 全绿 | pass |

## Document Consistency

| Document | Source | Implementation Consistency | Finding | Status |
|----------|--------|----------------------------|---------|--------|
| design | `design.md` + `design/{versions,api,cli,web}.md` | 实现按 design 形状落地（trait 方法集、端点表、DTO、命令面、页面交互） | PublishOutcome 以 `VersionPublish{created,record}` 返回，为 design api 200/201 语义的机械实现细节，与 design 描述（返回 record）不冲突 | pass |
| testing | `testing.md` + `testplan.yaml` | 测试命令、change_id 映射、case/design element 覆盖与 run artifact 一致 | F-1 修复后 testing.md 已补充新增行；无其他不一致 | pass |

## Result Summary

- Overall result: accepted
- Outcome: 版本显式创建/重复 409、多 app 发布/更新/删除、单版本全量 app 查询、不可逆锁定、按 app 下载在 server/cli/web 三面交付完成；server 单元 19 例/dv 1 例/集成 3 例，cli 21 例，web 44 例（37 unit + 7 integration），统一入口 14 步契约/测试全部通过
- Blocking issues: 无
- Next action: 完成生命周期收尾并从 unfinished index 移除；部署侧旧库按新 schema 重建（用户在提案 Q7 明确不做存量兼容）

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 独立缺陷发现覆盖全部类别；F-1 测试缺口已返回 testing 补齐并通过统一入口复跑，F-2 为不可达残余场景，F-3 为已记录 manual 理由的 UI 情境；四个 change_id 均有需求覆盖与实现证据，design/testing 文档一致，结论 selected after findings and category review。
