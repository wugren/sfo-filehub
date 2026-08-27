# 服务端配置切换为 YAML

- Status: complete
- Owner module: filehub
- Task manifest: docs/versions/v0.1/modules/filehub/055-server-yaml-config/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/055-server-yaml-config/proposal.md
- Affected paths: `server/src/main.rs`、`server/Cargo.toml`、`Cargo.lock`、`server/config.example.yaml`、`filehub-server.yaml`、对应旧 JSON 文件、`server/tests/unit/config.rs`、`docker/entrypoint.sh`、`README.md`、`docker/README.md`、`docs/modules/filehub.md`
- Explicit tier override: 用户 2026-08-27 回复“确认，按standard任务完成就好”，在已展示 public configuration contract、compatibility、deployment 与 dependency/build graph 风险后明确选择 standard
- Expanded high-risk packet: none

## Approach

- 服务端默认配置名统一为 `filehub-server.yaml`，使用维护中的
  `serde-saphyr` 1.x 把单文档 YAML 直接反序列化为现有 `ServerConfig`；解析错误
  仅暴露配置路径与行列，不渲染可能包含密钥/密码的源码片段。
- 两份仓库配置替换为 YAML；逐字段添加中文说明，真正具有可用 serde 默认值的键
  只保留为注释示例，必填字段和可登录账号凭据保持启用。
- Docker 入口生成块状 YAML，并利用 jq 的 JSON 字符串编码作为 YAML 安全标量，
  避免环境变量中的冒号、井号、引号、反斜杠或换行改变 YAML 结构。
- 当前用户入口文档改用 `.yaml`；历史任务记录保持不变。

## Risk Screen

- Public contract, protocol, or CLI change: yes；服务端公开部署配置从 JSON 改为 YAML-only 产品契约
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no；但解析错误与 Docker 标量转义需防止敏感配置泄露/结构注入
- Concurrency, lifecycle, or runtime integration change: yes；默认启动配置名与 Docker 生成配置同步变化
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: yes；新增 YAML 解析依赖，旧部署需按示例迁移
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

上述 `yes` 均为确认前已展示、且仍位于已确认需求范围内的风险。用户显式选择
standard，故不升级任务；通过定向解析/默认值/错误脱敏/Docker 标量测试与独立缺陷
检查保留证据。

## Verification

- Targeted check: `cargo check -p filehub-server --locked`；`cargo test -p filehub-server --test unit_tests --locked`（68/68）；配置定向测试（6/6）；非法 YAML 错误路径/行列与敏感值不回显探针；`sh -n docker/entrypoint.sh`；从入口脚本精确抽取 jq 生成命令，以包含冒号、井号、引号、反斜杠和换行的值生成块状 YAML，再由实际 `filehub-server` 解析到配置校验阶段；两份 YAML 一致性、默认字段注释态、活动 JSON 配置引用与共享基线增量核对
- Result: pass
- Residual risk or follow-up: JSON 文本属于 YAML 1.2 子集时可能被底层解析器偶然接受；产品仅承诺 YAML，不增加专用 JSON 拒绝或兼容分支。本环境无 Docker daemon 且未安装 shellcheck，完整镜像启动留给现有 CI 容器冒烟；本地已完成 `sh -n` 与真实 jq→server parser 链路验证
