# filehub-cli（`cli/`）

filehub 文件集散发布客户端：登录后发布/下载/查询项目版本，支持密码与 token
登录、本地最小权限凭据存储、统一 `.tar.gz` push/pull 与稳定退出码。

## 构建

```bash
cargo build --release --manifest-path cli/Cargo.toml
# 产物：cli/target/release/filehub
```

跨平台单二进制：Windows/macOS/Linux 各自在目标平台执行上述命令（或
`cargo build --release --target <triple>` 交叉构建）；依赖经 `cli/Cargo.lock`
锁定，TLS 使用 rustls 内置根证书，不依赖系统 TLS 链变异。

## 命令面

| 命令 | 说明 |
|------|------|
| `filehub login [SERVER] [-u <USER>] [--password-stdin \| --token-stdin] [--config <PATH>]` | 密码或 token 登录并保存本地凭据 |
| `filehub logout [SERVER] [--config <PATH>]` | 清除指定/默认服务器凭据 |
| `filehub push <server/project/version/name> <文件或目录> [--config <PATH>]` | 安全打包为 `.tar.gz` 并发布为指定版本内 `<name>` 应用 |
| `filehub pull <server/project/version/name> <输出文件路径> [--config <PATH>]` | 按 `<name>` 下载归档到精确文件路径，校验 SHA-256 后原子落盘 |
| `filehub versions <server/project> [-o <路径>] [--format text\|json] [--config <PATH>]` | 查询版本信息 |
| `filehub new-version <server/project/version> [--config <PATH>]` | 显式创建项目版本（已存在时失败） |
| `filehub lock-version <server/project/version> [--config <PATH>]` | 不可逆锁定项目版本 |
| `filehub delete-app <server/project/version/name> [--config <PATH>]` | 从项目版本中删除指定应用 |

目标串统一为 `<server/project/version/name>` 前缀形态（`versions` 为
`<server/project>`）；`server` 为 `host[:port]`，端口/IPv6 冒号保留，
目标字段以 `/` 分隔，缺段、空段与多余段都会报输入错误；`http(s)://` 前缀
仍按旧习惯接受，身份统一归一化为 `host[:port]`。
旧命令 `publish`/`download` 已移除，改为 `push`/`pull`。

凭据输入优先级：显式命令行选项 > 环境变量 > 交互提示；密码/token 不明文进入
命令行参数。环境变量：`FILEHUB_SERVER`、`FILEHUB_USERNAME`、
`FILEHUB_PASSWORD`、`FILEHUB_TOKEN`、`FILEHUB_CONFIG`。

## 退出码

| 退出码 | 类别 |
|--------|------|
| 0 | 成功（含 `--help`） |
| 1 | 用法/参数错误 |
| 2 | 认证失败（登录失败、401、续期失败、无可用凭据） |
| 3 | 授权失败（403） |
| 4 | 冲突（409） |
| 5 | 输入无效（422、项目解析不到、版本格式非法） |
| 6 | 网络/传输（超时、连接、TLS、5xx） |
| 7 | 内容/完整性（SHA-256 不一致、归档不安全） |
| 8 | 本地文件系统（目标目录不可写、配置损坏等） |

## 本地验证

```bash
UV_CACHE_DIR=.harness/uv-cache uv run --active python ./harness/scripts/test-run.py filehub/003-filehub-cli all
```

参考资料：`docs/versions/v0.1/modules/filehub/003-filehub-cli/`（提案/设计/测试）
与 `docs/api/v1-contract.md`（v1 API 契约）。
