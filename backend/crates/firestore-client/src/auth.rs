//! Access-token acquisition. Two strategies:
//!
//! 1. **Metadata server** — Cloud Run and GCE. `TokenSource::metadata()` calls
//!    `http://metadata.google.internal/...` which returns a short-lived token
//!    bound to the service's Workload Identity.
//! 2. **Service-account JSON key** — local dev. The private key signs a JWT
//!    assertion that's exchanged for an access token at
//!    `https://oauth2.googleapis.com/token`.

use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

use crate::error::FirestoreError;

const METADATA_URL: &str =
    "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";
const OAUTH_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPE: &str = "https://www.googleapis.com/auth/datastore";

/// Parsed Google service-account JSON file.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountKey {
    pub client_email: String,
    pub private_key: String,
    #[serde(default)]
    pub token_uri: Option<String>,
}

impl ServiceAccountKey {
    /// # Errors
    /// Decode errors if the string is not valid service-account JSON.
    pub fn from_json(s: &str) -> Result<Self, FirestoreError> {
        serde_json::from_str(s).map_err(|e| FirestoreError::Decode(e.to_string()))
    }
}

#[derive(Clone)]
enum Strategy {
    Metadata,
    ServiceAccount(ServiceAccountKey),
}

#[derive(Clone)]
pub struct TokenSource {
    strategy: Strategy,
    cache: Arc<Mutex<Option<Cached>>>,
    client: reqwest::Client,
}

#[derive(Clone)]
struct Cached {
    token: String,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

impl TokenSource {
    #[must_use]
    pub fn metadata() -> Self {
        Self {
            strategy: Strategy::Metadata,
            cache: Arc::new(Mutex::new(None)),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("reqwest"),
        }
    }

    #[must_use]
    pub fn service_account(key: ServiceAccountKey) -> Self {
        Self {
            strategy: Strategy::ServiceAccount(key),
            cache: Arc::new(Mutex::new(None)),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("reqwest"),
        }
    }

    /// Returns a cached access token or fetches a new one. Tokens are cached
    /// for `expires_in - 30s` to avoid boundary races.
    pub async fn access_token(&self) -> Result<String, FirestoreError> {
        {
            let guard = self.cache.lock().await;
            if let Some(c) = guard.as_ref()
                && Instant::now() < c.expires_at
            {
                return Ok(c.token.clone());
            }
        }

        let (token, lifetime) = match &self.strategy {
            Strategy::Metadata => self.fetch_metadata().await?,
            Strategy::ServiceAccount(key) => self.fetch_service_account(key).await?,
        };

        let mut guard = self.cache.lock().await;
        *guard = Some(Cached {
            token: token.clone(),
            expires_at: Instant::now() + lifetime - Duration::from_secs(30),
        });
        Ok(token)
    }

    async fn fetch_metadata(&self) -> Result<(String, Duration), FirestoreError> {
        #[derive(Deserialize)]
        struct Body {
            access_token: String,
            expires_in: u64,
        }
        let resp = self
            .client
            .get(METADATA_URL)
            .header("Metadata-Flavor", "Google")
            .send()
            .await
            .map_err(|e| FirestoreError::Auth(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(FirestoreError::Auth(format!(
                "metadata http {}",
                resp.status()
            )));
        }
        let body: Body = resp
            .json()
            .await
            .map_err(|e| FirestoreError::Auth(e.to_string()))?;
        Ok((body.access_token, Duration::from_secs(body.expires_in)))
    }

    async fn fetch_service_account(
        &self,
        key: &ServiceAccountKey,
    ) -> Result<(String, Duration), FirestoreError> {
        let now = Utc::now();
        let exp = now + ChronoDuration::minutes(60);
        let claims = JwtClaims {
            iss: key.client_email.clone(),
            scope: SCOPE.into(),
            aud: key.token_uri.clone().unwrap_or_else(|| OAUTH_URL.into()),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };
        let header = Header::new(Algorithm::RS256);
        let encoding_key = EncodingKey::from_rsa_pem(key.private_key.as_bytes())
            .map_err(|e| FirestoreError::Auth(format!("bad key: {e}")))?;
        let assertion = encode(&header, &claims, &encoding_key)
            .map_err(|e| FirestoreError::Auth(e.to_string()))?;

        let resp = self
            .client
            .post(OAUTH_URL)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &assertion),
            ])
            .send()
            .await
            .map_err(|e| FirestoreError::Auth(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(FirestoreError::Auth(format!(
                "oauth http {}",
                resp.status()
            )));
        }
        let body: OAuthTokenResponse = resp
            .json()
            .await
            .map_err(|e| FirestoreError::Auth(e.to_string()))?;
        Ok((body.access_token, Duration::from_secs(body.expires_in)))
    }
}
