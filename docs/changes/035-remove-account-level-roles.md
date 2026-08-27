# 移除账号级 owner/member 角色，项目归属由创建者持有

- Status: complete
- Owner module: filehub（filehub-server permissions/account/http 子模块）
- Task manifest: docs/versions/v0.1/modules/filehub/035-remove-account-level-roles/task.yaml
- Approved proposal: docs/versions/v0.1/modules/filehub/035-remove-account-level-roles/proposal.md
- Affected paths: `server/src/model/role.rs`、`model/mod.rs`、`model/principal.rs`、`model/config.rs`、`server/src/permissions/{mod,checker,model}.rs`、`server/src/http/{auth,mod}.rs`、`server/src/contract/mod.rs`、`server/src/account/{authn,mod}.rs`、`server/src/tokens/http.rs`、`server/src/projects/service.rs`、`server/migrations/0003_roles_grants.sql`、`server/tests/`、`filehub-server.json`、`docs/api/v1-contract.md`、`docs/modules/filehub.md`、`docs/versions/v0.1/modules/filehub/001-filehub-core-platform/design/`
- Explicit tier override: none
- Expanded high-risk packet: none

## Approach

- 模型层：删除 `AccountRole` 枚举、`UserConfig.role` 字段与 `account_roles` 建表
  （0003 迁移只保留 `project_grants`）；`Principal::User` 收敛为 `{ user_id }`，
  auth 包装不再经 `role_for_user` 补角色。
- 权限判定：`Resource::Feature` 仅保留 `projects:create`——任意已登录 User 允许、
  Anonymous 拒绝、token 需显式 `projects:create` scope；`projects:delete` 移到
  项目级动作，owner 允许列表新增该动作，admin 协作者不可删除；token 删除沿用
  `administration` 项目级校验并新增 `projects:delete` 项目动作判定（scope + owner）。
- 服务层：`ProjectService::delete` 对 User 走项目级 owner 判定，对 token 先做
  `administration` 校验（保留范围外 not_found/forbidden 语义）再做项目级删除判定。
- 测试：移除账号角色用例与构造；member 创建项目改为 201 且 owner 为创建者；
  删除矩阵覆盖非 owner 用户、admin 协作者、缺 scope、范围外、非 owner token、
  owner 成功、缺失项目错误路径；配置文件与公共测试夹具去掉 `role`。
- 文档：权限矩阵/设计/model/http/account/testing/api 契约与模块说明同步为
  “任意账号可创建、删除仅项目 owner”。

## Risk Screen

- Public contract, protocol, or CLI change: no（端点/方法/状态码形状不变；`docs/api/v1-contract.md` 中 create/delete 两行的授权说明随行为更新，前端契约不消费账号角色）
- Persistent data, schema, or migration change: yes（greenfield 迁移 0003 不再创建 `account_roles`；`[users].role` 配置键删除；已运行过的库会残留未被引用的空表，不影响运行）
- Security, privacy, or trust-boundary change: yes（`projects:create` 对所有已登录账号开放是用户明确要求；`projects:delete` 同时由“账号级 owner 可删任意项目”收紧为“仅目标项目 owner 可删”，删除面不扩大）
- Concurrency, lifecycle, or runtime integration change: no
- Material dependency/build graph, supply-chain trust, produced artifact, production default/feature rollout, release/deployment, compatibility, or rollback impact: no（无依赖/构建/发布变更）
- Material UI, accessibility, localization, or navigation workflow change: no（admin-web 未改动，单测 44/44 通过）
- Harness rule, checker, or test-infrastructure change: no
- Cross-project or architectural boundary change: no

## Verification

- Targeted check: `cargo test -p filehub-server` 全量通过（unit_tests 38/38、
  api_integration 4/4、dv_tests 2/2，exit 0；lib 0）；`admin-web` `npm run
  test:unit` 44/44 通过；`Principal`/`AccountRole` 引用在 `server/src` 与实时文档中清零
- Result: pass
- Residual risk or follow-up: 存量库若曾运行 0003 会残留空 `account_roles` 表
  （本任务不引用、不 DROP，如需物理清理另立任务）；携带 `"role"` 键的旧配置文件
  启动时被 serde 拒绝（仓库内唯一配置与文档已同步）；repo 基线非 rustfmt 全量
  整洁，仅对任务内改动的 common/mod.rs 应用 rustfmt，其余改动行沿用既有文件风格，
  不做无关重排。
