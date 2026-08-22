-- 归属：versions 子模块。版本为显式创建实体，可不可逆锁定；版本内具名 app 1:N。
CREATE TABLE IF NOT EXISTS versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    version TEXT NOT NULL,
    published_at TEXT NOT NULL,
    locked_at TEXT,
    UNIQUE (project_id, version)
);
CREATE TABLE IF NOT EXISTS version_apps (
    version_id INTEGER NOT NULL REFERENCES versions(id) ON DELETE CASCADE,
    app TEXT NOT NULL,
    file_id TEXT NOT NULL UNIQUE,
    sha256 TEXT NOT NULL,
    size INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (version_id, app)
);
CREATE INDEX IF NOT EXISTS idx_versions_project ON versions(project_id);
