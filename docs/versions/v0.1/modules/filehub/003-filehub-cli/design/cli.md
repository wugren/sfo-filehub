---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-20
approved_content_sha256: d55085449026426908b7cff61bd9157a713ba983bb03eda1e2a3d114af146d4e
---

## Approval Record

- approver: user
- approval_date: 2026-08-20
- user_statement: 自动完成003任务吧


# cli 子模块设计（命令装配）

## Responsibility

- 冻结并解析 `filehub-cli` 全部命令面（login/logout/publish/download/versions），包括参数、环境变量、交互与输出；校验参数互斥与组合规则；把命令 handler 编排结果映射为稳定退出码。
- 不持有跨命令持久状态；不直接读写 HTTP（归 apiclient）；不直接持久化凭据（归 credential_store）；不直接操作归档（归 archive）。

## Command Surface

### 全局与 login/logout（与已批准 proposal「filehub login 参数定义」一致）

| 命令/参数 | 说明 |
|-----------|------|
| `filehub login [SERVER] [-u <USER>] [--password-stdin \| --token-stdin] [--config <PATH>]` | 密码或 token 登录；两种模式互斥；`SERVER` 缺省依次取 `FILEHUB_SERVER` > 配置默认值 > 唯一已存 server |
| `-u, --username <USER>` | 密码登录用户名；缺省交互提示 |
| `--password-stdin` | 从 stdin 读密码（剥末尾换行）；非终端 stdin 必须显式二选一 |
| `--token-stdin` | 从 stdin 读 token JWT；与密码选项互斥 |
| `--config <PATH>` | 覆盖配置文件路径（缺省 `FILEHUB_CONFIG` 或平台默认目录 `config.toml`） |
| `filehub logout [SERVER] [--config <PATH>]` | 清除指定/默认服务器的全部本地凭据 |

禁止提供 `--password <明文>` / `--token <明文>`；密码/token 只经交互输入、stdin 或环境变量进入。

### publish / download / versions

- `filehub publish <文件或目录> <project>:<version> [--config <PATH>] [SERVER]`
  - `<project>` 是项目名；`<version>` 是发布版本号；服务端唯一性与不可覆盖由 API 保证。
- `filehub download <project>[:<version>] -o <目录> [--config <PATH>] [SERVER]`
  - 省略 `<version>` 等价于 `latest`（服务端最近发布语义）；`-o` 必填目录。
- `filehub versions <project> [-o <路径>] [--format text|json] [--config <PATH>] [SERVER]`
  - 省略 `-o` 输出 stdout；`--format` 缺省 `text`。
- 全部命令支持 `-h/--help` 与全局 `--config`；其余命令无 `SERVER` 位置参数时经 credential_store/server 解析规则取服务器。
- `SERVER` 传入/存储身份为 `host[:port]`，不要求协议头；显式协议头仅作为历史输入被剥除，请求层按 HTTPS 优先、loopback HTTP 降级执行。

## Exit Codes

| 退出码 | 类别 | 触发 |
|--------|------|------|
| 0 | 成功 | 命令完成（含 `--help`） |
| 1 | 用法/参数错误 | 参数解析失败、模式互斥冲突、stdin 非终端未显式选模式 |
| 2 | 认证失败 | 登录失败、401、续期失败、无可用凭据 |
| 3 | 授权失败 | 403 |
| 4 | 冲突 | 409（版本已存在等） |
| 5 | 输入无效 | 422、项目名解析不到、版本格式非法 |
| 6 | 网络/传输 | 超时、连接失败、TLS、5xx |
| 7 | 内容/完整性 | SHA-256 不一致、归档不安全、下载校验失败 |
| 8 | 本地文件系统 | 目标目录不可写、配置损坏且不可恢复、文件操作失败 |

## Environment Variables

`FILEHUB_SERVER`（默认 SERVER）、`FILEHUB_USERNAME`（默认用户名）、`FILEHUB_PASSWORD`（密码登录密码）、`FILEHUB_TOKEN`（token 登录凭据）、`FILEHUB_CONFIG`（默认配置文件路径）。优先级：显式命令行选项 > 环境变量 > 交互式提示；环境变量可见于同用户进程，日志不得输出其值（与 proposal 确认一致）。

## Key Flows

### login 交互与参数校验

1. 解析命令面；`--password-stdin`/`--token-stdin` 同时出现或与 `-u` 混用（token 模式）→ 退出码 1；
2. 无凭据选项且 stdin 为终端：提示选择账号密码或 token；密码/token 不回显；
3. 无凭据选项且 stdin 非终端：报用法错误（退出码 1），不读管道内容；
4. 凭据来源按「选项 > 环境变量 > 交互提示」解析；凭据只在本进程内传递；
5. 密码登录调用 apiclient `login_password`；token 登录保存前用受保护只读接口验证（`GET /api/v1/projects` 200 即有效）；
6. 成功后经 credential_store 原子落盘并输出不含凭据的成功提示；失败输出类型化错误与退出码且不写凭据。

### 一般命令与 401 处理

1. 取 `SERVER`（显式/env/默认），未登录则退出码 2 并提示先 login；
2. apiclient `AuthClient::prepare` 注入 Bearer（token > session）；
3. session 凭据遇 401：若有 refresh_session 则续期一次并重试，仍失败退出码 2；token 401 直接退出码 2；
4. 4xx/5xx 按退出码表映射，不盲目重试。

## File-Level Modules

```rust
// cli/src/cli/args.rs：clap 命令面（参数校验与互斥在此集中）
pub fn parse(argv: &[OsString]) -> Result<CliArgs, ClapError>;
// cli/src/cli/mod.rs：App::run 分发与退出码映射
// cli/src/cli/login_handler.rs / publish_handler.rs / download_handler.rs / versions_handler.rs：
//   各命令编排（先校验参数 -> 调 apiclient/credential_store/archive -> 输出）
```

```rust
// cli/src/cli/login_handler.rs（fh-cli-login）
pub async fn run(args: LoginArgs) -> Result<i32, CliError>;
// cli/src/cli/publish_handler.rs（fh-cli-publish）
pub async fn run(args: PublishArgs) -> Result<i32, CliError>;
// cli/src/cli/download_handler.rs（fh-cli-download）
pub async fn run(args: DownloadArgs) -> Result<i32, CliError>;
// cli/src/cli/versions_handler.rs（fh-cli-versions）
pub async fn run(args: VersionsArgs) -> Result<i32, CliError>;
```

- Consumer: `cli/src/main.rs`（唯一入口，取 `App::run` 返回码）；handler 使用 apiclient / credential_store / archive 的公开接口
- Compatibility: new
- Migration path when required: 不适用（新命令面）

## Design Notes

- `<project>` 解析：经 `GET /api/v1/projects` 按 project 名精确匹配取 `project_id`；找不到给退出码 5/404 类错误；项目名唯一性由服务端契约保证，客户端仍对意外重复做防御性错误。
- 下载输出：`<project>-<version>.tar.gz`（archive 净化后），正文（stdout）只输出成功摘要/路径，不输出凭据。
- `--format json` 的 JSON 数组结构即服务端 `VersionDto[]` 字段；`text` 为固定列宽表格，字段顺序稳定。
- 帮助文本与退出码表保持同步；新增命令/参数必须先改 proposal/design（对外契约）。
