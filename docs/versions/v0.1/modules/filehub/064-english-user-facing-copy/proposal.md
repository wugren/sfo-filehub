---
task_manifest: task.yaml
status: approved
---

# 将命令提示和运行时错误信息统一为英文

Risk profile: not-created（lower-tier 不创建 risk-profile）

## Workflow Tier Judgment

- Proposed tier: standard
- Final tier: standard
- Tier rationale / triggered boundaries:
  - 用户已明确排除管理后台语言、词典、语言切换和页面文案，任务不再涉及 UI 或可访问语言能力变化；
  - 改动集中在同一 `filehub` 模块的 CLI 命令帮助、交互提示、状态/错误输出，以及少量 server/构建脚本运行时信息；
  - CLI 人类可读输出属于公开使用面，范围又跨多个 CLI 子模块，不能满足 `trivial` 的局部无公共 CLI 影响条件，因此建议 `standard`；
  - 命令、参数、退出码、结构化 JSON、HTTP API、错误码、数据和安全边界均不改变，不构成需要完整分阶段流程的 `high-risk`。
- Proposal and tier confirmation: 2026-08-29 用户回复“确认”，批准本提案与
  `standard` 层级，并授权在不修改管理后台语言和页面文案的边界内完成实施、验证和验收。

## Approval Record

- approver: 用户
- approval_date: 2026-08-29
- user_statement: “确认”
- selected_tier: standard

## Background and Goal

当前 CLI 的命令帮助、交互提示、成功/失败信息和底层错误文本主要使用中文，构建脚本及少量服务端运行时错误/日志也可能向操作者输出中文。用户明确说明本任务只处理这类命令提示、错误信息和相邻运行时输出，不修改管理后台的语言能力或页面文案。

目标是将上述终端和运行时用户可见信息翻译为自然、准确且术语一致的英文，同时保留原有控制流、错误类别、插值数据、命令/API 契约和稳定退出码。

## Scope

### In scope

- 将 CLI 的 clap 命令说明、参数帮助、交互式输入提示和确认信息翻译为英文；
- 将 CLI 的成功输出、状态输出、日志、错误前缀，以及网络、认证、授权、输入、归档、完整性和本地文件系统错误信息翻译为英文；
- 将 server 中确实会进入 API 响应或运行日志的中文错误/状态信息翻译为英文；
- 将 `build-docker.sh` 实际输出到终端的中文提示翻译为英文；
- 更新直接断言上述输出文本的现有 CLI/server 测试；
- 对翻译后的占位符、路径、状态码、错误类别和退出码映射进行一致性验证，并扫描目标运行时字符串中的中文残留。

### Out of scope

- 不修改 admin-web 的中英文词典、默认语言、浏览器语言选择、语言偏好存储、语言切换入口、页面标题或任何页面文案；
- 不翻译源代码注释、Rustdoc、SQL 注释、测试用例说明、README、API/架构/历史任务文档或 YAML 示例注释；
- 不修改 Cargo/npm 包描述等不属于命令提示、错误信息或运行时终端输出的元数据；
- 不修改标识符、类型、函数名、命令名、参数名、环境变量、结构化 JSON/YAML 字段、HTTP 状态码、API 错误码或数据库内容；
- 不新增语言配置、国际化库或翻译基础设施；
- 不改变权限、认证、网络重试、文件处理、安全边界、CLI 退出码或服务端协议；
- 不触碰工作树中既有的 `Cargo.lock`、`harness/scripts/edit-guard.py`、未跟踪 `filehub-server.yaml` 和 `filehub.db`；
- 不把任务扩大为整个仓库所有中文字符清零，Harness/任务文档仍按项目自定义规则使用中文。

### Boundary with neighboring modules

管理后台完全不在本任务交付范围内。CLI 只改变人类可读输出，server 只改变会向客户端或操作者暴露的错误/日志文本，构建脚本只改变终端提示。API 数据形状、鉴权、持久化、发布产物和业务行为均不变化。

## Requirement Review

澄清后的需求合理。“所有中文文案”在本任务中按用户指定边界解释为命令提示、错误信息及同类运行时输出，而不是管理后台文案或源码内所有汉字。这样既能统一终端使用体验，也避免误删现有 Web 双语能力和工程说明。

实施时只翻译固定文本，保留格式化占位符、动态错误详情、HTTP 状态码和错误类别。对 CLI 自动化消费者，稳定接口仍是退出码与 `--format json`；不把翻译后的英文句子新增为机器稳定契约。

## Proposal Items

| proposal_id | change_id | requirement | boundary | tradeoff | success_evidence | non_goal |
|-------------|-----------|-------------|----------|----------|------------------|----------|
| P-001 | fh-english-cli-runtime-messages | 将 CLI 命令帮助、交互提示、状态/错误输出及相邻 server/构建运行时信息从中文翻译为英文 | 管理后台和工程说明完全排除；只改变人类可读固定文本及其测试期望 | 依赖精确中文文本的非正式脚本需改用稳定退出码或结构化输出 | CLI/server 目标测试通过；相关构建通过；运行时字符串扫描无未豁免中文；关键错误类别、插值和退出码映射保持一致 | 不改变 Web 文案、命令/API、退出码、权限、数据或行为 |

## Success Criteria

- Concrete user-visible or system-visible result:
  - CLI 帮助、交互提示、成功/状态输出、日志和所有可达错误信息使用英文；
  - server 返回或记录的纳入范围错误/状态信息使用英文；
  - `build-docker.sh` 的终端提示使用英文；
  - admin-web 的语言功能和页面显示与任务开始前完全一致。
- Required evidence:
  - CLI 单元/集成测试及构建通过，关键帮助、错误和成功输出断言使用英文；
  - server 受影响测试通过；
  - 对 `cli/src`、`server/src` 和 `build-docker.sh` 的字符串执行汉字扫描，逐项确认残留仅为注释、测试数据或其他明确排除内容；
  - 独立缺陷发现检查认证/授权/输入/网络/完整性错误类别、格式化占位符及稳定退出码未发生语义漂移；
  - diff 确认 `admin-web/**` 未被本任务修改。
- Explicit non-goals:
  - 不承诺源码注释、文档、管理后台或 Harness 任务记录为英文；
  - 不把人类可读英文文案定义为新的机器稳定契约。

## Risks

- 大量字符串替换容易遗漏占位符、标点或错误类别；需通过现有断言和目标字符串扫描共同发现。
- CLI 固定文案不是正式结构化契约，但外部脚本可能仍在解析中文文本；此次变更会打破这类非正式依赖，稳定机器消费路径仍是退出码与 JSON。
- server 的英文错误信息可能被 CLI 原样拼接，需要避免形成重复前缀或失去关键动态详情。
