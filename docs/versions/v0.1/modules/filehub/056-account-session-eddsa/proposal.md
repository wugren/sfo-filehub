---
task_manifest: task.yaml
status: approved
---

# 账号会话 JWT 改用 EdDSA（Ed25519）签名

Risk profile: not-created（最终层级为 standard，不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 当前账号登录及 refresh JWT 由 `users.session_key` 驱动 HMAC-SHA256 签名；
    改为 EdDSA 会改变认证算法、密钥格式、服务启动配置与 Docker 密钥持久化方式。
  - 切换后，现有 HMAC session/refresh JWT 无法再通过验证，部署升级会要求用户
    重新登录，属于明确的 security、runtime/deployment 与 compatibility 边界。
  - 私钥属于高价值敏感材料，配置解析、错误输出、文件权限和容器日志均不能泄露
    私钥内容；这些安全责任需要分阶段设计、测试和独立验收。
  - 用户已看过上述风险，并于 2026-08-27 显式要求“按standard完成”。按仓库
    规则，当前用户明确选择的 tier 优先，因此最终按 standard 执行；风险保留在
    change record 和 completion review 中，不创建 high-risk 分阶段文档。
- Proposal and tier confirmation:
  - 用户 2026-08-27 回复“确认，按standard完成”，确认“仅配置 Ed25519 PKCS#8
    PEM 私钥、自动派生公钥、不兼容旧 HMAC 会话、Docker 缺省生成并持久化私钥”
    的提案边界，并把最终 tier 指定为 standard。

## Approval Record

- approver: 用户
- approval_date: 2026-08-27
- user_statement: “确认，按standard完成”
- selected_tier: standard
- accepted_residual_risk: 认证算法、私钥管理、部署配置和既有 session/refresh JWT
  兼容性变化按 standard 的变更记录、定向验证与比例化独立缺陷检查完成。

## Background and Goal

`AccountModule::init` 当前调用
`DefaultAccountManager::new_with_login_verifier_and_session_config`，把
`users.session_key` 作为 HMAC 密钥；项目 token JWT 已经使用 Ed25519，但账号登录
session 仍是对称签名。

目标是让账号登录 session JWT 与 refresh JWT 统一使用 EdDSA（Ed25519）签名。
服务端只接收 PKCS#8 PEM 私钥，验签公钥由 `sfo-account` 自动派生，不要求部署者
重复配置公钥。

## Scope

### In scope

- 把 `UsersConfig.session_key` 替换为语义明确的 `session_private_key`：值为
  Ed25519 PKCS#8 PEM 私钥；YAML 示例用块标量展示，并补充密钥生成说明。
- `AccountModule` 改用
  `DefaultAccountManager::new_eddsa_with_login_verifier_and_session_config`，保留现有
  密码验证器和 `SessionConfig` 行为，由 `sfo-account` 从私钥自动派生公钥。
- 启动校验拒绝空值、非 PKCS#8 PEM 或非 Ed25519 私钥，错误只描述字段和原因，
  不回显 PEM 内容。
- 调整账号、配置、API 与 CLI e2e 测试 fixture，新增 JWT header `alg=EdDSA`、
  login/refresh 可验证、篡改/错误密钥不可验证及 HMAC JWT 不可混用的反例。
- Docker 入口支持 `FH_SESSION_PRIVATE_KEY` PEM；未提供时生成 Ed25519 私钥，
  以 `0600` 持久化到 `/data/.session_private_key.pem`，重启复用同一私钥。为运行时
  镜像加入生成 Ed25519 私钥所需的 OpenSSL 工具。
- 同步根 README、Docker README、模块说明和两份 YAML 当前配置；完成 high-risk
  设计、实现、测试、验收与生命周期证据。

### Out of scope

- 不改变项目 token JWT 的独立 Ed25519 签发/公钥轮换设计。
- 不同时保留 HMAC 与 EdDSA 双验签，不为旧 `session_key` 字段或既有 HMAC JWT
  提供兼容窗口；升级后用户重新登录。
- 不要求配置验签公钥，不新增 JWKS、`kid`、外部 KMS/HSM 或密钥轮换 API。
- 不改变 JWT claims、session/refresh 有效期、HTTP 路由、密码校验与权限模型。
- 不借本任务修改 054 等其它任务的既有范围，也不清理共享工作树中的无关改动。

### Neighboring boundaries

- 账号 session JWT 仍只走 `AccountModule::decode_session`，项目 token JWT 仍只走
  `TokenService::resolve`；两类凭据不能互冒。
- Docker 负责私钥首次生成与持久化；非 Docker 部署必须在 YAML 中显式提供私钥。
- 当前 054 任务正在完成 `sfo-account 0.2.1` 来源切换，本任务以其已提供的
  private-key-only EdDSA 构造器为运行前提，但不改变依赖来源决策。

## Requirement Review

- 需求合理：EdDSA 使用非对称密钥，服务端签发和验签仍可封装在账号模块内，且
  当前 `sfo-account 0.2.1` 已提供从私钥自动派生公钥的构造器。
- 配置选择：使用 `session_private_key` 内联 PKCS#8 PEM，而不是继续沿用名字含混的
  `session_key`；这样能在 YAML 和部署文档中明确密钥类型，并避免把任意字符串误作
  Ed25519 私钥。
- 兼容选择：不做算法 fallback。双算法验签会延长对称密钥暴露面并使配置/审计
  边界含混；本次按一次性升级处理，旧 session/refresh JWT 失效是明确且可接受的
  部署动作。
- Docker 选择：自动生成并持久化私钥，避免每次重启使所有会话失效；显式环境变量
  仍适合由 secret manager 注入。文件权限和日志需要独立验证。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-account-session-eddsa-config | `users.session_private_key` 接收 Ed25519 PKCS#8 PEM 私钥，删除 HMAC `session_key` 契约，示例与测试 fixture 同步 | 只改变账号会话签名配置，不改变 claims 和账号列表 | 旧配置启动失败，换取明确且可校验的非对称密钥契约 | 有效私钥通过；空值、HMAC 字符串、错误 PEM、其它算法私钥均在启动期无泄露地失败 | 不兼容旧字段，不要求公钥 |
| P-002 | fh-account-session-eddsa-runtime | login 和 refresh JWT 使用 `alg=EdDSA`，服务端由同一私钥派生的公钥验签 | token JWT 维持独立签发和 resolve 路径 | 已签发 HMAC 会话全部失效，需要重新登录 | header 算法断言、login/refresh 正例、篡改/错误密钥/HMAC 反例均通过 | 不改 claims、TTL、路由、权限 |
| P-003 | fh-account-session-eddsa-deployment | Docker 支持显式 PEM 注入或首次生成并以 `0600` 持久化 Ed25519 私钥，文档同步升级步骤 | 私钥只存配置来源和 `/data` 持久卷，不进日志 | 运行镜像新增 OpenSSL 工具；密钥丢失会使现有会话失效 | 两次启动复用同一私钥；生成文件类型/权限正确；生成 YAML 可解析；日志不含私钥 | 不实现远程 KMS/HSM、在线轮换或 JWKS |

## Success Criteria

- 登录与 refresh 返回的 JWT header 明确为 `alg=EdDSA`，并可由账号模块正常解码；
  旧 HMAC JWT、被篡改 JWT 与不同私钥签发的 JWT 均被拒绝。
- 服务端配置只接受 `users.session_private_key` 的有效 Ed25519 PKCS#8 PEM；示例、
  根配置、测试 fixture、README 和 Docker 文档不再使用活动的 `session_key`/
  `FH_SESSION_KEY` 契约。
- Docker 在没有显式私钥时生成 `/data/.session_private_key.pem`，权限为 `0600`，
  重启后复用；显式 PEM 可安全写入 YAML，任何输出不泄露私钥。
- 账号相关定向测试、server 单元/集成测试、配置解析测试、Docker 入口检查通过；
  high-risk lifecycle 与独立 acceptance 结论为 accepted。
- 升级说明明确：旧配置必须改为 Ed25519 PKCS#8 PEM，既有用户重新登录。

## Risks

| 风险 | 等级 | 处理 |
|------|------|------|
| 私钥泄露 | 高 | 配置/错误/日志不回显；Docker 持久化文件 `0600`；验收检查输出路径 |
| 现有会话失效 | 高 | 明确不做 fallback；升级文档要求重新登录并说明回滚依赖保留旧密钥/版本 |
| 算法混用或降级 | 高 | 只构造 EdDSA signer；正反例检查 `alg`、HMAC 拒绝与两类 JWT 不互冒 |
| Docker 重启密钥漂移 | 高 | 私钥首次生成后持久化到 `/data`，重复启动比较公钥/签名连续性 |
| 无效密钥导致启动失败 | 中 | 启动期解析 PKCS#8/Ed25519，返回不含私钥的明确错误 |
| 运行镜像体积/供应面增加 | 中 | 仅加入发行版 OpenSSL 包，构建与容器 smoke 验证锁定实际行为 |
| 共享工作树覆盖 | 中 | high-risk 阶段基线记录既有差异，逐文件局部编辑，不做仓库级格式化 |

## Unresolved Questions

无。提案采用“仅配置 Ed25519 PKCS#8 PEM 私钥、自动派生公钥、不兼容旧 HMAC
会话、Docker 缺省生成并持久化私钥”的完整边界。
