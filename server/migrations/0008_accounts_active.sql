-- 归属：account 子模块（P-01）。users 增加 active 停用标记。
-- 幂等性说明：SQLite 不支持 ALTER TABLE ... ADD COLUMN IF NOT EXISTS，
-- 由 account/mod.rs 的 AccountModule::init 在 0002 之后以
-- PRAGMA table_info(users) 列探测守卫，仅当 active 列缺失时执行本文件；
-- 新库与已有库走同一条迁移路径。
ALTER TABLE users ADD COLUMN active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1));
