-- 归属：storage 子模块（P-04）。文件索引与 data_dir 物理字节一一对应。
CREATE TABLE IF NOT EXISTS files (
    id TEXT PRIMARY KEY,
    rel_path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
