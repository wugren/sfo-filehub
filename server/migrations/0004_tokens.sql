-- 归属：tokens 子模块（P-03）。
-- token 记录含 name/project_scope/当前验签公钥；不保存签名私钥、JWT 明文与过期时间。
CREATE TABLE IF NOT EXISTS tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    project_scope TEXT NOT NULL,
    public_key_pem TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revoked_at TEXT
);
CREATE TABLE IF NOT EXISTS token_scopes (
    token_id INTEGER NOT NULL,
    scope TEXT NOT NULL,
    PRIMARY KEY (token_id, scope)
);
