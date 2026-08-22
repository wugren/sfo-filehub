---
task_manifest: task.yaml
status: approved
---

## Approval Record

- approver: user
- approval_date: 2026-08-21
- user_statement: 确认，自动完成

# Project 每个版本支持多个应用（versions multi-app）

Risk profile: ./risk-profile.yaml

## Workflow Tier Judgment
- Proposed tier: high-risk
- Final tier: high-risk
- Tier rationale / triggered boundaries:
  - 触发持久化 schema/migration：`version_files` 的“一版本一文件”关联（`version_id` 主键、`file_id` UNIQUE）需要调整为“一版本多应用”（version → app 1:N），涉及新 schema/迁移；既有存量数据兼容按要求不做。
  - 触发公共契约/CLI：v1 API 契约新增版本创建/锁定端点、发布改为“app 发布到已存在版本”、`VersionRecord` 形状、app 删除端点与下载参数都会变化，`filehub-cli` 与 `admin-web` 两个既有消费面必须同步改动。
  - 触发跨交付面/兼容性：server、cli、web 三面同次交付，版本显式创建是对既有“发布即建版本”流程的契约破缺，需要决策下载语义；歧义直接影响验收边界。
  - 结论：命中 high-risk 多项“已确认的实质性影响”（schema/migration、公共契约/CLI、兼容/回滚、跨面协调、验收歧义），默认按 high-risk 全生命周期呈报；用户可确认后改选 standard（轻量变更记录 + 完成报告），仓库规则要求先在此处明示风险。
- Proposal and tier confirmation:
  - 用户已六轮澄清需求：① 版本内含多个具名 app；② app 可独立发布到指定版本、可更新、可删除；③ 查询单版本返回全部 app 信息；④ 版本必须经接口显式创建，版本已存在时创建失败；⑤ 版本可锁定，锁定后不能再修改其中的 app 内容；⑥ 不处理既有数据兼容问题（不做存量回填/兼容迁移）。
  - 用户于 2026-08-21 回复“确认，自动完成”批准本提案，最终 tier 为 high-risk；`workflow_tier` 已写入 `high-risk`，本提案置为 `status: approved`，后续按 design → implementation → testing → acceptance 全流程自动执行。

## Background and Goal
- 现状：`versions` 以 `<project_id, version>` 唯一并在首次上传文件时隐式创建；`version_files` 以 `version_id` 为主键，每版本最多绑定唯一一个 `.tar.gz` 产物。没有“应用”概念，版本也没有独立创建入口。
- 目标：Project 中的版本成为显式实体，每个版本可承载多个具名“应用”（application），每个应用是版本内一个独立 `.tar.gz` 产物：
  - 版本经接口显式创建（`project_id + version` 唯一），重复创建同一版本返回 409 失败；
  - 应用独立发布到已存在的指定版本（首次发布创建该 app，重复发布即更新替换）；
  - 应用可从版本中删除；
  - 版本可被锁定，锁定后该版本内的 app 内容不可再修改（不能发布、更新或删除 app），读取与下载不受影响；
  - 查询单个版本返回该版本包含的全部 app 信息（app 名、file_id、sha256、size、更新时间）；
  - 下载按应用区分，同时保留单应用版本的既有使用方式。

## Scope
### In scope
- 数据模型与迁移：
  - 将版本-文件关联调整为 1:N：`version_apps(version_id, app, file_id, sha256, size, created_at, updated_at)`，`UNIQUE(version_id, app)`，`file_id` 保持唯一。
  - `versions` 表 `UNIQUE(project_id, version)` 语义不变，但版本行改为显式创建实体（无 app 也可存在），并增加可空的 `locked_at` 字段表示锁定状态。
  - 不做既有数据兼容：不使用旧 `version_files` 存量数据回填，旧表结构直接由新 schema 取代；当前 `data/filehub.db` 实测 0 条版本数据（1 个 project），如部署环境存在旧数据由部署方按新 schema 重建，本次不提供兼容迁移。
  - `VersionRecord` 调整为 `{project_id, version, published_at, apps: [{app, file_id, sha256, size, updated_at}]}`；显式创建版本时 `published_at = created_at`，`apps` 为空数组。
- 服务与 API（`versions` 子模块与 v1 契约）：
  - 新增版本创建端点 `POST /api/v1/projects/{id}/versions`（JSON body `{version}`）：创建版本元数据，201；`(project_id, version)` 已存在时 409 创建失败；版本不属于 app 发布流程，创建时不接收文件。
  - app 发布/更新端点 `PUT /api/v1/projects/{id}/versions/{version}/apps/{app}`（multipart：`file` + 可选 `sha256`）：版本不存在 → 404；app 不存在 → 创建（201）；app 已存在 → 更新替换该 app 的文件并刷新 sha256/size/updated_at（200），旧文件引用解除后由 files 孤儿回收清理。
  - app 删除端点 `DELETE /api/v1/projects/{id}/versions/{version}/apps/{app}`：从版本中移除该 app（204）；权限沿用 `artifacts:write`/Project 粒度；版本行不受影响（显式实体，app 删光后仍保留）。
  - `GET /versions`、`GET /versions/{version}` 返回按版本聚合的 `apps[]`；查询单个 version 必须返回该版本全部 app 信息；显式存在但无 app 的版本返回空 `apps`。
  - `GET /versions/{version}/download?app=<name>`：按应用下载对应 `.tar.gz`；`app` 缺省时单应用版本保持兼容下载，多应用版本返回 422 要求显式指定，目标 app 或版本不存在返回 404。
  - `latest` 语义保持为“按版本发布时间倒序最近一次”的版本行；其 `apps` 按实际内容返回。
  - 新增版本锁定端点 `PUT /api/v1/projects/{id}/versions/{version}/lock`（需 Project 级 `administration`）：锁定后 `locked_at` 写入并于响应中返回；对已锁定版本重复锁定幂等成功。锁定后 `PUT .../apps/{app}` 与 `DELETE .../apps/{app}` 一律返回 409（版本已锁定），`GET`/下载不受影响。
- 消费面同步：
  - `filehub-cli`：新增创建版本、锁定版本命令；`publish` 增加 `--app`（发布到已存在版本，重复即更新）；`download` 增加 `--app`；`versions` 输出展示应用列表与锁定状态；提供删除 app 命令；对锁定版本执行发布/删除返回明确错误（命令形态在 design 冻结）。
  - `admin-web`：ProjectDetailPage 调整为“先创建版本、再向版本上传/更新/删除 app，可锁定版本”；版本行展示每个 app 的大小/SHA-256/更新时间/下载入口与删除操作（带确认提示），锁定后禁用新增/更新/删除并显示锁定标记；重复创建版本与对锁定版本操作时展示 409 错误。
  - `docs/api/v1-contract.md`、design/versions、`docs/modules/filehub.md` 边界描述同步。
- 验证：server 单测/dv/integration、cli 测试、admin-web build/unit/integration 全绿，并对版本显式创建/409、多 app 发布/更新/删除/下载/兼容路径做端到端验证。
### Out of scope
- 不做版本删除端点、应用级权限/可见性管理（仍以 Project 粒度授权）、app 重命名（可通过删除后重新发布达成）。
- 不处理既有存量数据的兼容迁移/回填（用户明确划出范围）。
- 锁定为不可逆终态：不提供解锁端点（用户已确认：不要可逆）。
- 不做“整版本合并打包下载”。
- 不改 files 物理存储/原子写入/流式下载机制本身，不改认证与会话模型，不新增依赖。
### Boundary with neighboring modules
- `versions` 拥有版本实体与 `version_apps` 关联，独占创建/发布/更新/删除编排；`files` 仍拥有物理字节、下载流转发与孤儿回收（`referenced_file_ids` 语义随 app 引用变化）；CLI/Web 只经 `docs/api/v1-contract.md` 交互；权限仍停留在 Project 层级。

## Requirement Review
- 需求合理且语义已充分收敛：版本显式创建符合“版本是独立发布单元、app 是其子产物”的实体建模；创建重复版本 409 是标准资源冲突语义。
- 材料权衡与选定方向：
  - 版本先建、app 后发布（用户已确认）：`POST /versions` 专职版本创建，`PUT /versions/{version}/apps/{app}` 专职 app 发布/更新。发布到不存在版本返回 404，把“写错版本号”从隐式建版本改为显式失败。
  - 具名 `app`（用户已确认）而非匿名文件列表：稳定唯一标识、可解释的更新/删除目标、下载选择与审计友好。
  - 发布与更新同一 PUT（幂等 upsert）：首次创建、重复替换，符合“可发布到指定版本也可更新”的需求，无需额外显式标记。
  - 删除 app 不删除版本（用户显式创建版本语义的自然推论）：版本是独立实体，空版本仍可查询与继续接收 app。
  - 按 app 单独下载（用户已确认）而非版本整包下载：服务端无需二次打包，物理存储不变；不做整版本合并下载。
  - 版本锁定作为一成不变的发布语义（用户已确认不可逆）：锁定后版本内的 app 内容冻结，契约上所有写操作返回 409；不提供解锁端点；锁定动作需要 Project 级 `administration`，防止普通协作者误锁/绕过锁定。
  - 契约形状一次性破缺到 `apps[]`，不同时保留顶层单文件字段：仓库尚无正式发布版本（git 无 commit、无对外发布），三个交付面同次交付闭环可吸收此变更。
  - 不做存量数据兼容（用户已确认）：省去回填/迁移路径，把“旧库如何变新”留给部署方；当前库无版本数据，风险可控。
- 已确认项：
  - Q1 具名 app（第一轮澄清）；Q3 查询单版本返回全部 app 信息（第三轮澄清）；Q4 版本为显式实体、删除 app 不影响版本行；Q5 重复发布同 app 即更新（PUT upsert 语义）；Q2 下载按 app 单独进行、不做整版本合并下载；Q6 锁定不可逆、不提供解锁。
  - Q7 不处理既有数据兼容问题（本轮澄清）：不做存量回填、不做兼容迁移，schema 直接按新模型重建。
- 待用户在确认时答复的未决问题：无（六轮澄清后语义与边界均已确定；剩余仅确认 tier 并批准执行）。

## Proposal Items
| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-versions-multi-app-model | 版本-文件关联改为 version→app 1:N：新 schema + `VersionRecord.apps[]`；版本显式创建（重复 409）、`locked_at` 状态与不可逆锁定操作、app 创建/更新（UPSERT）、app 删除；锁定后写操作全拒；不做存量回填/兼容迁移 | `versions` 子模块 + `model/record.rs` + migrations；物理文件清理交给 files 孤儿回收 | `UNIQUE(project_id, version)` 与 `UNIQUE(version_id, app)`；更新/删除/锁定后旧文件脱离引用集由 `files.gc_orphans` 回收；存量数据兼容由部署方重建，不属本次交付 | 显式创建版本后列表/查询可见；重复创建 409；同版本发布会话 app 成功且重复发布更新 sha256/size/updated_at；锁定后发布/更新/删除均被拒；server 单元/集成测试通过 | 不做版本删除端点/app 重命名/独立权限/解锁/存量回填 |
| P-002 | fh-versions-multi-app-api | v1 契约：版本显式创建（409）；锁定端点；`PUT .../versions/{version}/apps/{app}` 发布/更新（版本不存在 404、锁定 409）；app 删除端点（锁定 409）；list/get 返回 `apps[]` 与 `locked_at`；download 支持 `?app=`，缺省单 app 兼容、多 app 422 | `server/src/versions/http.rs`、`docs/api/v1-contract.md`；权限仍按 Project 粒度 | 创建/锁定/发布职责分离，接口意图清晰；PUT 幂等承载发布/更新；锁定在服务层统一拦截写操作 | 契约文档与实现一致；dv/integration 覆盖创建/409、锁定、404、发布、更新、删除、latest、按 app 下载、空版本、缺省/422 路径 | 不新增整版本打包下载，不改认证/权限模型 |
| P-003 | fh-cli-multi-app | CLI 新增创建版本、锁定版本命令；`publish --app` 发布到已存在版本（缺省 `default`，重复=更新）；`download --app`；删除 app 命令；`versions` 显示 app 列表与锁定状态；锁定版本上的发布/删除报错清晰 | `cli/src/`、`cli/tests/` | 版本必须先行创建，旧“发布即建版本”流程被替换；命令形态在 design 冻结 | 创建版本成功/409；锁定成功且状态可见；两次不同 `--app` 发布到同版本成功且列表可见；重复 `--app` 更新成功；锁定后发布/删除被拒且退出码明确；`--app` 下载 SHA-256 校验通过；cli 测试全绿 | 不改凭据/配置/其他命令语义 |
| P-004 | fh-web-multi-app | admin-web 版本详情调整为“创建版本 → 按 app 上传/更新/删除 → 可锁定”；展示每个 app 的 size/SHA-256/更新时间/下载入口；锁定后禁用写操作并显示锁定标记；重复创建/对锁定版本操作展示 409 | `admin-web/src/`、`admin-web/tests/` | 发布流程由“上传即建版本”改为显式两步；锁定状态贯穿版本行与操作可用性 | `npm run build`/`test:unit`/`test:integration` 通过；创建版本、按 app 上传/更新/删除、锁定、下载交互可用 | 不改导航与其他页面流程 |

## Success Criteria
- Concrete user-visible or system-visible result:
  - 版本必须先经接口创建：创建成功后列表/查询可见；重复创建同一版本返回 409 且不产生副作用。
  - 同一 Project 版本可发布多个具名应用；查询某个 version 的响应包含该版本全部 app 信息（app 名、file_id、sha256、size、updated_at）。
  - 对同一 version+app 重新发布即更新该应用产物；删除 app 后列表不再显示且下载 404；空版本（无 app）仍可查询并继续发布。
  - 锁定版本后：列表/详情显示锁定状态；向锁定版本发布、更新或删除 app 均失败（409）；读取与该版本下载不受影响。
  - 未带 `app` 发布仍得到 `default` 应用；多应用版本下载未指定 `app` 时收到明确 422。
- Required evidence:
  - schema 直接按新模型重建，无存量回填要求；如目标环境存在旧库由部署方先重建（本次不提供兼容迁移）。
  - `./test-run.sh`（server 全部单元/dv/集成）、`cli` 测试、`admin-web` build + unit + integration 全绿。
  - 端到端 curl/CLI 演示：创建版本 v1/v2→v1 重复创建 409→向 v1 发布 app-a/app-b→`GET /versions/v1` apps 含 2 项完整信息→更新 app-a 后 sha256/size 变化且旧文件进入孤儿回收→锁定 v1→向 v1 发布/更新/删除 app 均 409、读取与下载仍 200→在未锁定的 v2 上删除 app-b 后其下载 404→向不存在版本发布 404→多 app 缺省下载 422；单 app 版本缺省下载仍 200。
  - `docs/api/v1-contract.md` 与 design/versions 更新后与实现一致；生命周期文档（design/implementation/testing/acceptance）齐全（high-risk）并全部通过检查。
- Explicit non-goals:
  - 不做版本删除端点、app 级权限、重命名、整版本合并打包；不改文件存储与认证模型；不引入新依赖。

## Risks
- schema/迁移风险：schema 直接重建、不做存量兼容；存在旧数据的部署环境会丢失旧结构数据或需部署方先行重建，属用户明确接受的范围（非本次交付）。
- 更新/删除的数据丢失风险：更新替换与删除会解除旧文件引用，旧文件随后由孤儿回收物理删除；服务层必须先落库成功再解除引用，并在测试覆盖“更新失败不丢旧文件”路径。
- 公共契约/兼容风险：版本显式创建是对既有“上传即建版本”流程的契约破缺，CLI/Web 发布流程与旧测试断言需同步改写；仓库无正式发布版本，三面同批交付可吸收，但契约文档需标注 v1 破缺。
- 语义风险：`latest` 返回最近创建的版本行，可能为空版本（无 app）；下载目标 app 或版本不存在均返回 404；并发创建同版本时靠 `UNIQUE(project_id, version)` 与 409 兜底。
- 锁定并发/终态风险：锁定与发布/删除在服务层以事务/顺序检查保证原子性；锁定不可逆，一旦确认即终态，契约与文档需明示该承诺，避免误锁。
