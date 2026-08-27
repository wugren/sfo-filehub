-- 归属：permissions 子模块（P-02）。项目协作角色，授权以项目为载体；项目 owner
-- 由 projects.owner 隐式持有，不写入本表。
-- project_id 外键级联：项目删除时自动清除授权（新库生效；存量库由服务层显式清理兜底）。
CREATE TABLE IF NOT EXISTS project_grants (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('read', 'write', 'admin')),
    PRIMARY KEY (project_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_project_grants_user ON project_grants(user_id);
