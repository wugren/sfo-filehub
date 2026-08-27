//! SqliteTokenService：token 生命周期与 JWT 签发/解析（I-004）。

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::SigningKey;
use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
use serde_json::Value;
use sfo_http::token_helper::{
    Algorithm, DecodingKey, EncodingKey, JWTBuilder, JsonWebToken, Payload,
};
use sqlx::Connection;
use sqlx::Row;
use sqlx::sqlite::SqlitePool;
use std::collections::HashSet;
use std::str::FromStr;

use crate::model::{ProjectScope, Scope, ScopeSet, TokenId, TokenIssued, TokenSummary, UserId};

use super::model::{
    TokenCreateRequest, TokenError, TokenErrorKind, TokenPayload, TokenPrincipal, TokenResult,
    TokenUpdateRequest,
};

const MAX_EXPIRY: Duration = Duration::days(365);

pub struct SqliteTokenService {
    db: SqlitePool,
}

impl SqliteTokenService {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    fn generate_keypair(&self) -> TokenResult<(String, String)> {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed)
            .map_err(|e| TokenError::db(format!("generate token key failed: {e}")))?;
        let signing = SigningKey::from_bytes(&seed);
        let private_pem = signing
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| TokenError::invalid_input(format!("encode private key failed: {e}")))?
            .to_string();
        let public_pem = signing
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| TokenError::invalid_input(format!("encode public key failed: {e}")))?;
        Ok((private_pem, public_pem))
    }

    fn sign(
        &self,
        private_pem: &str,
        payload: &TokenPayload,
        expires_at: Option<DateTime<Utc>>,
    ) -> TokenResult<String> {
        let encoding = EncodingKey::from_ed_pem(private_pem.as_bytes())
            .map_err(|e| TokenError::invalid_input(format!("invalid token key: {e}")))?;
        let mut builder = JWTBuilder::new(payload.clone())
            .sub(payload.user_id.to_string())
            .jti(payload.token_id.0 as u64)
            .iat(Utc::now());
        if let Some(exp) = expires_at {
            builder = builder.exp(exp);
        }
        builder
            .build(Algorithm::EdDSA, &encoding)
            .map_err(|e| TokenError::db(format!("sign jwt failed: {e}")))
    }

    fn validate_expiry(&self, expires_at: Option<DateTime<Utc>>) -> TokenResult<()> {
        if let Some(exp) = expires_at {
            let remaining = exp - Utc::now();
            if remaining > MAX_EXPIRY {
                return Err(TokenError::invalid_input(
                    "expires_at must be at most 1 year in the future",
                ));
            }
            if remaining.num_seconds() <= 0 {
                return Err(TokenError::invalid_input(
                    "expires_at must be in the future",
                ));
            }
        }
        Ok(())
    }

    fn parse_expires(raw: Option<String>) -> Option<DateTime<Utc>> {
        raw.and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
            .map(|d| d.with_timezone(&Utc))
    }

    async fn load_token_row(
        &self,
        token_id: &TokenId,
        owner: &UserId,
    ) -> TokenResult<
        Option<(
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        )>,
    > {
        let row = sqlx::query("SELECT id, owner_id, name, project_scope, public_key_pem, created_at, updated_at, revoked_at FROM tokens WHERE id = ? AND owner_id = ?")
            .bind(token_id.0)
            .bind(owner.0)
            .fetch_optional(&self.db)
            .await?;
        Ok(match row {
            None => None,
            Some(row) => Some((
                row.try_get::<String, _>("name")?,
                row.try_get::<String, _>("project_scope")?,
                row.try_get::<String, _>("public_key_pem")?,
                row.try_get::<String, _>("created_at")?,
                row.try_get::<String, _>("updated_at")?,
                row.try_get::<Option<String>, _>("revoked_at")?,
                row.try_get::<Option<String>, _>("revoked_at")?,
            )),
        })
    }

    async fn load_scopes(&self, token_id: &TokenId) -> TokenResult<ScopeSet> {
        let rows = sqlx::query("SELECT scope FROM token_scopes WHERE token_id = ? ORDER BY scope")
            .bind(token_id.0)
            .fetch_all(&self.db)
            .await?;
        let mut set = HashSet::new();
        for row in rows {
            let raw: String = row.try_get("scope")?;
            set.insert(Scope::from_str(&raw).map_err(|e: String| {
                TokenError::invalid_input(format!("invalid scope row: {e}"))
            })?);
        }
        Ok(ScopeSet(set))
    }

    async fn replace_scopes(&self, token_id: &TokenId, scopes: &ScopeSet) -> TokenResult<()> {
        let mut tx = self.db.begin().await?;
        sqlx::query("DELETE FROM token_scopes WHERE token_id = ?")
            .bind(token_id.0)
            .execute(&mut *tx)
            .await?;
        for scope in &scopes.0 {
            sqlx::query("INSERT INTO token_scopes (token_id, scope) VALUES (?, ?)")
                .bind(token_id.0)
                .bind(scope.to_string())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    fn unverified_jti(&self, bearer: &str) -> TokenResult<TokenId> {
        let parts: Vec<&str> = bearer.split('.').collect();
        if parts.len() != 3 {
            return Err(TokenError::invalid_input("malformed token jwt"));
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| TokenError::invalid_input(format!("invalid token payload: {e}")))?;
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|e| TokenError::invalid_input(format!("invalid token payload json: {e}")))?;
        let jti = value
            .get("jti")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| TokenError::invalid_input("token payload missing jti"))?;
        Ok(TokenId(jti as i64))
    }
}

#[async_trait]
impl super::TokenService for SqliteTokenService {
    async fn create(&self, req: TokenCreateRequest) -> TokenResult<TokenIssued> {
        self.validate_expiry(req.expires_at)?;
        if req.name.trim().is_empty() {
            return Err(TokenError::invalid_input("token name required"));
        }
        let (private_pem, public_pem) = self.generate_keypair()?;
        let scopes = ScopeSet(req.scopes.iter().copied().collect());
        let project_scope = req
            .project_scope
            .clone()
            .unwrap_or(ProjectScope::All)
            .normalize();
        let now = Utc::now();
        let payload = TokenPayload {
            token_id: TokenId(0),
            user_id: req.owner,
        };
        let mut tx = self.db.begin().await?;
        let result = sqlx::query("INSERT INTO tokens (owner_id, name, project_scope, public_key_pem, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(req.owner.0)
            .bind(&req.name)
            .bind(project_scope.to_string())
            .bind(&public_pem)
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        let token_id = TokenId(result.last_insert_rowid());
        let mut payload = payload;
        payload.token_id = token_id;
        for scope in &scopes.0 {
            sqlx::query("INSERT INTO token_scopes (token_id, scope) VALUES (?, ?)")
                .bind(token_id.0)
                .bind(scope.to_string())
                .execute(&mut *tx)
                .await?;
        }
        let jwt = self.sign(&private_pem, &payload, req.expires_at)?;
        tx.commit().await?;
        // 私钥即弃：函数返回后 private_pem 不再使用、不落库、不进日志。
        Ok(TokenIssued {
            token_id,
            jwt,
            name: req.name,
            expires_at: req.expires_at,
        })
    }

    async fn list(&self, owner: &UserId) -> TokenResult<Vec<TokenSummary>> {
        let rows = sqlx::query("SELECT id, name, project_scope, created_at, updated_at FROM tokens WHERE owner_id = ? AND revoked_at IS NULL ORDER BY id")
            .bind(owner.0)
            .fetch_all(&self.db)
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let token_id = TokenId(row.try_get::<i64, _>("id")?);
            let project_scope_raw: String = row.try_get("project_scope")?;
            let project_scope =
                ProjectScope::from_str(&project_scope_raw).map_err(|e: String| {
                    TokenError::invalid_input(format!("invalid project_scope: {e}"))
                })?;
            out.push(TokenSummary {
                token_id,
                name: row.try_get("name")?,
                project_scope,
                scopes: self.load_scopes(&token_id).await?,
                created_at: Self::parse_expires(Some(row.try_get("created_at")?))
                    .unwrap_or_else(Utc::now),
                updated_at: Self::parse_expires(Some(row.try_get("updated_at")?))
                    .unwrap_or_else(Utc::now),
            });
        }
        Ok(out)
    }

    async fn update(
        &self,
        token_id: &TokenId,
        owner: &UserId,
        patch: TokenUpdateRequest,
    ) -> TokenResult<TokenSummary> {
        let Some((name, project_scope_raw, _, created_at_raw, updated_at_raw, _, _)) =
            self.load_token_row(token_id, owner).await?
        else {
            return Err(TokenError::not_found("token not found"));
        };
        let existing_project_scope =
            ProjectScope::from_str(&project_scope_raw).map_err(|e: String| {
                TokenError::invalid_input(format!("invalid stored project_scope: {e}"))
            })?;
        let existing_scopes = self.load_scopes(token_id).await?;
        let created_at = Self::parse_expires(Some(created_at_raw)).unwrap_or_else(Utc::now);
        if patch.name.is_none() && patch.project_scope.is_none() && patch.scopes.is_none() {
            // 空操作：不写库、不重签，返回当前摘要。
            return Ok(TokenSummary {
                token_id: *token_id,
                name,
                project_scope: existing_project_scope,
                scopes: existing_scopes,
                created_at,
                updated_at: Self::parse_expires(Some(updated_at_raw)).unwrap_or_else(Utc::now),
            });
        }
        let new_name = patch.name.unwrap_or(name);
        let new_project_scope = patch
            .project_scope
            .unwrap_or(existing_project_scope)
            .normalize();
        let scopes_pending = patch.scopes.is_some();
        let new_scopes = patch
            .scopes
            .map(|v| ScopeSet(v.into_iter().collect()))
            .unwrap_or_else(|| existing_scopes.clone());
        let now = Utc::now().to_rfc3339();
        let mut tx = self.db.begin().await?;
        sqlx::query("UPDATE tokens SET name = ?, project_scope = ?, updated_at = ? WHERE id = ? AND owner_id = ?")
            .bind(&new_name)
            .bind(new_project_scope.to_string())
            .bind(&now)
            .bind(token_id.0)
            .bind(owner.0)
            .execute(&mut *tx)
            .await?;
        if scopes_pending {
            // 属性修改不重签：只替换权限行，验签公钥保持不变，旧 JWT 继续有效。
            sqlx::query("DELETE FROM token_scopes WHERE token_id = ?")
                .bind(token_id.0)
                .execute(&mut *tx)
                .await?;
            for scope in &new_scopes.0 {
                sqlx::query("INSERT INTO token_scopes (token_id, scope) VALUES (?, ?)")
                    .bind(token_id.0)
                    .bind(scope.to_string())
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;
        Ok(TokenSummary {
            token_id: *token_id,
            name: new_name,
            project_scope: new_project_scope,
            scopes: new_scopes,
            created_at,
            updated_at: Self::parse_expires(Some(now)).unwrap_or_else(Utc::now),
        })
    }

    async fn rotate(&self, token_id: &TokenId, owner: &UserId) -> TokenResult<TokenIssued> {
        // 先取当前行（池连接）拿 name 与验签公钥快照：公钥只被 rotate 替换，
        // 作为 CAS 版本号，把并发 rotate/revoke 中非当前状态的调用拒掉。
        let Some((name, _project_scope_raw, current_public_pem, _, _, _, _)) =
            self.load_token_row(token_id, owner).await?
        else {
            return Err(TokenError::not_found("token not found"));
        };
        // rotate 无请求体：服务端默认不过期（exp 不写入），旧 JWT 因验签公钥替换立即失效。
        let (private_pem, new_public_pem) = self.generate_keypair()?;
        let payload = TokenPayload {
            token_id: *token_id,
            user_id: *owner,
        };
        let jwt = self.sign(&private_pem, &payload, None)?;
        // BEGIN IMMEDIATE 单写者事务 + CAS：revoke 先提交 -> 0 行（404）；
        // 并发 rotate 先提交 -> 公钥快照失配 0 行，复查后返回 409；同一状态
        // 出发的并发调用只有一个能提交，响应中的 JWT 在返回时立即可用。
        let mut conn = self.db.acquire().await?;
        let mut tx = conn.begin_with("BEGIN IMMEDIATE").await?;
        let result = sqlx::query(
            "UPDATE tokens SET public_key_pem = ?, updated_at = ? WHERE id = ? AND owner_id = ? AND revoked_at IS NULL AND public_key_pem = ?",
        )
        .bind(&new_public_pem)
        .bind(Utc::now().to_rfc3339())
        .bind(token_id.0)
        .bind(owner.0)
        .bind(&current_public_pem)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            // 在同一写事务内复查原因，避免把并发轮换误报为已撤销/不存在。
            let stale = sqlx::query("SELECT revoked_at FROM tokens WHERE id = ? AND owner_id = ?")
                .bind(token_id.0)
                .bind(owner.0)
                .fetch_optional(&mut *tx)
                .await?;
            return match stale {
                None => Err(TokenError::not_found("token not found")),
                Some(row) if row.try_get::<Option<String>, _>("revoked_at")?.is_some() => {
                    Err(TokenError::not_found("token not found or already revoked"))
                }
                _ => Err(TokenError::conflict(
                    "token was rotated concurrently; retry with current state",
                )),
            };
        }
        tx.commit().await?;
        Ok(TokenIssued {
            token_id: *token_id,
            jwt,
            name,
            expires_at: None,
        })
    }

    async fn revoke(&self, token_id: &TokenId, owner: &UserId) -> TokenResult<()> {
        let result = sqlx::query("UPDATE tokens SET revoked_at = ?, updated_at = ? WHERE id = ? AND owner_id = ? AND revoked_at IS NULL")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .bind(token_id.0)
            .bind(owner.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            return Err(TokenError::not_found("token not found or already revoked"));
        }
        Ok(())
    }

    async fn resolve(&self, bearer: &str) -> TokenResult<TokenPrincipal> {
        let token_id = self.unverified_jti(bearer)?;
        let row = sqlx::query(
            "SELECT owner_id, public_key_pem, revoked_at, project_scope FROM tokens WHERE id = ?",
        )
        .bind(token_id.0)
        .fetch_optional(&self.db)
        .await?;
        let Some(row) = row else {
            return Err(TokenError::not_found("token not found"));
        };
        let owner_id: i64 = row.try_get("owner_id")?;
        let public_key_pem: String = row.try_get("public_key_pem")?;
        let revoked_at: Option<String> = row.try_get("revoked_at")?;
        let project_scope_raw: String = row.try_get("project_scope")?;
        let project_scope = ProjectScope::from_str(&project_scope_raw).map_err(|e: String| {
            TokenError::invalid_input(format!("invalid stored project_scope: {e}"))
        })?;
        if revoked_at.is_some() {
            return Err(TokenError::not_found("token revoked"));
        }
        let decoding = DecodingKey::from_ed_pem(public_key_pem.as_bytes())
            .map_err(|e| TokenError::invalid_input(format!("invalid token key: {e}")))?;
        let header = jsonwebtoken::decode_header(bearer)
            .map_err(|e| TokenError::not_found(format!("invalid token header: {e}")))?;
        let mut validation = jsonwebtoken::Validation::new(header.alg);
        validation.validate_exp = true;
        // token 本身无过期时间：exp 可选，只有存在时才校验。
        validation.required_spec_claims = Default::default();
        let token_data =
            jsonwebtoken::decode::<Payload<TokenPayload>>(bearer, &decoding, &validation).map_err(
                |e| TokenError::not_found(format!("token signature or expiry invalid: {e}")),
            )?;
        let claims = token_data.claims;
        if claims.jti != Some(token_id.0 as u64) || claims.data.token_id != token_id {
            return Err(TokenError::invalid_input("token claims mismatch"));
        }
        if claims.data.user_id != UserId(owner_id) {
            return Err(TokenError::invalid_input("token owner claims mismatch"));
        }
        // 权限属性不在 JWT claims 中，统一以数据库为权威读取。
        let scopes = self.load_scopes(&token_id).await?;
        Ok(TokenPrincipal {
            token_id,
            user_id: claims.data.user_id,
            scopes,
            project_scope,
        })
    }
}
