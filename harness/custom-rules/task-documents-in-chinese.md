# 任务文档中文描述规则 (Task Documents in Chinese)

## Goal
- 新增的任务文档必须用中文描述，保证需求、范围、决策和验收结论对项目成员可读、无歧义。
- 本规则为项目自有自定义规则，路由时优先于生成的 Harness 规则输出；不替代任何生成的机械检查。

## Scope
- 适用文档：本规则生效后新建的任务文档，包括：
  - 所有层级任务包内的 `proposal.md` 和 `completion-report.md`；
  - high-risk 任务包内的 `design.md`、任务本地 `design/` 说明文档、`testing.md` 和 `acceptance-report.md`；
  - standard 层级的 `docs/changes/<change>.md`；
  - 任务包内新增的其他人工可读说明文档。
- 适用条件：trivial、standard、high-risk 三个层级；proposal、design、implementation、testing、acceptance 以及 general 路由阶段；manual 和 auto-pipeline 两种模式。
- 不追溯强制重写存量任务文档；机器生成或机器校验的文件不改写其固定结构。

## Required Content
- 描述性正文必须使用中文，包括但不限于：背景与目标、范围与非目标、需求/提案项描述、风险与权衡说明、设计覆盖说明、测试与验收说明、变更记录说明、缺陷发现记录和结论。
- 保留模板与检查器要求的固定英文结构和值，不强制翻译：
  - 机械检查依赖的固定章节标题、字段标签和值，例如 `## Independent Defect Discovery`、`## Conclusion`、`## Verification`、`Status: complete`、`- Result: pass`、`workflow_tier`、`status: approved` 等；
  - 稳定标识如 `change_id`、`proposal_id`；文件路径、代码/命令/模块名、YAML 键；
  - 通用技术术语和专有名词（如 KYC、CI/CD、Harness、API、GitHub）可以保留英文。
- 句子主体应为中文；允许专业术语中英文混排，但不允许整段或整节仅用英文描述。

## Guardrails
- 本规则不替换任何生成式机械门禁。若中文描述要求与模板、检查器或 schema 要求的固定英文 token 冲突，以模板与检查器要求为准，并在同一文档的对应描述位置用中文补充说明。
- 本规则优先级高于生成的 Harness 规则；系统/开发者指令、安全要求、文件系统权限和用户当前明确指令始终优先。
