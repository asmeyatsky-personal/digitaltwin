//! RS256 JWT issuer. Private key loaded from Secret Manager via env
//! (`IDENTITY_JWT_PRIVATE_KEY_PEM`). HS256 is explicitly rejected — AUDIT
//! §2.1 Critical #6 flagged prior HS256 use.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use identity_application::ports::{IssuedTokens, RefreshClaims, TokenError, TokenIssuer};
use identity_domain::user::User;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum Rs256TokenIssuerError {
    #[error("invalid PEM: {0}")]
    BadPem(String),
}

pub struct Rs256TokenIssuer {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    iss: String,
    aud: String,
    exp: i64,
    iat: i64,
    jti: String,
    typ: String,
}

impl Rs256TokenIssuer {
    /// # Errors
    /// Returns `BadPem` when either PEM input cannot be parsed as an RSA key.
    pub fn new(
        private_pem: &[u8],
        public_pem: &[u8],
        issuer: impl Into<String>,
        audience: impl Into<String>,
        access_ttl: Duration,
        refresh_ttl: Duration,
    ) -> Result<Self, Rs256TokenIssuerError> {
        let encoding_key = EncodingKey::from_rsa_pem(private_pem)
            .map_err(|e| Rs256TokenIssuerError::BadPem(e.to_string()))?;
        let decoding_key = DecodingKey::from_rsa_pem(public_pem)
            .map_err(|e| Rs256TokenIssuerError::BadPem(e.to_string()))?;
        Ok(Self {
            encoding_key,
            decoding_key,
            issuer: issuer.into(),
            audience: audience.into(),
            access_ttl,
            refresh_ttl,
        })
    }

    fn sign(
        &self,
        sub: EntityId<User>,
        jti: &str,
        now: DateTime<Utc>,
        exp: DateTime<Utc>,
        typ: &str,
    ) -> Result<String, TokenError> {
        let claims = Claims {
            sub: sub.to_string(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: jti.into(),
            typ: typ.into(),
        };
        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .map_err(|e| TokenError::Sign(e.to_string()))
    }
}

#[async_trait]
impl TokenIssuer for Rs256TokenIssuer {
    async fn issue(&self, user: &User, now: DateTime<Utc>) -> Result<IssuedTokens, TokenError> {
        let access_exp = now + self.access_ttl;
        let refresh_exp = now + self.refresh_ttl;
        let refresh_jti = Uuid::now_v7().to_string();
        let access_jti = Uuid::now_v7().to_string();
        let access = self.sign(user.id(), &access_jti, now, access_exp, "access")?;
        let refresh = self.sign(user.id(), &refresh_jti, now, refresh_exp, "refresh")?;
        Ok(IssuedTokens {
            access_token: access,
            refresh_token: refresh,
            refresh_jti,
            expires_at: access_exp,
        })
    }

    async fn verify_refresh(&self, token: &str) -> Result<RefreshClaims, TokenError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.audience]);
        validation.set_issuer(&[&self.issuer]);
        let data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| TokenError::Verify(e.to_string()))?;
        if data.claims.typ != "refresh" {
            return Err(TokenError::Verify("wrong token type".into()));
        }
        let user_id = EntityId::<User>::from_str(&data.claims.sub)
            .map_err(|e| TokenError::Verify(e.to_string()))?;
        let expires_at = DateTime::<Utc>::from_timestamp(data.claims.exp, 0)
            .ok_or_else(|| TokenError::Verify("bad exp".into()))?;
        Ok(RefreshClaims {
            user_id,
            jti: data.claims.jti,
            expires_at,
        })
    }
}
