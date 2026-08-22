# filehub 服务后台（filehub-server）验收报告

## Findings
| ID | Severity | Owning Stage | Correctness Category | Evidence | Problem | Blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-1 | none | none | overall | proposal/design/implementation/tests 与存活运行证据已逐项核对 | 未发现缺陷；interface 差异已作为「契约落盘说明」记录（sfo-http Actix 后端不支持 PATCH，`docs/api/v1-contract.md` 定义 POST= PATCH 语义并作为 002/003 唯一消费源） | no |

## Object and Scope
- Task manifest: task.yaml
- Review date: 2026-08-19
- In-scope implementation: `server/` crate（account/permissions/tokens/storage/versions/projects/http/contract/model）、`server/migrations/`、`docs/api/v1-contract.md`、`server/tests/`（unit/dv/integration）
- Review mode: independent falsification（独立证伪）；conclusion selected after findings and category review

## Requirement Coverage
| change_id | Requirement or Boundary | Source | Implementation Evidence | Finding | Status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-server-account | 配置初始化、sfo-account 直接导出登录/session/refresh；不建 SessionService | proposal.md P-01 / design/account.md | `server/src/account/{mod,store,authn,http}.rs`；`tests/unit/account.rs`、`tests/api_integration.rs` 登录/会话正反例 | 无需求缺陷 | pass |
| fh-server-permissions | 统一权限判定与访问矩阵、协作者管理 | proposal.md P-02 / design/permissions.md | `server/src/permissions/{checker,http,model}.rs`；`tests/unit/permissions.rs` 矩阵断言 | 无需求缺陷 | pass |
| fh-server-tokens | JWT token 创建/列表/修改/轮换/撤销、私钥即弃、exp 仅 JWT 承载、凭据类型区分 | proposal.md P-03 / design/tokens.md | `server/src/tokens/{service,http,model}.rs`；`tests/unit/tokens.rs`、`tests/api_integration.rs`；DB 无过期字段 | 无需求缺陷 | pass |
| fh-server-files | .tar.gz 原子入库/下载/SHA/路径防穿越/孤儿回收 | proposal.md P-04 / design/files.md | `server/src/storage/{store,integrity,http}.rs`；`tests/unit/storage.rs` | 无需求缺陷 | pass |
| fh-server-versions | 版本不可覆盖、latest、原子发布协调 | proposal.md P-05 / design/versions.md | `server/src/versions/{service,http,model}.rs`；`tests/unit/versions.rs`、集成 409/下载 SHA | 无需求缺陷 | pass |
| fh-server-projects | 项目 CRUD、public/private、owner 隐式 admin | proposal.md P-06 / design/projects.md | `server/src/projects/{service,http,model}.rs`；`tests/unit/projects.rs` | 无需求缺陷 | pass |
| fh-server-http | /api/v1 路由/DTO/错误映射、sfo-http 装配、契约文档 | proposal.md P-07 / design/http.md | `server/src/{http,contract}/`、`docs/api/v1-contract.md`；`tests/api_integration.rs` | 无需求缺陷；PATCH/POST 映射见 F-1 | pass |

## Independent Defect Discovery
| Category | Applicable Scope | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|------------------|--------------------|-------------------|----------------------------------|--------|
| requirement-and-behavior | 提案 P-01~P-07 全部行为边界 | proposal/design/实现/测试/契约逐项对照 | 搜索缺失/多余行为与反例 | 未发现行为缺失；PATCH/POST 契约落盘说明记录于本表 + 契约文档 | pass |
| logic-and-control-flow | can_access、token resolve、发布事务、multipart | permissions/checker.rs、tokens/service.rs、versions/service.rs、versions/http.rs | 挑战 can_access 三分支、token resolve 签名/exp 校验、发布 INSERT 冲突、latest 排序与 multipart 解析终止 | 矩阵与 409/latest/轮换逻辑经正反例断言 | pass |
| boundary-and-input | 参数域（空名、非法 scope、过期、非 gzip、超限、path param） | model/*、storage/integrity.rs、contract/mod.rs | 注入空 token 名、非法 scope、超 1 年 expires、非 gzip 上传、超 max_archive_bytes、缺失 multipart fields、非法 path param | 空/非法/超限/SHA 不一致均有拒绝断言 | pass |
| state-and-data-integrity | 版本唯一、token 旧 JWT 失效、项目删除后引用、孤儿回收 | versions/service.rs、tokens/service.rs、storage/store.rs | 挑战重复发布 409、rotate 后旧 JWT 401、revoke 后 resolve 失败、项目删除后引用/孤儿状态、启动 gc | 409、rotate 旧 JWT 401、gc 断言通过 | pass |
| error-handling-and-recovery | 发布失败回滚、discard、启动 GC | versions/http.rs、files discard/gc；集成 409 | 注入 publish 冲突后 discard 回滚、ingest 失败清理临时文件、409 后无孤儿残留、启动 gc 清理删除项目文件 | 发布失败 discard、启动 gc 清理验证 | pass |
| resource-lifetime-and-cleanup | 临时文件、SQLite 事务、下载流 | storage/store.rs、tokens/service.rs | 检查 ingest 临时文件删除、discard referenced 拒绝、tokens 事务提交、DownloadStream 生命周期与进程结束后 gc | 无孤儿残留；测试与代码检查未发现泄漏 | pass |
| concurrency-and-ordering | rotate/revoke 更新顺序、公钥替换 | tokens/service.rs、checker.rs | 构造 rotate 后旧 JWT 验签失败、revoke 后 resolve 失败、公钥替换与 claims jti 一致性反例 | rotate 后旧 JWT 验签失败（单元/集成断言） | pass |
| interface-and-compatibility | v1 契约、consumer 对齐、凭据类型区分 | docs/api/v1-contract.md、http/*、集成 HTTP 测试 | 挑战 token 列表无过期字段、TokenIssued 过期一次性返回、凭据类型区分（session 当 token/token 当 session 均 401/403）、POST 承载 PATCH 语义 | PATCH→POST 为 契约落盘说明；凭据类型区分经 401/403 断言 | pass |
| security-and-capacity | 认证/授权、私钥不落库、路径穿越、上限 | tokens/service.rs、permissions/checker.rs、storage/store.rs | 挑战 read-only token 写 403、token<=用户二次限制、私钥不落库检查、safe_path 越界拒绝、max_archive_bytes 放大拒绝 | token scope 二次限制、read-only 写 403、私钥不落库、max_archive_bytes 均验证 | pass |
| test-adequacy | unit/dv/integration 真实性 | 三份测试文件、testplan、run artifact | 检查 17 单元/2 DV/1 集成是否覆盖正常/边界/负向/错误/生命周期/跨模块并可复现 401/403/404/409/422 | 17 单元 + 2 DV + 1 集成全通过；覆盖正常/边界/负向/错误/生命周期/跨模块 | pass |

## Document Consistency
| Document | Source | Implementation Consistency | Finding | Status |
|----------|--------|----------------------------|---------|--------|
| design | `design.md` + `design/`（含 model.md） | 实现按 design 形状落地；PATCH/POST 契约落盘说明已在 `docs/api/v1-contract.md` 对齐 | 无其他不一致 | pass |
| testing | `testing.md` + `testplan.yaml` | 测试命令、change_id 映射与 run artifact 一致 | 无其他不一致 | pass |

## Result Summary
- Overall result: accepted
- Outcome: filehub-server 七子模块、SQLite schema、token/session 双凭据认证授权、.tar.gz 发布下载与 v1 契约交付完成；unit/dv/integration 及统一入口全部通过
- Blocking issues: 无
- Next action: 完成任务生命周期收尾并从 unfinished index 移除；002-web/003-cli 按 `docs/api/v1-contract.md` 消费（更新语义端点使用 POST）

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 独立证伪检查未发现阻断缺陷；唯一说明项（sfo-http Actix 后端 PATCH 不支持，wire 以 POST 对等提供）已记录于契约与测试回归说明，作为 002/003 唯一消费源的 `docs/api/v1-contract.md` 已同步，验收予以通过。
