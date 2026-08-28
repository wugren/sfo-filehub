# Lightweight Acceptance Report

## Object and Scope

- Task manifest: task.yaml
- Workflow tier: trivial
- Change record: not-applicable
- Approved proposal: proposal.md

## Delivery Summary

- Outcome: Docker 镜像已从 `FH_*` 环境变量生成配置切换为只读取固定路径 `/etc/filehub/filehub-server.yaml`。入口脚本不再生成、覆盖或补全 YAML，不再自动生成管理员密码和 Ed25519 私钥；配置缺失/不可读时直接失败，并持续监督 filehub-server 与 nginx，任一进程退出都会终止容器，避免配置或端口错误后只留下 nginx 假存活。
- Handoff: 部署前复制 `docker/filehub-server.example.yaml`，替换示例私钥和管理员密码，设置宿主文件权限 `0600`，以只读 bind mount 挂载到固定配置路径，同时把持久化目录挂载到 `/data`。容器内部 YAML 必须使用 `server_addr: 127.0.0.1`、`port: 8080`、`files.data_dir: /data/files`、`db_path: /data/filehub.db`；外部端口只调整 Docker `-p` 左侧。

## Proposal Consistency

| change_id | requirement_or_boundary | proposal_source | delivery_evidence | finding | status |
|-----------|-------------------------|-----------------|-------------------|---------|--------|
| fh-docker-mounted-config | Docker 镜像仅从只读挂载的固定 YAML 启动，不读取 `FH_*` 生成配置；提供 Docker 专用模板、固定内部 8080、同步 CI/文档/测试，并保持 `/data` 数据布局 | proposal.md P-001 | `docker/entrypoint.sh` 固定读取并监督配置启动；`docker/nginx.conf` 与 `docker/filehub-server.example.yaml` 对齐 `127.0.0.1:8080`；Dockerfile 移除 jq/openssl 生成依赖；README/Compose/Actions smoke 全部改用只读挂载；新增 8 项 Docker 契约并扩展 Actions 契约 | 与批准提案一致 | pass |

## Independent Defect Discovery

| category | evidence_inspected | adversarial_check | finding_or_not_applicable_reason | status |
|----------|-------------------|------------------|----------------------------------|--------|
| behavior-and-logic | Dockerfile、entrypoint、nginx、Docker YAML 示例、Actions smoke 与两个契约测试文件 | 设置旧 `FH_ADMIN_PASSWORD` 但不挂配置执行入口；扫描 runtime/CI/docs 的 `FH_*`、jq 配置生成、openssl 自动密钥与 nginx 端口模板；核对 server/nginx 任一退出是否会结束容器 | 旧环境变量不能绕过固定配置，缺失配置返回非零和明确错误；runtime 配置面无 `FH_*` 或生成器；entrypoint 持续监督两个进程，消除了初版仅等待 1 秒可能留下 nginx 假存活的缺陷 | pass |
| boundaries-and-failure-paths | 固定配置路径检查、只读 mount、配置哈希、示例中的监听/数据/私钥字段、进程退出传播和旧部署迁移说明 | 检查配置是否被重定向、sed/cp/mv 改写；用真实 filehub-server 解析 Docker 示例并在临时端口/数据目录启动；检查无配置容器反例、CI 配置哈希前后比较以及错误私钥迁移后果 | 未发现配置写入路径；真实 server 已解析示例并完成数据库初始化；CI 会验证只读配置可启动、文件哈希不变且无配置镜像失败；固定 8080、密钥迁移及重新登录影响已明确 | pass |
| regression-and-side-effects | 059 pre-edit 基线、与 058 重叠的 workflow/test 基线快照、GitHub Actions 全套契约、server 配置相关 Rust 测试、README 相对链接 | 逐行比较 `.github/workflows/build.yml` 和 `tests/github_actions_build_contract.py` 相对 059 基线，只允许 smoke 挂载与对应测试增量；反查 latest 双标签发布、发布门禁、Cargo lock、Release 资产和任务外脏文件是否漂移 | 058 的 latest 发布差异保持在基线中，059 只增加配置挂载 smoke；Actions 16/16、server 配置相关 12/12、文档链接 13/13 通过；未触碰 Cargo.lock、Harness 脚本或根目录现有配置/数据库 | pass |

## Verification

- Targeted check: `tests/docker_config_contract.py` unit 3/3、DV 3/3、integration 2/2；`tests/github_actions_build_contract.py` unit 5/5、DV 6/6、integration 5/5；`cargo test -p filehub-server --locked config -- --nocapture` 12/12；真实 `target/debug/filehub-server` 使用 Docker 示例（仅把探针端口改为 38080、数据路径改到临时目录）成功启动并初始化 SQLite；旧 `FH_ADMIN_PASSWORD` + 缺失配置运行探针按预期失败；PyYAML、`sh -n`、`py_compile`、13 个相对链接、runtime `FH_*` 零命中与任务范围 `git diff --check` 均通过
- Result: pass
- Exception reason: 当前环境没有 Docker daemon、nginx、actionlint 或 shellcheck，无法本地构建/启动完整镜像及执行 nginx 原生命令；GitHub Actions smoke 已包含真实镜像正例、配置哈希不变和缺失配置反例，需后续 hosted runner 结果确认。

## Findings

| ID | Severity | Evidence | Problem | Blocking |
|----|----------|----------|---------|----------|
| F-1 | medium | Docker entrypoint 已完全删除 `FH_*` 兼容层；README 提供迁移与回滚步骤；用户确认时选择 trivial 并接受提案 | 现有 `-e FH_*` 部署升级后会因缺少挂载 YAML 而启动失败，属于有意 breaking deployment change | no |
| F-2 | medium | YAML 必须包含完整 Ed25519 私钥；旧镜像自动生成的私钥位于 `/data/.session_private_key.pem` | 未迁移原私钥会使已有 session/refresh JWT 失效；配置文件泄露会暴露签名私钥和管理员凭据 | no |
| F-3 | low | `docker`、`nginx`、`actionlint`、`shellcheck` 均不可用；静态契约、真实 server 探针和 hosted smoke 合同通过 | 当前机器不能证明完整 nginx+server 镜像实际启动，最终证据来自后续 GitHub-hosted Docker smoke | no |
| F-4 | low | nginx 固定反代 `127.0.0.1:8080`，示例与测试对齐并在文档突出 | Docker YAML 若修改 server 地址或内部端口，server 与 nginx 会失联；该字段在容器部署中不是可变配置 | no |

## Conclusion

- Accepted / rejected / needs changes: accepted
- Reason: `fh-docker-mounted-config` 已按批准范围完成；Docker 不再通过环境变量生成配置，只读 YAML、固定内部端口、持久化路径、失败关闭、双进程监督、CI smoke、迁移文档和契约测试均已闭合。实现与 059 基线比较未夹带 058 或其它工作树差异，未发现阻塞缺陷；部署兼容中断、secret 管理、完整镜像本地不可验证和固定端口限制已作为非阻塞残余风险记录。
