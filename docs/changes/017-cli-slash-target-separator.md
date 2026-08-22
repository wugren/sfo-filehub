# 017 CLI 目标串分隔符改为 `/`（server/project/version[/name]）

- Status: complete
- Owner module: filehub（文件集散 v0.1）
- Task manifest: `docs/versions/v0.1/modules/filehub/017-cli-slash-target-separator/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/017-cli-slash-target-separator/proposal.md`
- Affected paths: `cli/src/cli/args.rs`、`cli/tests/api_integration.rs`、`cli/tests/dv_tests.rs`、`cli/README.md`
- Explicit tier override: none（用户确认 standard）
- Expanded high-risk packet: none

## Approach

- `cli/src/cli/args.rs` 三个目标解析函数改为按 `/` 切分：`server/project`、
  `server/project/version`、`server/project/version/name`，段数严格校验
  （缺段/多余段/空段均报输入错误），server 段保留 `host[:port]` 冒号
  （含 IPv6 `[::1]:8080`、`::1:8080`）。
- 切分前先剥离可选 `http(s)://` 协议前缀，与 login/logout 的旧习惯兼容；
  身份仍统一归一化为 `host[:port]`。
- 六个命令（push/pull/versions/new-version/lock-version/delete-app）的行为
  借此从「冒号右切分」改为确定性的分段解析，消除 016 中带端口 server 漏写字段
  时的静默错拆歧义。
- 测试目标串、README 命令表同步迁移到 `/` 形态；docs/modules/filehub.md
  无目标分隔符引用，无需改动。
- 旧 `:` 分隔输入不再接受；该命令面为新近确认形态，仓库内无存量脚本依赖。

## Risk Screen

- Public contract, protocol, or CLI change: **yes**——CLI 目标串分隔符 `:` ->
  `/`（属用户确认范围，README 同步；016 刚定稿的新命令面无存量兼容负担）。
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-cli` 全量（单元 + api_integration + dv）；
  `rustfmt --edition 2024 --check` 覆盖本次改动源/测试文件；`filehub push/pull
  --help` 与 `filehub --help` 命令表核对；真实 filehub-server 端到端验证。
- Result: passed
  - `cargo test -p filehub-cli`：14 单元 + 15 api_integration + 4 dv = 33 全通过
    （含 `/` 目标解析正反例：IPv4+端口、IPv6、无端口 host、缺段/空段/多余段、旧
    `:` 分隔拒绝）；
  - `rustfmt --edition 2024 --check` 覆盖 args.rs 与两个测试文件通过；
  - `push --help`：`<server/project/version/name>`；`versions --help`：
    `<server/project>`；delete-app：`<server/project/version/name>`；
  - 真实联调（临时库 127.0.0.1:18182）：login -> new-version
    `127.0.0.1:18182/e2e/1.1.0` -> push `.../e2e/1.1.0/logs`（sha256
    4d6e9d77...）-> versions 显示 `logs:103` -> pull 到 logs.tar.gz 且 SHA 一致；
    new-version 1.2.0 + delete-app（成功/404 exit 5/锁定后 409 exit 4）+
    lock-version 均按 `/` 形态工作；旧冒号目标串明确报 exit 5。
- Residual risk or follow-up: project/version/name 不能包含 `/`（与现命名习惯
  一致）；如未来需要斜杠版本号需重新设计分隔符。
