-- 归属：permissions 子模块（P-02）。账号角色与项目协作角色，授权以项目为载体。
CREATE TABLE IF NOT EXISTS account_roles (
    user_id INTEGER PRIMARY KEY,
    role TEXT NOT NULL CHECK (role IN ('owner', 'member'))
);
CREATE TABLE IF NOT EXISTS project_grants (
    project_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('read', 'write', 'admin')),
    PRIMARY KEY (project_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_project_grants_user ON project_grants(user_id);
