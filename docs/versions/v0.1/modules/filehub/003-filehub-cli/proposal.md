---
status: approved
task_manifest: task.yaml
approved_by: user
approved_at: 2026-08-19
approved_content_sha256: 2c7c515e274dff9f21ea24eb4c7e7e48024c1666a633d53c148aa07fc0aea794
---

## Approval Record

- approver: user
- approval_date: 2026-08-19
- user_statement: 确认，按本次修订后的提案定稿（2026-08-19），进入设计阶段


# filehub 发布客户端（filehub-cli）提案

## Workflow Tier Judgment

- Proposed tier: high-risk
- Final tier: high-risk
  - 确认记录：2026-08-19 当前用户回复「确认」，按本次修订后的提案定稿——CLI 命令面按默认命令集定稿，login 参数定义（含环境变量输入通道与「显式选项 > 环境变量 > 交互提示」优先级）按新增小节定稿；最终层级 high-risk，进入设计阶段。
- Risk profile: ./risk-profile.yaml
- 触发边界/理由：发布客户端是面向用户的公开 CLI 产品，承担账号密码/token 登录、本地凭据存储、把文件或目录发布为服务端版本的对外合约；命中公开 CLI 合约、凭据安全与新产物发布面等实质性风险类别，按 high-risk 全流程（提案 -> 设计 -> 实现 -> 测试 -> 验收）执行。
- 拆分说明：本任务是三模块实现中的“发布客户端”模块；服务后台与页面分别由兄弟任务 `001-filehub-core-platform`、`002-filehub-web` 承担。
- 本次修订（2026-08-19）：按当前用户要求补充 `filehub login` 登录参数与输入方式的详细定义（见 Scope「`filehub login` 参数定义」与 P-01）；层级维持 high-risk，修订后的提案需重新获得用户确认。
- 确认陈述：本提案需获得当前用户明确确认后才能进入执行；用户可选择按本提案确认、以替换层级（trivial/standard/high-risk）确认，或要求修订提案。

## Background and Goal

filehub 需要一个版本发布工具（`filehub-cli`）：支持账号密码登录和 token 登录，登录后可以指定一个文件或目录进行发布，也可以把指定项目的指定版本下载到本地目录，或把指定项目的版本信息查询结果输出到指定位置。登录与发布交互方式参考 Docker CLI（`docker login` / `docker push` 的形态）：命令式交互、凭据保存在本机受保护位置、发布前自动验权。

## Scope

### In scope

1. 登录与会话/凭据
   - `filehub login`：支持账号密码登录与 token 登录两种方式；命令面与参数详见下方「`filehub login` 参数定义」；
   - 登录凭据保存在本机受保护位置（参考 `~/.docker/config.json` 的本地凭据存储模式），支持 `logout` 清除；
   - 凭据可被后续命令自动复用，不随命令参数明文传递。
2. 发布
   - `filehub publish <文件或目录> <project>:<version>`：登录后把文件或目录发布为指定项目的指定版本；
   - 发布内容统一封装为 `.tar.gz`（单个文件与目录相同），服务端只接受 `.tar.gz`；
   - 版本不可覆盖：同一 `<project>:<version>` 已存在时服务端拒绝（HTTP 409），客户端明确提示版本已存在并建议使用新版本号；
   - 发布前自动做服务端权限校验（write 权限）；失败时给出明确的退出码与错误信息。
3. 下载与版本查询（面向脚本）
   - `filehub download <project>[:<version>] -o <目录>`：把指定项目的指定版本 `.tar.gz` 归档下载到指定目录；省略 `<version>` 时下载该项目的最新版本（最新版 = 最近发布的版本，按发布时间倒序取最近一次发布，与 `001-filehub-server` 的 latest 语义一致）；
   - `filehub versions <project> -o <路径>`：查询指定项目的版本信息（版本号、发布时间、大小、SHA-256 等）并写入指定位置；省略 `-o` 时输出到 stdout；支持文本与 JSON 格式；
   - 稳定的退出码、`--help` 与错误输出，便于 CI/脚本集成。
4. 跨平台交付
   - 单二进制 CLI，覆盖 Windows/macOS/Linux；
   - 可通过本地 Harness 测试与打包脚本验证。

### `filehub login` 参数定义（本次修订）

语法：`filehub login [SERVER] [选项]`

| 参数/选项 | 必需 | 说明 |
|---|---|---|
| `SERVER`（位置参数） | 否 | filehub 服务地址（`https://host[:port]`）；缺省时使用默认服务器地址（首版为单一服务器，配置文件中可保存），位置参数形态与 `docker login [SERVER]` 一致 |
| `-u, --username <USER>` | 否 | 账号密码登录的用户名；缺省时交互式提示输入 |
| `--password-stdin` | 否 | 账号密码登录：密码从 stdin 读取（剥离末尾换行），供脚本/CI 使用；与 token 登录选项互斥 |
| `--token-stdin` | 否 | token 登录：token（JWT）从 stdin 读取，供脚本/CI 使用；与账号密码登录选项互斥 |
| `--config <PATH>` | 否 | 覆盖凭据与配置文件路径（默认类 Unix `~/.config/filehub/config.toml`，权限 `0600`；Windows/macOS 使用对应用户配置目录），便于测试与多环境隔离 |
| `-h, --help` | 否 | 显示帮助 |

环境变量（建议补充的脚本/CI 输入通道，与下表一并确认）：`FILEHUB_SERVER`（默认服务地址）、`FILEHUB_USERNAME`（默认用户名）、`FILEHUB_PASSWORD`（账号密码登录密码）、`FILEHUB_TOKEN`（token 登录凭据）、`FILEHUB_CONFIG`（默认配置文件路径）。优先级：显式命令行选项 > 环境变量 > 交互式提示；环境变量同样不允许日志输出。

行为约定：

- 交互模式：stdin 为终端且未给 `--password-stdin`/`--token-stdin` 时，提示输入；未指定登录方式时先提示选择账号密码或 token，密码/token 输入不回显；
- 非交互模式：stdin 非终端时必须显式使用 `--password-stdin` 或 `--token-stdin`，否则报用法错误并给出稳定退出码，避免管道内容被误当凭据或静默失败；
- 互斥校验：账号密码与 token 两种登录模式的选项同时出现（如 `--username` 搭配 `--token-stdin`）时解析失败，给出用法错误与稳定退出码；
- 凭据不进入命令参数：不提供 `--password <明文>` / `--token <明文>` 选项，密码与 token 只经交互输入、stdin 或环境变量进入，避免进入 shell 历史、`ps` 进程列表与日志；
- 登录成功后的保存与复用：
  - 账号密码登录：调用服务端 `POST /account/login`，本地保存返回的 session 与 refresh_session；后续请求携带 `Authorization: Bearer <session>`；session 失效（401）时自动用 refresh_session 调 `/account/refresh_session` 续期并更新本地凭据，续期失败则提示重新登录；
  - token 登录：本地保存 token，后续请求携带 `Authorization: Bearer <token>`；登录时调用受保护只读接口（如 `GET /api/v1/projects`）验证 token 有效性，token 无效不发版、不写凭据；
  - 凭据复用优先级：token > 登录 session（本机同时存在两者时优先使用 token）；
  - 重新 login 覆盖当前服务器已有凭据；`filehub logout` 清除该服务器的全部本地凭据；
- 失败行为：用户名/密码错误、token 无效/已撤销、服务不可达均给出明确错误信息与稳定退出码，且不写入凭据文件；
- 输出限制：成功/失败提示与日志不得包含密码、token、session 明文（配合 `sfo-log` 日志脱敏）。

### Out of scope / non-goals

- 服务端认证、授权、版本与产物 API 的实现（归属 `001-filehub-server`）；
- 管理后台页面（归属 `002-filehub-web`）；
- 镜像 registry/layer 协议（只参考 Docker 的交互与凭据形态，不复刻协议）；
- 安装器/自动更新/签名分发（首版只交付可执行二进制与打包流程）；
- 断点续传/分片上传的协议设计；
- 下载时自动解压归档（首版下载仅保存 `.tar.gz` 文件本身，解压由用户/后续任务处理）；
- 版本覆盖/重写：版本一经发布不可修改（服务端模型决定），客户端不提供覆盖发布语义；
- `--password <明文>` / `--token <明文>` 命令行参数形态（凭据安全考虑，明文参数会进入 shell 历史与进程列表；见「`filehub login` 参数定义」）；
- 交互式 TUI、图形界面。

### 相邻边界

- 客户端不在本地做最终授权判断：写权限由服务端返回 401/403，客户端只负责呈现与重试提示；
- `.tar.gz` 打包时排除绝对路径与越界符号链接，防止服务端收到不安全归档；
- 下载到目录时对文件名做安全净化，防止路径穿越写入任意位置；版本查询输出到指定路径时同样校验路径安全；
- 凭据文件权限按本机最小权限设置（如类 Unix `0600`）。

## 实现模块拆分（Implementation Module Split）

三模块实现由用户确认：

1. `001-filehub-core-platform`（服务后台 `filehub-server`）：认证/授权、项目/版本/产物 API，不包含前端托管；
2. `002-filehub-web`（页面/管理后台）：React 管理页面；
3. `003-filehub-cli`（本任务，发布客户端）：跨平台 CLI。

本任务与 `001-filehub-server` 只通过公开 v1 API 契约交互。

## Requirement Review

需求合理：Docker-like 的“登录存凭据、发布前验权、命令式交互”非常适合文件集散场景，和 `docker login` / `docker push` 的用户心智一致。

关键取舍与建议方向：

- 凭据存储参考 Docker：类 Unix 下放 `~/.config/filehub/config.toml`（权限 `0600`），Windows/macOS 下采用对应用户配置目录；token 优先于会话凭证复用；
- login 参数与输入方式（本次修订）：密码/token 只经交互输入、stdin 或环境变量进入，不定义明文命令行参数，避免进程参数/shell 历史泄漏；脚本/CI 使用 `--password-stdin`/`--token-stdin`（建议同时支持对应环境变量，见参数定义）；
- 统一 `.tar.gz`：客户端负责本地打包并在上传前做内容安全检查（路径穿越/符号链接越界）；
- 日志约束（用户已确认）：CLI 作为 Rust 项目统一使用 `sfo-log` 输出日志（可用 `nolog` 特性关闭）；
- 版本号由用户显式给出（`<project>:<version>`），服务端负责唯一性与更新策略。

### 待确认问题（Open questions）

用户已确认：技术栈（跨平台 CLI）、发布格式统一 `.tar.gz`、token 过期策略不收紧、三模块独立建任务。

用户已确认的日志约束：Rust 项目日志统一使用 `sfo-log` 库。

以下待确认项已于 2026-08-19 由当前用户回复「确认」一并定稿：

- CLI 命令面命名：按默认命令集定稿——`filehub login`（密码/token）与 `filehub logout`、`filehub publish <文件或目录> <project>:<version>`、`filehub download <project>[:<version>] -o <目录>`（省略版本时下载最近发布的版本）、`filehub versions <project> -o <路径>`（省略 `-o` 输出到 stdout）；
- login 参数与输入方式：按上文「`filehub login` 参数定义」定稿，保留环境变量输入通道（`FILEHUB_SERVER`/`FILEHUB_USERNAME`/`FILEHUB_PASSWORD`/`FILEHUB_TOKEN`/`FILEHUB_CONFIG`），优先级为显式命令行选项 > 环境变量 > 交互式提示。

## Proposal Items

每个提案项均给出稳定 `proposal_id` 与实现侧 `change_id`，后续设计/测试/验收按 `change_id` 追踪。

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|---|---|---|---|---|---|---|
| P-01 | fh-cli-login | 账号密码/token 两种 `filehub login` 与 `logout`；参数按「`filehub login` 参数定义」；本地最小权限凭据存储、自动复用与 session 续期 | 只做凭据获取/保存/清除与复用，不做服务端授权判定 | token 优先于 session 复用；凭据只经交互输入、stdin 或环境变量进入，明文不入命令行参数 | 两种模式、模式互斥、交互/非交互、续期、logout 均有正反例可验证 | 不做交互式 TUI/图形界面、不做服务端认证实现 |
| P-02 | fh-cli-publish | `filehub publish <文件或目录> <project>:<version>` 统一 `.tar.gz` 发布、同版本 409 不覆盖、发布前验权 | 服务端版本/产物 API 归属 001；打包在客户端完成安全裁剪 | 上传前验权减少半成品发布；版本号由用户显式给出 | 发布成功/409/无权限/打包安全正反例可验证 | 不做断点续传、分片上传的协议设计 |
| P-03 | fh-cli-download | `filehub download <project>[:<version>] -o <目录>` 下载 `.tar.gz` 并校验 SHA-256、文件名净化防穿越 | 下载仅保存归档本身，不自动解压 | 缺省版本时与服务端 latest 语义保持一致 | 哈希一致、latest、路径防穿越正反例可验证 | 不实现断点续传与自动解压 |
| P-04 | fh-cli-versions | `filehub versions <project> -o <路径>` 文本/JSON 输出到指定路径或 stdout、路径安全校验 | 输出字段契约由 001 服务端 API 定义 | 面向脚本优先，输出格式稳定 | 文本/JSON 与服务端一致、stdout/文件输出正反例可验证 | 不做在线编辑与交互式浏览 |

### P-01 fh-cli-login：登录与本地凭据存储

- 密码/token 两种 `filehub login` 登录方式与 `logout` 清除；命令参数按「`filehub login` 参数定义」执行（`SERVER`/`-u`/`--password-stdin`/`--token-stdin`/`--config`，两模式互斥，密码与 token 不明文进入命令行参数）；
- 账号密码登录保存 session + refresh_session 并支持 `/account/refresh_session` 自动续期；token 登录保存 token 并在登录时验证有效性；本机复用优先级 token > session；
- 本地凭据文件（最小权限，类 Unix `0600`）与自动复用；`logout` 清除当前服务器全部本地凭据；凭据不明文泄漏到进程参数/日志；登录失败不写凭据文件。

### P-02 fh-cli-publish：文件/目录统一 `.tar.gz` 发布

- `filehub publish <文件或目录> <project>:<version>`；同一版本已存在时返回明确冲突错误，不覆盖既有版本；
- 客户端打包安全 `.tar.gz`、上传前服务端验权、明确错误/退出码。

### P-03 fh-cli-download：指定/最新版本下载到指定目录

- `filehub download <project>[:<version>] -o <目录>`：把 `.tar.gz` 归档下载到指定目录并校验内容（SHA-256 与服务端一致）；省略版本时下载该项目最新版本（最近发布的版本）；
- 目标目录可写校验、文件名安全净化，防止路径穿越；
- 稳定退出码与 `--help`，便于脚本化。

### P-04 fh-cli-versions：版本信息查询与输出

- 查询指定项目的版本信息并以文本/JSON 格式写入指定位置（或 stdout）；
- 输出内容与服务后台版本信息 API 一致；路径安全校验。

## Success Criteria

可见结果与必须的证据：

1. `filehub login` 可分别用账号密码与 token 登录并保存本机凭据：交互输入、`--password-stdin`/`--token-stdin`（及确认后的环境变量通道）均可完成登录；模式选项互斥、无效凭据给出明确错误与稳定退出码；登录与日志产物中不含密码/token/session 明文；session 失效后可用 refresh_session 续期；`logout` 清除后命令要求重新登录；
2. `filehub publish` 能把本地文件与目录发布为 `<project>:<version>`；发布内容为统一 `.tar.gz`，服务端可下载且内容校验一致；同一版本号重复发布被拒绝（409），既有版本内容不变；
3. 无权限 token/过期 token/已撤销 token 发布时被服务端拒绝，客户端给出明确错误与退出码；
4. `filehub download <project>[:<version>] -o <目录>` 可用：归档写入指定目录，下载内容哈希与服务端一致；省略版本时下载到最新版本且与服务端 latest 语义一致；
5. `filehub versions <project> -o <路径>` 可用：版本信息写入指定位置且与服务后台 API 一致；省略 `-o` 时正常输出到 stdout；
6. 三平台（Windows/macOS/Linux）可构建出单二进制，本地 Harness 测试覆盖打包与命令正反例；
7. 交付证据：CLI 自动化测试经仓库 `test-run.sh`/`test-run.py` 可运行；与服务后台 API 契约一致的契约测试通过；high-risk 全生命周期文档齐全并逐级校验通过。

非目标成功证据：本任务不验收服务端权限实现本身，也不验收管理后台页面。

## Risks

- 凭据安全（高）：登录 token/密码一旦写入日志、进程参数、环境变量或权限过宽的配置文件即为泄露；设计阶段明确凭据文件权限、输入方式（交互式/stdin/环境变量/受保护文件）与日志脱敏，并验证 `--config` 等路径覆盖不降低默认权限。
- 对外 CLI 合约（高）：命令、参数与退出码一旦发布即承担兼容负担；命令面在提案确认时定稿，设计阶段再冻结细节。
- 外部依赖（低）：`sfo-log` 的来源与版本在设计阶段锁定，避免供应链漂移。
- 归档安全（中）：打包 `.tar.gz` 时需排除路径穿越、越界符号链接等不安全内容，服务端再校验一遍。
- 网络与服务端可用性（中）：上传失败/超时的重试策略与幂等性在设计阶段明确；已存在的版本号直接以 409 拒绝，不做覆盖/重写。
- 跨平台差异（中）：凭据目录、路径与归档行为在 Windows/macOS/Linux 的差异纳入测试矩阵。
