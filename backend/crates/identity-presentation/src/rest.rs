//! JSON REST adapter over the Identity use cases. Present alongside the gRPC
//! handlers so browsers and mobile clients can use a plain HTTP client rather
//! than a gRPC-Web layer. Field names match the proto (lower_snake_case) so
//! the generated TS types are compatible on the wire.

use crate::IdentityServices;
use audit::Actor;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use identity_application::{
    AuthenticateError, AuthenticateInput, GetUserError, GetUserInput, RefreshTokenError,
    RefreshTokenInput, RegisterUserError, RegisterUserInput, RevokeTokenError, RevokeTokenInput,
};
use identity_domain::DomainError;
use identity_domain::ports::RepositoryError;
use identity_domain::user::UserStatus as DomainStatus;
use kernel::EntityId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub fn router(services: IdentityServices) -> Router {
    Router::new()
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/authenticate", post(authenticate))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/auth/revoke", post(revoke))
        .route("/v1/users/{user_id}", get(get_user))
        .with_state(services)
}

// ---- request / response shapes — lower_snake_case matches the proto --------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RegisterBody {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RegisterResponse {
    user_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct AuthenticateBody {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: String, // RFC3339
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RefreshBody {
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct RevokeBody {
    refresh_token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct RevokeResponse {
    revoked: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct UserResponse {
    user_id: String,
    email: String,
    status: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ErrorBody {
    error: String,
}

// ---- error mapping ---------------------------------------------------------

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(ErrorBody { error: self.1 })).into_response()
    }
}

fn domain_to_api(e: &DomainError) -> ApiError {
    match e {
        DomainError::InvalidEmail | DomainError::WeakPassword => {
            ApiError(StatusCode::BAD_REQUEST, e.to_string())
        }
        DomainError::InvalidCredentials | DomainError::UserInactive => {
            ApiError(StatusCode::UNAUTHORIZED, "invalid credentials".into())
        }
    }
}

fn repo_to_api(e: &RepositoryError) -> ApiError {
    match e {
        RepositoryError::Conflict(f) => {
            ApiError(StatusCode::CONFLICT, format!("{f} already exists"))
        }
        RepositoryError::Backend(_) => ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage unavailable".into(),
        ),
    }
}

fn status_string(s: DomainStatus) -> &'static str {
    match s {
        DomainStatus::Active => "active",
        DomainStatus::Suspended => "suspended",
        DomainStatus::Deleted => "deleted",
    }
}

// ---- handlers --------------------------------------------------------------

async fn register(
    State(s): State<IdentityServices>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<RegisterResponse>, ApiError> {
    let out = s
        .register_user
        .execute(RegisterUserInput {
            email: body.email,
            password: body.password,
            actor_id: EntityId::<Actor>::from_uuid(uuid::Uuid::nil()),
        })
        .await
        .map_err(|e| match e {
            RegisterUserError::Domain(d) => domain_to_api(&d),
            RegisterUserError::EmailTaken => {
                ApiError(StatusCode::CONFLICT, "email already registered".into())
            }
            RegisterUserError::Repository(r) => repo_to_api(&r),
            RegisterUserError::Audit(_) => ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "audit ledger unavailable".into(),
            ),
        })?;
    Ok(Json(RegisterResponse {
        user_id: out.user_id.to_string(),
    }))
}

async fn authenticate(
    State(s): State<IdentityServices>,
    Json(body): Json<AuthenticateBody>,
) -> Result<Json<TokenResponse>, ApiError> {
    let out = s
        .authenticate
        .execute(AuthenticateInput {
            email: body.email,
            password: body.password,
        })
        .await
        .map_err(|e| match e {
            AuthenticateError::Domain(d) => domain_to_api(&d),
            AuthenticateError::Repository(r) => repo_to_api(&r),
            AuthenticateError::Token(_) => ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "token issuance failed".into(),
            ),
        })?;
    Ok(Json(TokenResponse {
        access_token: out.tokens.access_token,
        refresh_token: out.tokens.refresh_token,
        expires_at: out.tokens.expires_at.to_rfc3339(),
    }))
}

async fn refresh(
    State(s): State<IdentityServices>,
    Json(body): Json<RefreshBody>,
) -> Result<Json<TokenResponse>, ApiError> {
    let out = s
        .refresh_token
        .execute(RefreshTokenInput {
            refresh_token: body.refresh_token,
        })
        .await
        .map_err(|e| match e {
            RefreshTokenError::Domain(d) => domain_to_api(&d),
            RefreshTokenError::Repository(r) => repo_to_api(&r),
            RefreshTokenError::Token(_) | RefreshTokenError::Revoked => {
                ApiError(StatusCode::UNAUTHORIZED, "invalid refresh token".into())
            }
        })?;
    Ok(Json(TokenResponse {
        access_token: out.tokens.access_token,
        refresh_token: out.tokens.refresh_token,
        expires_at: out.tokens.expires_at.to_rfc3339(),
    }))
}

async fn revoke(
    State(s): State<IdentityServices>,
    Json(body): Json<RevokeBody>,
) -> Result<Json<RevokeResponse>, ApiError> {
    let out = s
        .revoke_token
        .execute(RevokeTokenInput {
            refresh_token: body.refresh_token,
        })
        .await
        .map_err(|e| match e {
            RevokeTokenError::Token(_) => {
                ApiError(StatusCode::UNAUTHORIZED, "invalid refresh token".into())
            }
        })?;
    Ok(Json(RevokeResponse {
        revoked: out.revoked,
    }))
}

async fn get_user(
    State(s): State<IdentityServices>,
    Path(user_id): Path<String>,
) -> Result<Json<UserResponse>, ApiError> {
    let id = EntityId::from_str(&user_id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "malformed user_id".into()))?;
    let out = s
        .get_user
        .execute(GetUserInput { user_id: id })
        .await
        .map_err(|e| match e {
            GetUserError::NotFound => ApiError(StatusCode::NOT_FOUND, "user not found".into()),
            GetUserError::Domain(d) => domain_to_api(&d),
            GetUserError::Repository(r) => repo_to_api(&r),
        })?;
    Ok(Json(UserResponse {
        user_id: out.user_id.to_string(),
        email: out.email,
        status: status_string(out.status).into(),
        created_at: out.created_at.to_rfc3339(),
    }))
}
