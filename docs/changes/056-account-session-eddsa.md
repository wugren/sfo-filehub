# 账号会话 JWT 改用 EdDSA（Ed25519）签名

- Status: complete
- Owner module: filehub
- Task manifest: `docs/versions/v0.1/modules/filehub/056-account-session-eddsa/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/056-account-session-eddsa/proposal.md`
- Affected paths: `server/src/model/config.rs`、`server/src/account/mod.rs`、当前 YAML 配置、账号/配置/CLI e2e 测试、`docker/`、`README.md`、`docs/modules/filehub.md`
- Explicit tier override: 用户 2026-08-27 明确选择 standard，接受认证算法切换、私钥管理、部署和既有会话失效风险
- Expanded high-risk packet: none

## Approach

把账号配置从 HMAC 字符串 `session_key` 替换为 Ed25519 PKCS#8 PEM
`session_private_key`，启动期用 `ed25519-dalek` 脱敏校验密钥格式，账号装配改用
`sfo-account` 的 private-key-only EdDSA 构造器并自动派生公钥。Docker 由 OpenSSL
首次生成私钥并以 `0600` 持久化，或接收 secret manager 注入的完整 PEM；不做
HMAC fallback，既有 session/refresh JWT 在升级后失效。

## Risk Screen

- Public contract, protocol, or CLI change: yes — 服务端配置字段由 `session_key` 改为 `session_private_key`，JWT header 算法变为 EdDSA
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: yes — 对称 HMAC 密钥改为 Ed25519 私钥，需防止私钥出现在错误和日志中
- Concurrency, lifecycle, or runtime integration change: yes — Docker 首次生成并跨重启复用私钥
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: yes — runtime 镜像加入 OpenSSL；旧配置不兼容且已有会话失效
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

上述均属于已展示并由用户以 standard 明确接受的提案内风险，不改变已确认需求，
因此保持 standard；通过定向正反例、Docker 密钥持久化探针与比例化独立缺陷检查
收口。

## Verification

- Targeted check:
  - `cargo check -p filehub-server --locked`
  - `cargo test -p filehub-server --test unit_tests --locked`（69 passed）
  - `cargo test -p filehub-server --test unit_tests unit::config --locked`（6 passed）
  - EdDSA 正反例定向测试：login/refresh/续期 JWT header、篡改、HMAC access 与
    refresh、不同 Ed25519 私钥
  - `cargo test -p filehub-server --test api_integration --locked`；并行运行 17 passed、
    3 个已知 502/EOF 夹具竞争失败，失败项逐个串行重跑全部通过
  - `cargo test -p filehub-cli --test e2e_cli_server --locked`（1 passed）
  - `sh -n docker/entrypoint.sh`；OpenSSL 两次启动密钥复用、`0600` 权限、jq 多行
    PEM YAML→真实 server parser、错误输出不含私钥探针
  - 两份 YAML 内容一致、活动 `session_key`/`FH_SESSION_KEY` 引用扫描、任务路径
    `git diff --check`、pre-edit 基线逐文件差异检查
- Result: pass
- Residual risk or follow-up:
  - 旧 HMAC 配置和 JWT 不兼容；升级必须配置/生成 Ed25519 私钥并让用户重新登录。
  - 本机无 Docker daemon 且未安装 shellcheck，未运行完整镜像 smoke/shellcheck；
    已以 shell 语法检查、真实 OpenSSL/jq/server parser 链路和现有 CI 容器 smoke
    作为替代证据。
