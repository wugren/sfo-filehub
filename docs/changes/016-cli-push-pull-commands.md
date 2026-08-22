# 016 CLI 命令面改为 push / pull，统一 `<server:...>` 目标形态

- Status: complete
- Owner module: filehub（文件集散 v0.1）
- Task manifest: `docs/versions/v0.1/modules/filehub/016-cli-push-pull-commands/task.yaml`
- Approved proposal: `docs/versions/v0.1/modules/filehub/016-cli-push-pull-commands/proposal.md`
- Affected paths: `cli/src/cli/`（args、mod.rs、全部 handler）、`cli/src/apiclient/mod.rs`（注释）、`cli/tests/`、`cli/README.md`、`docs/modules/filehub.md`
- Explicit tier override: none（用户确认 standard）
- Expanded high-risk packet: none

## Approach

- `publish` 改名为 `push`，参数改为 `push server:project:version:name + 本地路径`；
  新增 `pull server:project:version:name + 输出文件路径`，输出路径为精确文件路径，
   SHA-256 校验后经隐藏临时文件原子 rename 落盘（复用 `archive::finalize_download`）。
- `versions`/`new-version`/`lock-version`/`delete-app` 同步统一为
  `server:project` / `server:project:version` / `server:project:version:name`
  目标形态，删除独立 `[SERVER]` 位置参数。
- 旧命令 `publish`/`download` 直接移除，不留别名（用户确认）。
- 目标串从右侧固定切分：最后 N 段为 project/version/name 固定字段，剩余部分
  整体作为 server，因此 `127.0.0.1:8080:test:1.1.0:logs` 与 IPv6 含冒号
  server（`[::1]:8080`、`::1:8080`）均正确解析；固定字段不允许为空。
- 服务端 v1 API、登录/凭据机制、退出码、打包与完整性校验逻辑均不变；login/
  logout 保持 `[SERVER]` 形态。
- 行为变化：pull 不再支持省略版本取 latest（目标必含 version）；下载跳过
  `sanitize_artifact_name` 自动命名，直接写用户给定文件路径。

## Risk Screen

- Public contract, protocol, or CLI change: **yes**——CLI 命令面破坏性变更
  （`publish`/`download` 移除、六条命令目标形态改变）；属用户确认范围，旧脚本
  需按新命令改写，本记录提供映射。
- Persistent data, schema, or migration change: no
- Security, privacy, or trust-boundary change: no
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no
- Material UI, accessibility, localization, or navigation workflow change: no
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-cli` 全量（单元 + api_integration + dv）；
  `cargo fmt --check --edition 2024` 覆盖本次改动的源文件；`filehub push/pull
  --help` 与 `filehub --help` 命令表核对；真实 filehub-server 端点到端验证。
- Result: passed
  - `cargo test -p filehub-cli`：14 单元 + 15 api_integration + 4 dv = 33 全通过；
  - `rustfmt --edition 2024 --check` 覆盖本次实际改动的源/测试文件通过；
  - `filehub --help` 只显示 login/logout/push/pull/versions/new-version/
    lock-version/delete-app，publish/download 已消失；
  - 真实服务端联调（临时库 + 127.0.0.1:18181）：login -> new-version 1.1.0 ->
    push `e2e:1.1.0:logs`（sha256 f2a0422d...）-> versions 显示 `logs:103` ->
    pull 到精确路径 logs.tar.gz，sha256 与服务端一致、归档内 `logs/access.log`
    完整；new-version 1.2.0 + delete-app（成功/404 exit 5/锁定后 409 exit 4）+
    lock-version 均按新目标形态工作。
- Residual risk or follow-up: 破坏性命令面变更（已确认）；历史 `latest` 下载快捷
  方式随 `download` 移除，pull 需显式版本；如需要可后续增加
  `server:project:name` 省略版本回退 latest 的扩展。
