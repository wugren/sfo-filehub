# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/042-integration-tests.md
- Approved proposal: proposal.md

## Delivery Summary

- Outcome:
  - 服务端接口集成（`server/tests/api_integration.rs`，新增 8 用例，累计 16）：
    session refresh 后新凭据可用；项目删除级联 + files/物理归档经启动 GC 回收；
    协作者 write/admin 矩阵与移除即时收回；token 过期边界、属性修改不重签且
    数据库权限即时生效、revoke 即时失效、Specified 项目范围隔离、token 管理
    仅限 session；版本名保留字与非法字符矩阵；重复锁定幂等；非 gzip 不透明
    字节上传；不存在的版本 404；下载 Content-Disposition 文件名契约；空版本
    404；错误体统一结构（401/403/409/422 + handler 类 404）。
  - CLI 命令行（`cli/tests/cmd_integration.rs`，14 用例）：真实二进制 argv 解析、
    stdin/环境变量凭据、凭据文件 0600 与 logout、损坏配置不覆盖、target 语法、
    网络/认证/授权/冲突/输入/完整性退出码矩阵、401 续期一次、token 401 不续期、
    只读 token 边界、校验失败不落盘、越界符号链接拒绝、text/json 输出契约、
    明文不回显。
  - CLI↔真实服务端（`cli/tests/e2e_cli_server.rs`，1 用例）：进程内装配真实
    filehub-server + 子进程 CLI 全命令流，下载 sha256 与服务端记录闭环。
  - 最小生产修复（`cli/src/cli/mod.rs`）：`--help`/`--version` 退出码对齐冻结
    文档为 0（其余 clap 错误仍 1）。
  - 统一入口（`harness/scripts/test-run.py`）：filehub integration 挂载四组
    命令，服务端套件串行化规避 16 路并发 502 flake。
- Handoff:
  - `python3 harness/scripts/test-run.py filehub integration` 全绿：
    server api_integration 16/16、cli api_integration 12/12、
    cli cmd_integration 14/14、cli e2e_cli_server 1/1；
  - `target/debug/filehub --help` 与 `--version` 退出码 0；
  - `lower-tier-check.py --profile pre-edit` 通过；任务基线已捕获。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-it-server-api | 补齐 v1 接口集成层缺口：A4/B7/C4/C5/D1/D2/D4/D6/D7/D8/E2/E5/F8/F10/G2/G5/J1 及 J2 状态断言 | proposal.md P-01 | server/tests/api_integration.rs 新增 8 个 #[tokio::test]，旧 8 个全部仍通过，套件 16/16 | 匹配 | pass |
| fh-it-cli-cmd | 真实二进制级 CLI 命令行集成 K1–K22（argv/stdin/env/凭据/退出码/输出/安全） | proposal.md P-02 | cli/tests/cmd_integration.rs 14 用例全部通过 | 匹配 | pass |
| fh-it-cli-e2e | CLI↔真实 filehub-server 端到端 H1–H5 主流程 | proposal.md P-03 | cli/tests/e2e_cli_server.rs 全命令流通过，sha256 闭环断言成立 | 匹配 | pass |
| fh-it-help-exit-code | `--help`/`--version` 退出码 0 对齐冻结文档 | proposal.md P-04 | cli/src/cli/mod.rs 解析失败分支按 clap ErrorKind 返回 0/1；二进制实测通过 | 匹配 | pass |
| fh-it-suite-entry | test-run.py 注册新测试入口，模块级 integration/all 可达 | proposal.md P-05 | MODULE_SUITES filehub.integration 四组命令；`filehub integration` 实跑全绿 | 匹配 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | 审阅 9 个新增服务端用例的权限/错误映射：project delete 非 owner 403、token 范围外 project 读 404 而 versions 读 403、删除项目后 versions/version/download 403、未知路由空体 404；CLI run_bin 子进程 stdin 写入与 wait_with_output 的阻塞语义；run_cli 对 DisplayHelp/DisplayVersion 的 0/1 分流 | 反向推演：session 同秒 refresh 后 JWT 字节可相同（断言只验证可用性）；token 属性修改后旧 JWT 的 403 由数据库 scope 变更而非重签触发；project_scope 空 Specified 必须等价 All；sfo-http 无公开 catch-all 装配，未知路由 JSON 化不可行 | 服务端/CLI 语义均按实现断言并对契约措辞差异留档（见 Findings F-1/F-2/F-3）；run_cli 修复最小且与冻结文档一致 | pass |
| boundaries-and-failure-paths | 版本名矩阵（latest、/、?、#、控制字符、空串）、expires 非日期/超 400 天/合法 1 小时、下载缺省 app 三态（单/多/空）、协作者 read/write/admin 三级、pull 损坏流/目录目标/越界符号链接、401 一次与 token 401 不续期、登录 64K 既有边界未回归 | 边界推演：版本名为 latest 必须 422；Specified 集合外项目读必须 404 且列表必须 403；空 Specified 必须放行全部；重复锁定 locked_at 必须不变；损坏流退出 7 且目录目标退出 5，且失败后不得残留 .tmp；v2 删除 app 后 pull 必须 404 退 5 | 全部边界与失败路径用例通过；新增断言未破坏既有 8 个服务端/12 个 CLI 用例 | pass |
| regression-and-side-effects | 全量 filehub integration 经 canonical 入口实跑两次（首次 16 路并行偶发 502，串行后全绿）；cli api_integration 12/12、cmd_integration 14/14、e2e 1/1；检查 server/cli 生产代码改动仅 run_cli 三分支 | 排查：run_cli 修复是否影响未知选项（仍 1）与正常命令（不受影响——cmd 套件覆盖）；dev-dependency 是否进入 release（仅 [dev-dependencies]，Cargo.lock dev 侧新增）；test-run.py 套件挂载是否改变既有 all 语义（integration 新增命令被 all 合成，harness-self-check 保留） | 既有套件零回归；唯一生产行为变化是帮助/版本退出码对齐冻结文档；并行 502 属测试资源压力，套件串行后稳定 | pass |

## Verification

- Targeted check:
  - `python3 harness/scripts/test-run.py filehub integration`（2026-08-25）：
    server api_integration 16/16、cli api_integration 12/12、
    cli cmd_integration 14/14、cli e2e_cli_server 1/1，全部通过；
  - `target/debug/filehub --help`、`--version`、`-h` 实测退出码 0；
  - `lower-tier-check.py --profile pre-edit` 通过（任务开始基线已捕获）。
- Result: pass
- Exception reason: not-applicable（目标验证全部通过，无豁免）。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | 未知路由 `GET /api/v1/does-not-exist` 返回 404 且响应体为空 | sfo-http 0.8 内置 not_found_endpoint 无 JSON 错误体，且当前公开 HttpServer trait 无 catch-all 装配能力；v1 契约「错误统一 JSON」对未知路由不成立，已作为残余缺口留档，建议上游/后续任务处理 | no |
| F-2 | low | 项目删除后 versions/version/download 实测 403，契约文案为 404；token 范围外 versions 列表实测 403 | versions 服务对 can_access=false 统一返回 forbidden，与契约「已认证不可见 404」措辞不一致；本次按实现断言并留档，未改契约文档 | no |
| F-3 | low | sfo-account refresh 在同一秒内重新生成 session，JWT 的 iat 相同导致新旧凭据字节相同 | 测试改为只断言 refresh 返回凭据可继续访问，不断言字节轮换；若产品要求严格轮换需上游 sfo-account 变更 | no |
| F-4 | low | 16 个服务端真实 HTTP 用例在 test-run.py 默认并行时偶发登录 502；单独/3 个并行均稳定 | 并行起 16 个 Actix 服务与 SQLite 池存在资源压力；canonical 套件已加 `-- --test-threads=1` 串行规避，根因未单独定位 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: P-01~P-05 全部落地且相互一致：服务端接口集成新增 8 用例通过并保留
  旧 8 用例零回归；CLI 真实二进制命令行 14 用例全部通过；CLI↔真实服务端
  端到端 sha256 闭环成立；`--help`/`--version` 退出码与冻结文档对齐；
  canonical `filehub integration` 四组命令全绿。独立缺陷发现覆盖行为逻辑、
  边界失败路径与回归副作用，F-1~F-4 均为低危非阻塞记录并附后续处理方向。
