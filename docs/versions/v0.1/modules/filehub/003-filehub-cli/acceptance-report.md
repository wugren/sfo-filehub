# filehub 发布客户端（filehub-cli）验收报告

## Findings
| ID | Severity | Owning Stage | Correctness Category | Evidence | Problem | Blocking |
|----|----------|--------------|----------------------|----------|---------|----------|
| F-1 | none | none | overall | proposal/design/实现/测试逐项核对 + 真实 filehub-server 联调（发布/版本/下载字节一致、409 与未登录反例退出码正确） | 未发现缺陷；联调与测试共同覆盖全部四个 change_id 的正反边界 | no |

## Object and Scope
- Task manifest: task.yaml
- Review date: 2026-08-20
- In-scope implementation: `cli/` crate（`cli/src/{cli,apiclient,credential_store,archive}/`、`main.rs`、`lib.rs`、`Cargo.toml`、`Cargo.lock`、`README.md`）与 `cli/tests/`（unit/dv/integration）
- Review mode: independent falsification（独立证伪）；结论在 Findings 与分类核查完成后选定

## Requirement Coverage
| change_id | Requirement or Boundary | Source | Implementation Evidence | Finding | Status |
|-----------|-------------------------|--------|-------------------------|---------|--------|
| fh-cli-login | 密码/token 登录、logout、本地最小权限凭据、token > session 复用、401 续期一次、明文不入参数/日志 | proposal.md P-01 / design/credential-store.md + design/cli.md | `cli/src/cli/login_handler.rs`、`cli/src/credential_store/{mod,model,security}.rs`；真实服务端登录成功、logout 后 401 退出码 2；mock 集成断言错误密码/无效 token 不写凭据、续期落盘 s2 | 无需求缺陷 | pass |
| fh-cli-publish | `<文件或目录> <project>:<version>` 统一 `.tar.gz`、409 不覆盖、打包安全、发布前解析项目 | proposal.md P-02 / design/archive.md + design/apiclient.md | `cli/src/cli/publish_handler.rs`、`cli/src/archive/{mod,safe_tar}.rs`；真实服务端发布 201 成功且下载字节一致、重复发布真实 409 退出码 4、越界符号链接打包拒绝 | 无需求缺陷 | pass |
| fh-cli-download | `download <project>[:<version>] -o <目录>`、latest 语义、SHA-256 校验后落盘、文件名净化 | proposal.md P-03 / design/archive.md + design/apiclient.md | `cli/src/cli/download_handler.rs`、`archive::finalize_download`；真实服务端下载 `e2e-project-v1.0.0.tar.gz`，sha256 与服务端一致、归档内文件与原文件一致；corrupt 流 exit 7 | 无需求缺陷 | pass |
| fh-cli-versions | `versions <project> -o <路径>` 文本/JSON、stdout/文件输出、路径安全 | proposal.md P-04 / design/cli.md + design/apiclient.md | `cli/src/cli/versions_handler.rs`；真实服务端 JSON 输出与服务端 VersionRecord 字段一致；集成断言 text/json 输出 | 无需求缺陷 | pass |

## Independent Defect Discovery
| Category | Applicable Scope | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|------------------|--------------------|-------------------|----------------------------------|--------|
| requirement-and-behavior | 提案 P-01~P-04 全部行为边界 | proposal/design/实现/契约/测试/真实联调逐项对照 | 搜索缺失/多余行为与反例（409、403、401 续期、latest、哈希、logout） | 四个 change_id 的正反边界均被 mock 集成与真实服务端联调覆盖，未发现行为缺失 | pass |
| logic-and-control-flow | run_auth 续期、下载重试、凭据互斥、打包遍历 | `cli/src/apiclient/mod.rs`、`download_handler.rs`、`archive/safe_tar.rs` | 挑战 401 只续期一次、流开始后不部分重试、token 401 不续期、符号链接包含判定、登录模式互斥 | 续期一次/不重试/互斥逻辑经测试断言；打包遍历对越界链接拒绝、树内链接收敛为相对 target | pass |
| boundary-and-input | 参数域、stdin/环境变量、目标目录、文件名净化 | `cli/src/cli/*`、`credential_store/mod.rs`、`archive/mod.rs` | 注入互斥选项、非终端无模式、空密码、超长/保留名、损坏 TOML、只读 token | 互斥/非终端/空输入/保留名/损坏配置均有正反断言；真实服务端 422 缺字段路径经 mock 覆盖 | pass |
| state-and-data-integrity | 凭据状态机、lodout、原子写 | `credential_store/{mod,security}.rs`、dv persistence 用例 | 挑战 login 覆盖互斥、logout 清空、原子写不产生半截配置、损坏不覆盖 | save_session/save_token 互斥与 logout 清空断言；损坏配置 exit 8 且文件原样保留 | pass |
| error-handling-and-recovery | 登录失败、上传失败、下载失败、续期失败 | `login_handler.rs`、`publish_handler.rs`、`download_handler.rs`、integration tests | 注入坏密码、409、corrupt 流、401、无凭据 | 坏密码/无效 token 不写凭据；409 exit 4；corrupt exit 7；未登录 exit 2；临时文件在失败路径清理 | pass |
| resource-lifetime-and-cleanup | 临时归档/下载文件、连接与 mock 进程 | `publish_handler.rs` CleanupGuard、`download_handler.rs`、`apiclient/mod.rs` | 检查发布失败/成功清理、下载失败清理、流文件 sync、mock 服务生命周期 | 发布 CleanupGuard 成功/失败都清理；下载失败删除 tmp；release 二进制与真实服务端联调无残留临时文件 | pass |
| concurrency-and-ordering | 401 续期、多服务器凭据隔离 | `apiclient/mod.rs` run_auth、`AuthClient.prepare`、integration refresh tests | 构造续期一次后重试成功、续期失败不重试、token 不续期 | run_auth 对 session 续期一次并落盘 s2；下载端点 401 一次后成功；token 401 直接失败 | pass |
| interface-and-compatibility | v1 契约形状、错误体、multipart、sfo-http 包装 | `docs/api/v1-contract.md`、`cli/tests/common/mod.rs` mock、`cli/src/apiclient/contract.rs` | 对照真实服务端 `POST /account/login`、`POST versions` multipart、`GET download` 流 | 真实联调证明契约完全对齐（登录包装、multipart 字段、VersionRecord JSON、下载 .tar.gz 流）；PATCH/POST 说明仅为服务端既有落盘说明 | pass |
| security-and-capacity | 凭据防泄漏、最小权限、路径穿越、归档越界 | `credential_store/security.rs`、`archive/*`、login 输出路径 | 挑战凭据明文入参数/日志、文件权限、净化名穿越、越界符号链接 | 无明文参数选项；日志不含凭据；0600 原子写；净化名无分隔符；越界链接拒绝；真实联调日志无凭据明文 | pass |
| test-adequacy | 测试真实性与可复现性（unit/dv/integration 覆盖） | `cli/src/**` 单测、`cli/tests/{dv_tests,api_integration}.rs`、testplan、run artifact | 判断 unit(6)+dv(4)+integration(10) 是否覆盖七类用例并可复现 401/403/404/409/422、续期、latest、哈希 | 20 项测试覆盖正常/边界/负向/错误/兼容/生命周期/跨模块七类；`.harness/test-results/test-runs/*003-filehub-cli-all.json` 记录全部通过；真实服务端联调另行覆盖闭环 | pass |

## Document Consistency
| Document | Source | Implementation Consistency | Finding | Status |
|----------|--------|----------------------------|---------|--------|
| design | `design.md`, `design/apiclient.md`, `design/archive.md`, `design/cli.md`, `design/credential-store.md` | 实现遵循设计模块边界、接口签名与实现顺序 | cli/apiclient/credential_store/archive 子模块与文件级序列对应；Scope Paths 与实现路径一致 | pass |
| testing | `testing.md`, `testplan.yaml` | 实现行为与测试文档一致 | 表格、testplan step 与真实测试一一对应；测试均经统一入口运行 | pass |

## Result Summary
- Overall result: accepted
- Outcome: `cli/` crate 交付全部四条命令面；真实 filehub-server 联调通过（登录/发布/版本/下载/409/未登录退出码）；20 项自动化测试通过并记录 run artifact
- Blocking issues: 无
- Next action: 完成验收登记并从 unfinished task index 移除 003

## Conclusion
- Accepted / rejected / needs changes: accepted
- Reason: 独立证伪核查未发现缺陷：需求覆盖逐项通过，全部 10 个缺陷发现类别有具体证据，测试覆盖七类用例且真实服务端联调（含 409 与未登录反例）佐证契约一致性；实现遵循已批准提案与设计。
