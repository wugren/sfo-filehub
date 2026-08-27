---
task_manifest: task.yaml
status: approved
approved_by: user
approved_at: 2026-08-23
user_statement: 用户 2026-08-23 回复「确认，自动完成」，设计按已确认提案执行。
---

# model 子模块设计（Principal::Token 变体）

## Design Scope

- 归属：`filehub-server` crate 共享 `model` 子模块。
- 覆盖：`Principal::Token` 变体新增 `project_scope` 字段，使权限承载结构
  同时携带 scopes 与项目范围；`Principal::user_id/display_kind` 辅助逻辑
  不变。
- 不覆盖：account/permissions/tokens/http 子模块行为。

## Module Relationship UML

```mermaid
classDiagram
  direction LR
  class Principal {
    Token(token_id, scopes, user_id, project_scope)
  }
  class scope_model {
    ProjectScope
    ScopeSet
  }
  Principal --> scope_model : 字段类型
```

## File-Level Interfaces

```rust
// server/src/model/principal.rs
pub enum Principal {
    Anonymous,
    User { user_id: UserId, account_role: AccountRole },
    Token { token_id: TokenId, scopes: ScopeSet, user_id: UserId,
            project_scope: ProjectScope },
}
```

- Consumer: `server/src/http/auth.rs`（构造）、`server/src/permissions/checker.rs`
  （消费并判定）、`server/tests/unit/permissions.rs`、
  `server/tests/unit/versions.rs`（构造点）。change_id
  `fh-token-permissions-server-side`
- Compatibility: breaking（变体新增必填字段，仓库内消费方同步迁移；无外部
  消费者）
- Migration path when required: 本任务实现/测试阶段同步更新全部构造点。

## State and Ownership

- `Principal` 为进程内请求级值类型，无持久化状态。字段类型的权限数据
  （ScopeSet/ProjectScope）由 tokens 模块持久化，checker 只读消费。
- not-applicable: 无生命周期状态机。

## Design Notes

- 保持变体字段命名与现有 `scopes` 一致；`project_scope` 无默认值，避免
  隐式放行为 `All`。
