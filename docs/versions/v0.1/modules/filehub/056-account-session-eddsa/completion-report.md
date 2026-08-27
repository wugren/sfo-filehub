# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: standard
- Change record: docs/changes/056-account-session-eddsa.md

## Delivery Summary

- Outcome: 账号 login 与 refresh JWT 已从 HMAC 切换为 EdDSA（Ed25519）；服务端
  配置只接收 `users.session_private_key` 的 Ed25519 PKCS#8 PEM 私钥，公钥由
  `sfo-account` 自动派生。两份 YAML 示例、账号装配、测试 fixture、Docker 入口、
  README 与模块说明均已同步。
- Handoff: 不保留 `users.session_key` 或 HMAC 双验签；升级后已有 session/refresh
  JWT 失效，用户需重新登录。Docker 未显式注入私钥时，在
  `/data/.session_private_key.pem` 首次生成并以 `0600` 持久化，后续启动复用。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-account-session-eddsa-config | `session_private_key` 只接受 Ed25519 PKCS#8 PEM，删除 HMAC `session_key`，不要求公钥 | proposal.md P-001 | `server/src/model/config.rs` 用 `SigningKey::from_pkcs8_pem` 脱敏校验；两份 YAML 与所有 fixture 已换成 PEM；配置测试覆盖有效 Ed25519、普通字符串和有效但算法错误的 X25519 PKCS#8 | 匹配 | pass |
| fh-account-session-eddsa-runtime | login/refresh JWT 使用 `alg=EdDSA`，claims/TTL/路由不变，HMAC/篡改/错误私钥拒绝 | proposal.md P-002 | `AccountModule` 调用 private-key-only EdDSA 构造器；unit/API 测试断言登录、refresh 和续期 header；HMAC access+refresh、篡改、不同 Ed25519 私钥均失败；账号/token 回归通过 | 匹配 | pass |
| fh-account-session-eddsa-deployment | Docker 显式 PEM 注入或首次生成并持久化私钥，权限最小化，文档说明升级影响 | proposal.md P-003 | `docker/Dockerfile` 安装 OpenSSL；entrypoint 用 `openssl genpkey -algorithm Ed25519`、`0600` 文件和 jq `@json` 写 YAML；两次启动复用、PEM 解析和日志脱敏探针通过；README 同步 | 匹配 | pass |

## Independent Defect Discovery

| Category | Evidence Inspected | Adversarial Check | Finding or Not-Applicable Reason | Status |
|----------|--------------------|-------------------|----------------------------------|--------|
| behavior-and-logic | `UsersConfig::validate`、`AccountModule::init`、sfo-account EdDSA 构造器、login/refresh API、JWT header | 核对 private-key-only 构造器确实自动派生公钥；断言初始 access/refresh 与续期后两枚 JWT 都是 EdDSA；确认项目 token 的独立 Ed25519 路径未改 | 账号两类 JWT 均使用 EdDSA，现有 claims、TTL、密码验证和项目 token 生命周期未漂移 | pass |
| boundaries-and-failure-paths | PKCS#8 解析、jsonwebtoken 算法族校验、Docker 密钥分支、jq YAML 编码、main 脱敏错误 | 输入普通字符串和 X25519 PKCS#8；修改 JWT 签名；用 HMAC JWT 调用 access/refresh；换另一 Ed25519 私钥；把多行 PEM 经 jq 写入 YAML 再交给真实 server parser；检查 stderr 无 PEM header | 无效/错误算法密钥启动期 fail closed；HMAC、篡改和错误私钥均拒绝；私钥未出现在错误输出 | pass |
| regression-and-side-effects | 056 pre-edit 基线、重叠的 account/config/test/Docker/README 文件、69 个 server unit tests、20 个 API 集成场景、CLI e2e | 逐文件与基线对比，确认只叠加 EdDSA 字段/构造器/测试/部署差异；API 并行 3 个 502/EOF 失败逐个串行复跑；两次模拟 Docker 启动比较私钥哈希和权限 | 69 unit 全通过；API 并行其余 17 个通过且 3 个失败项串行全部通过，符合既有共享服务器竞争症状；CLI e2e 通过；未覆盖共享工作树其它任务修改 | pass |

## Verification

- Targeted check: `cargo check -p filehub-server --locked`；server unit 69/69；配置
  6/6；EdDSA 正反例定向测试；API integration 并行 17/20 后 3 个失败项逐个串行
  通过；CLI real-server e2e 1/1；`sh -n`；OpenSSL 两次启动私钥复用/`0600`；
  jq 多行 PEM→真实 server parser 与错误脱敏；YAML diff、活动旧字段扫描、
  `git diff --check`、任务基线差异检查
- Result: pass
- Exception reason: 本环境没有 Docker daemon 且未安装 shellcheck，未执行完整容器
  启动和 shellcheck；Docker 密钥生成/复用、权限、YAML 生成与 server 解析使用真实
  OpenSSL、jq 和 server binary 完成替代验证，仓库现有 CI 保留容器 smoke。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | low | API integration 并行运行中 3 个用例出现 login 502/EOF；同三项逐个串行均通过，EdDSA 定向与其它 17 项通过 | 共享测试服务器的既有并行夹具竞争会让全量并行结果不稳定，不是本任务的算法或验签失败 | no |
| F-2 | low | `docker info` 不可用，shellcheck 未安装；真实 OpenSSL/jq/server parser 探针通过 | 当前环境不能提供完整镜像启动和 shellcheck 证据 | no |
| F-3 | medium | 配置字段和签名算法均无 fallback；README 已明确升级动作；用户已在提案确认时接受 | 旧 HMAC 配置无法启动，既有 session/refresh JWT 在升级后全部失效 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: 三个 change_id 均与批准提案一致；账号 login、refresh 与续期 JWT 已有
  `alg=EdDSA` 的真实正例，错误算法/密钥/签名和敏感信息边界有反例，Docker 私钥
  生成与跨启动持久化有独立探针。未发现阻塞缺陷；并行夹具噪声、缺少 Docker
  daemon/shellcheck 和明确接受的旧会话失效均已作为非阻塞残余记录。
