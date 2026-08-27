---
task_manifest: task.yaml
status: approved
---

# 服务端配置切换为 YAML，并完善带注释的示例

Risk profile: not-created（最终层级为 standard，不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 服务端当前只通过 `serde_json::from_str` 读取配置，默认文件名、根运行配置、
    Docker 入口和 README 都固定为 `filehub-server.json`；改为 YAML 会直接改变
    公开部署配置契约与默认启动行为。
  - 本提案按用户“修改成 YAML”的字面要求采用 YAML-only 产品契约，不实现 JSON
    专用兼容或格式自动探测，因而存在明确的兼容与升级边界；Docker 生成配置和
    发布用户手册也必须同步。
  - 实现需要新增 YAML 解析依赖，触发 dependency/build graph、deployment、
    public configuration contract 与 compatibility 的实质影响，符合 high-risk。
  - `serde_yaml` 与 `serde_yml` 当前均已停止维护；实现拟采用仍在维护且提供
    `serde_saphyr::from_str` 强类型反序列化入口的 `serde-saphyr` 1.x，避免把
    已弃用依赖引入服务端。
  - 用户已看过上述 public configuration contract、compatibility、deployment
    与 dependency/build graph 风险，并于 2026-08-27 显式要求“按standard任务
    完成就好”。按仓库规则，当前用户明确选择的 tier 优先，因此最终按 standard
    执行；这些风险作为 change record 与 completion review 的残余风险保留。
- Proposal and tier confirmation:
  - 用户 2026-08-27 回复“确认，按standard任务完成就好”，确认本提案的
    YAML-only 产品契约、示例注释/default 字段处理、Docker/README 同步范围，
    并把最终 tier 指定为 standard。

## Approval Record

- approver: 用户
- approval_date: 2026-08-27
- user_statement: “确认，按standard任务完成就好”
- selected_tier: standard
- accepted_residual_risk: 配置格式升级、部署入口同步与 YAML 解析依赖变更按
  standard 的变更记录、定向验证和比例化独立缺陷检查完成，不创建 high-risk
  风险档案或分阶段设计/测试/验收文档。

## Background and Goal

当前 `server/src/main.rs` 默认读取 `filehub-server.json`，并只用 JSON 解析器装载
`ServerConfig`。仓库同时提供根运行配置 `filehub-server.json`、示例配置
`server/config.example.json`，Docker 入口也生成 JSON；JSON 无法承载用户要求的
逐字段注释。

目标是把服务端配置入口完整切换为 YAML：默认路径、示例、根运行配置、Docker
生成配置和用户文档使用统一的 `.yaml` 形式；示例中的每个字段都有就近说明，
具有可用默认值的字段以注释形式展示，实际省略后由现有 `serde(default)` 逻辑补齐。

## Scope

### In scope

- `server/src/main.rs`：默认配置名改为 `filehub-server.yaml`，使用维护中的 YAML
  解析器把单文档 YAML 反序列化为现有 `ServerConfig`；显式路径参数与
  `FH_CONFIG` 的优先级保持不变。
- `server/Cargo.toml`、`Cargo.lock`：加入 YAML 强类型反序列化所需的直接依赖；
  `serde_json` 仍用于 HTTP JSON 契约、token payload 等既有路径，不删除。
- 用 `server/config.example.yaml` 替换 `server/config.example.json`，并用
  `filehub-server.yaml` 替换根 `filehub-server.json`：
  - 为顶层分组以及每个可配置字段添加就近中文注释；
  - 必填或没有可用默认值的字段保持启用，包括监听地址、端口、会话密钥、至少
    一个可登录账号、文件目录与归档上限；
  - 已有可用默认值的字段全部保留为注释示例而不实际赋值，包括 CORS 列表、
    `max_age`、`support_credentials`、两项登录限流和 `db_path`；
  - `password_hash` 作为 `password` 的可选替代项以注释示例说明，不把
    “两者都省略后账号不可登录”误写成可用默认值。
- `server/tests/unit/config.rs`：直接解析带注释且省略默认字段的示例 YAML，断言
  必填字段和所有默认值；增加非法 YAML 与缺少必填字段的失败边界核对。
- `docker/entrypoint.sh`：默认配置路径切换为 `.yaml`，生成 YAML 文档并安全转义
  环境变量值；容器现有账号、端口、数据目录、限流与持久化语义不变。
- 更新 `README.md`、`docker/README.md` 和 `docs/modules/filehub.md` 中当前有效的
  配置文件名、复制/启动命令和解析依赖说明。
- 完成 standard 的 change record、定向验证和独立缺陷检查材料。

### Out of scope

- 不保留 JSON/YAML 双格式自动探测，不为 `.json` 扩展名提供专用兼容分支，也不
  添加配置迁移命令；升级方式是把旧配置按新示例改写为 YAML。JSON 文本若因属于
  YAML 1.2 子集而被底层解析器接受，只视为未承诺的实现行为。
- 不改变 `ServerConfig` 字段名、字段层级、业务含义、默认值或校验规则；只改变
  外部序列化格式与示例呈现。
- 不修改 HTTP API 的 JSON 请求/响应格式，不移除项目其它用途的 `serde_json`。
- 不回写历史已完成任务文档中的 `.json` 记录；只更新当前用户入口和模块说明。
- 不借本任务清理、格式化或回退共享工作树中其他任务的既有未提交修改。

### Boundary with neighboring modules

- 变更归属 `filehub-server` 的启动/部署配置边界；admin-web 与 CLI 的 HTTP JSON
  契约不变。
- Docker 入口是同一服务端配置的部署消费者，必须同步切换；nginx 配置和镜像
  对外端口不变。
- 当前 `054-switch-published-sfo-account` 正在 acceptance，且已修改
  `server/src/main.rs`、`server/src/model/config.rs`、Cargo、README 等相邻文件；
  本任务在进入实现前使用独立 Harness 基线，只叠加 YAML 相关差异。

## Requirement Review

- 需求合理：YAML 原生支持注释，适合需要逐字段解释和展示默认值的运维配置；
  把有默认值的字段注释掉还能验证真实默认路径，减少示例值与代码默认值漂移。
- 关键权衡 1（兼容性）：YAML 1.2 接受 JSON 子集，但本提案仍把产品契约定义为
  YAML-only，并移除/重命名仓库 JSON 配置入口；不为拒绝 JSON 额外实现语法探测，
  也不把底层解析器偶然接受 JSON 的行为写入兼容承诺。
- 关键权衡 2（默认字段展示）：示例里的注释行只承担说明与可复制覆盖用途；测试
  必须证明删去这些实际键后得到的值与注释一致。`password`/`password_hash` 虽在
  serde 层可缺省为 `None`，但没有可登录语义，因此保留一个有效密码示例。
- 关键权衡 3（解析依赖）：不使用已经标注不再维护的 `serde_yaml`/`serde_yml`；
  拟使用 `serde-saphyr` 1.x 的 `from_str`，并由 Cargo.lock 固定实际版本。
- 关键权衡 4（Docker 安全转义）：入口生成的是机器配置，不要求复用示例注释；
  但用户名、密码、会话密钥和路径必须按 YAML 安全标量写入，不能用未经转义的
  heredoc 插值。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-server-yaml-loader | 服务端默认读取 `filehub-server.yaml`，以维护中的 YAML 解析器反序列化现有 `ServerConfig`，只承诺 YAML 配置入口 | 只改启动装载与直接依赖；HTTP JSON 契约不变 | 明确产生配置升级动作，换取单一 YAML 契约与注释能力 | YAML 示例可解析；默认路径为 `.yaml`；非法 YAML 报错包含配置路径但不泄露密钥；targeted server tests/build 通过 | 不做双格式探测或 JSON 专用兼容，不改字段语义 |
| P-002 | fh-server-yaml-examples | 两份当前配置改为带逐字段中文注释的 YAML；有可用默认值的键全部注释掉，并由解析测试证明注释值与代码默认一致 | 替换根运行配置和 `server/config.example.*`；不重写历史任务文档 | 必填字段继续启用；密码替代项不能被误当作可用默认值 | YAML 中无未注释的默认键；测试断言 CORS、max_age、credentials、限流和 db_path 默认值；示例账号可通过现有配置校验 | 不新增/重命名业务字段，不改变默认值 |
| P-003 | fh-server-yaml-deployment | Docker 入口、根 README、Docker README 与模块依赖说明全部切换到 `.yaml`，容器生成值安全转义 | nginx、端口、数据卷和环境变量语义不变 | Docker 机器生成配置无需携带示例注释，但必须是真实 YAML 且可被同一解析器读取 | `sh -n` 通过；入口生成配置的定向测试可解析且关键值正确；当前文档无活动 `.json` 配置引用 | 不改变镜像运行拓扑，不新增环境变量 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - 用户按 `server/config.example.yaml` 复制为 `filehub-server.yaml` 后可直接启动；
    示例每个字段都有中文说明，代码已有可用默认值的字段均保持注释状态。
  - 无参数启动默认查找 `filehub-server.yaml`；旧 JSON 配置不再是承诺的兼容路径，
    仓库不再提供当前 `.json` 配置文件。
  - Docker 入口生成并启动 YAML 配置，现有部署参数和数据位置不变。
- Required evidence:
  - 示例 YAML 解析与默认值单元测试；无默认值字段缺失时的失败测试；配置校验测试。
  - `cargo test -p filehub-server` 中与配置相关的 targeted 测试和至少一次
    `cargo check -p filehub-server`；依赖锁文件与代码调用一致。
  - `sh -n docker/entrypoint.sh`，以及隔离环境下对入口生成 YAML 的解析/关键值核对；
    `rg` 确认当前运行入口、README 与 Docker 文档不再引用活动 `.json` 配置名。
  - 独立验收覆盖兼容边界、默认值漂移、YAML 标量转义、敏感值错误输出与共享工作树
    副作用。
- Explicit non-goals:
  - 不兼容旧 JSON；不改变 API JSON；不变更业务默认值、数据模型、端口或目录语义。

## Risks

- 兼容/升级：已有部署必须按 YAML 示例迁移；旧 JSON 可能因属于 YAML 子集而被
  当前底层解析器接受，也可能在未来解析器/语法边界变化后失败，不能把这种偶然
  接受写成兼容承诺。
- 供应链与构建：新增 YAML 解析依赖会改变 Cargo.lock 和 musl 构建图；需用当前
  工具链完成 server check/test，并在设计中记录版本与回滚方式。
- 默认值漂移：示例注释不是 serde 自动生成；必须通过读取真实示例的测试绑定注释
  期望与代码默认值，并在验收中逐项核对。
- YAML 标量：`on`、`yes`、冒号、井号、引号、反斜杠和换行等输入可能被错误解释；
  Docker 生成逻辑必须使用安全编码，不直接拼接未经转义的环境变量。
- 敏感信息：解析/校验错误不得输出 `users.session_key` 或密码内容；沿用只报告路径
  与解析位置的错误边界，并增加针对性检查。
- 共享工作树：相关 Cargo、server、README 与配置文件已有其他在制任务改动；本任务
  必须在实现前捕获基线，逐块编辑且不运行仓库级格式化，验收时区分本任务增量。
