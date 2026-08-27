# Lightweight Acceptance Report

## Object and Scope
- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/055-server-yaml-config.md

## Delivery Summary

- Outcome: 服务端配置的默认文件名与解析入口已从 JSON 切换到 YAML；仓库根配置
  与 `server/config.example.*` 已替换为 `.yaml`，每个字段都有就近中文说明，
  CORS 列表、`max_age`、credentials、登录限流和 `db_path` 等具有可用默认值的
  字段全部保持注释态。Docker 入口生成安全转义的块状 YAML，README、Docker
  README 与模块依赖说明已同步。
- Handoff: 产品只承诺 YAML 配置，不实现 JSON 专用兼容或语法拒绝；`FH_CONFIG`
  的显式路径优先级、HTTP API JSON、账号/端口/目录/限流字段语义均未改变。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-server-yaml-loader | 默认读取 `filehub-server.yaml`，以维护中的 YAML 解析器反序列化 `ServerConfig`；不做双格式探测或 JSON 专用兼容，HTTP JSON 不变 | proposal.md P-001 | `server/src/main.rs` 改用 `serde_saphyr::from_str` 并只返回路径/行列的脱敏错误；`server/Cargo.toml` 仅启用 `serde-saphyr` 的 `deserialize` feature，Cargo.lock 固定 1.1.0；locked check 通过 | 匹配 | pass |
| fh-server-yaml-examples | 两份当前配置改为逐字段中文注释的 YAML，有可用默认值的键全部注释，必填/可登录字段保持启用 | proposal.md P-002 | `server/config.example.yaml` 与 `filehub-server.yaml` 内容一致；旧 JSON 文件删除；配置测试读取真实示例，断言九个默认键处于注释态且解析值与代码默认一致，必填 port 缺失时失败 | 匹配 | pass |
| fh-server-yaml-deployment | Docker 入口和当前文档统一 `.yaml`，环境变量值安全编码，容器参数/拓扑不变 | proposal.md P-003 | `docker/entrypoint.sh` 默认路径改为 `.yaml`，jq `@json` 生成 YAML 安全字符串标量并移除已废弃 role；特殊字符探针生成真实块状 YAML且被 server 解析；README/Docker README/docs/modules 当前引用更新，无活动 `.json` 配置引用 | 匹配 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `ServerConfig`/各子配置 serde 默认、两份 YAML、main 装载顺序、Cargo feature/lock、Docker jq 输出 | 逐项把示例注释值与 `default_max_age=3600`、登录限流 30/60、空 CORS 列表、`support_credentials=false`、`db_path=filehub.db` 对照；确认必填地址/端口/用户/会话密钥/文件目录/上限仍启用；只启用反序列化 feature 后再次 locked 编译与测试 | 字段层级和默认值未漂移；示例可解析且配置校验通过；无多余 YAML 序列化依赖 | pass |
| boundaries-and-failure-paths | YAML 缺字段/非法类型、解析错误渲染、Docker 用户名/密码/密钥标量、YAML/JSON 边界 | 删除 `server.port` 必须解析失败；用含测试密钥的非法 YAML 启动，断言仅输出文件路径与行列且不回显密钥；从实际 entrypoint 抽取 jq 命令，输入冒号、井号、引号、反斜杠和换行，输出不是 JSON 对象且可被 server 解析至会话密钥校验 | 缺字段 fail closed、错误脱敏、Docker 标量无结构注入；JSON 子集偶然可解析仅为非承诺行为 | pass |
| regression-and-side-effects | 055 pre-edit 基线、Cargo.lock 增量、main/Cargo/README/docker/config-test 重叠文件差异、全 server unit suite、活动配置引用扫描 | 对每个既有脏文件与基线逐项 diff；发现 Cargo 首次解析曾顺带刷新已有 windows-sys/getrandom 引用后已回退，并以 `--locked` 复验；扫描排除历史任务文档后无 `.json` 活动入口；Rust 格式检查只剩基线中已存在的两处测试折行差异 | 本任务增量未覆盖 054 等任务修改；最终锁文件只新增 deserialize 所需包；68 个 server unit tests 全通过 | pass |

## Verification

- Targeted check: `cargo check -p filehub-server --locked`；
  `cargo test -p filehub-server --test unit_tests --locked`（68 passed）；
  `cargo test -p filehub-server --test unit_tests unit::config -- --nocapture`
  （6 passed）；非法 YAML 敏感值不回显探针；Docker entrypoint 精确 jq 片段的
  特殊字符 YAML→实际 server parser 探针；`sh -n docker/entrypoint.sh`；
  两份 YAML `diff`；活动 `.json` 引用 `rg`；任务路径 `git diff --check`；
  基线逐文件差异与 Cargo.lock 最小增量核对
- Result: pass
- Exception reason: 本环境没有 Docker daemon 且未安装 shellcheck，未执行完整镜像启动与 shellcheck；现有 CI 已有真实容器启动冒烟，本地以 `sh -n` 和从实际入口抽取 jq 命令后交给真实 server 解析器的链路验证替代

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | `docker info` 不可用；入口 jq 片段的块状 YAML、特殊字符转义与 server 解析探针通过 | 本机无法执行完整 Docker 镜像启动；真实 entrypoint+nginx+server 组合仍由现有 CI smoke test 覆盖 | no |
| F-2 | low | YAML 1.2 允许 JSON 子集；实现没有扩展名/语法拒绝分支 | 旧 JSON 文本可能被当前底层解析器偶然接受，但不构成兼容承诺；文档、默认路径和仓库配置均只提供 YAML | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 三个 change_id 均按用户确认的 standard 范围交付；YAML 装载、逐字段
  注释/default 注释态、Docker 安全生成与当前文档同步均有定向证据，最终 locked
  编译和 68 个 server unit tests 通过；独立检查未发现阻塞缺陷，Docker 本机不可用
  与 JSON 子集行为均已作为非阻塞残余记录。
