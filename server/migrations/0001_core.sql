-- I-001 crate 骨架：核心 schema 元表。
-- 归属：account 子模块执行；后续 0002-0007 由各归属子模块幂等执行。
CREATE TABLE IF NOT EXISTS schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
