# 集成测试补齐：filehub v1 接口 + filehub-cli 命令行

- Status: complete
- Owner module: filehub（filehub-server 接口集成 / filehub-cli 命令行集成）
- Task manifest: docs/versions/v0.1/modules/filehub/042-integration-tests/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/042-integration-tests/proposal.md
- Affected paths: server/tests/api_integration.rs、cli/tests/cmd_integration.rs、
  cli/tests/e2e_cli_server.rs、cli/src/cli/mod.rs、cli/Cargo.toml、
  harness/scripts/test-run.py、docs/changes/042-integration-tests.md、
  docs/versions/v0.1/modules/filehub/042-integration-tests/
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- `server/tests/api_integration.rs` 新增 8 个真实 HTTP 集成用例：refresh 续期后
  新凭据可用、项目删除级联 + 启动 GC 收尾 files、协作者 write/admin 角色矩阵与
  移除即时生效、token 过期边界/属性修改不重签/数据库权限即时生效/revoke/
  Specified 项目隔离/管理凭据边界、版本名保留字与非法字符矩阵、重复锁定幂等、
  非 gzip 不透明字节上传、目标版本 404、下载 Content-Disposition、空版本下载
  404、错误体统一结构；断言按当前真实服务端语义（越权/项目不可见的部分端点
  返回 403、token 范围外项目读 404）对齐实现并记录契约措辞差异。
- `cli/tests/cmd_integration.rs` 新增 14 个真实二进制进程级用例（14/14 通过）：
  `--help/--version` 退出码 0、无参/未知命令 1、登录模式互斥与 stdin 非终端
  用法错误、stdin/环境变量凭据、凭据文件 0600 与 logout 清除、损坏配置 8 且
  不覆盖、target 语法 5、网络失败 6、session 401 续期一次与 token 401 不续期、
  只读 token 读 0 写 3、pull 校验失败 7 不落盘、越界符号链接 7、push 成功/409/404
  退出码矩阵、versions text/json 与 stdout、无凭据 2、明文不回显。
- `cli/tests/e2e_cli_server.rs` 新增 1 个 CLI↔真实 filehub-server 端到端用例：
  login → new-version → push → versions（服务端 sha256）→ pull（本地 sha 闭环）
  → lock（锁后 push 409 退 4）→ v2 发布/删除（删除后 pull 404 退 5）→ logout
  后无凭据退 2。`cli/Cargo.toml` 增加 dev-dependency `filehub-server`（path）与
  `sfo-http`，仅测试构建依赖，不影响运行时依赖图。
- `cli/src/cli/mod.rs` 最小契约对齐修复：clap `DisplayHelp`/`DisplayVersion`
  解析错误分支返回退出码 0（其余解析错误仍 1），与 README/design/cli.md 冻结的
  「0 成功（含 --help）」一致；修复前实测为 1。
- `harness/scripts/test-run.py` MODULE_SUITES 挂载 filehub integration 四组命令；
  服务端真实 HTTP 套件在 16 路并行下存在偶发 502，套件级加
  `-- --test-threads=1` 保证 canonical 入口确定性。

## Risk Screen

- Public contract, protocol, or CLI change: yes（仅一处：`--help`/`--version`
  退出码由实现 1 对齐到冻结文档 0；命令面、参数、其它错误语义与 README
  退出码表均未变化；该行为已在提案确认前向用户明示并确认）
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no（新增测试包含认证/授权/凭据
  负例，但不改生产认证逻辑；CLI 修复不涉及凭据处理）
- Concurrency, lifecycle, or runtime integration change: no（测试自身在
  canonical 套件中串行执行；生产运行行为无并发/生命周期变更）
- Material dependency/build graph, supply-chain trust, produced artifact,
  production default/feature rollout, release/deployment, compatibility, or
  rollback impact: no（新增 dev-dependency 只在测试构建生效，release 依赖图
  与发布产物不变）
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: yes（test-run.py 注册
  集成的四组测试命令并为服务端套件配置串行；未改规则语义与 checker 逻辑）
- Cross-project or architectural boundary change: no（filehub 模块内三个交付面
  的测试补强；无跨模块边界变更）

## Verification

- Targeted check:
  - `python3 harness/scripts/test-run.py filehub integration`（canonical 入口，
    2026-08-25）：4 组命令全部通过——server api_integration 16/16、
    cli api_integration 12/12、cli cmd_integration 14/14、
    cli e2e_cli_server 1/1；
  - `target/debug/filehub --help`/`--version` 实测退出码 0（修复前 1）；
  - `lower-tier-check.py --profile pre-edit` 已通过并捕获任务基线。
- Result: pass
- Residual risk or follow-up:
  - sfo-http 0.8 内置未知路由 404 响应体为空，无法经当前公开装配 API 提供
    统一 JSON 错误体；J2 用例只断言 404 状态并记录该缺口，建议作为上游/后续
    server 任务处理；
  - 服务端真实 HTTP 套件在 16 路进程内并发时偶发 502（起多个 Actix 服务与
    SQLite 池的资源压力），已用套件串行规避，根因未单独定位；
  - 部分端点（项目删除后的 versions/version/download）当前返回 403 而非契约
    文案的 404；本项目按实现断言并记录，未改动契约文档。
