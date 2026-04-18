//! gRPC handler — implements the `IdentityService` trait generated from
//! `contracts/identity/v1/identity.proto`. Also served as gRPC-Web via
//! `tonic-web` at the service binary.

use crate::IdentityServices;
use audit::{Actor, hash_state};
use identity_application::{
    AuthenticateError, AuthenticateInput, GetUserError, GetUserInput, RefreshTokenError,
    RefreshTokenInput, RegisterUserError, RegisterUserInput, RevokeTokenError, RevokeTokenInput,
};
use identity_contracts::v1::{
    AuthenticateRequest, AuthenticateResponse, GetUserRequest, GetUserResponse,
    RefreshTokenRequest, RefreshTokenResponse, RegisterUserRequest, RegisterUserResponse,
    RevokeTokenRequest, RevokeTokenResponse, UserStatus,
    identity_service_server::{IdentityService, IdentityServiceServer},
};
use identity_domain::DomainError;
use identity_domain::ports::RepositoryError;
use identity_domain::user::UserStatus as DomainStatus;
use kernel::EntityId;
use prost_types::Timestamp;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct IdentityGrpc {
    services: IdentityServices,
}

impl IdentityGrpc {
    #[must_use]
    pub fn new(services: IdentityServices) -> IdentityServiceServer<Self> {
        IdentityServiceServer::new(Self { services })
    }
}

fn ts(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

fn map_status(s: DomainStatus) -> i32 {
    match s {
        DomainStatus::Active => UserStatus::Active as i32,
        DomainStatus::Suspended => UserStatus::Suspended as i32,
        DomainStatus::Deleted => UserStatus::Deleted as i32,
    }
}

fn map_domain_err(e: &DomainError) -> Status {
    match e {
        DomainError::InvalidEmail | DomainError::WeakPassword => {
            Status::invalid_argument(e.to_string())
        }
        DomainError::InvalidCredentials | DomainError::UserInactive => {
            Status::unauthenticated("invalid credentials")
        }
    }
}

fn map_repo_err(e: &RepositoryError) -> Status {
    match e {
        RepositoryError::Conflict(field) => {
            Status::already_exists(format!("{field} already exists"))
        }
        RepositoryError::Backend(_) => Status::unavailable("storage unavailable"),
    }
}

#[tonic::async_trait]
impl IdentityService for IdentityGrpc {
    async fn register_user(
        &self,
        request: Request<RegisterUserRequest>,
    ) -> Result<Response<RegisterUserResponse>, Status> {
        let req = request.into_inner();
        // Actor is "anonymous" for self-registration; a real deployment extracts
        // it from an admin bearer token for admin-initiated creates.
        let anon_actor = EntityId::<Actor>::from_uuid(uuid::Uuid::nil());
        let out = self
            .services
            .register_user
            .execute(RegisterUserInput {
                email: req.email,
                password: req.password,
                actor_id: anon_actor,
            })
            .await
            .map_err(|e| match e {
                RegisterUserError::Domain(d) => map_domain_err(&d),
                RegisterUserError::EmailTaken => Status::already_exists("email already registered"),
                RegisterUserError::Repository(r) => map_repo_err(&r),
                RegisterUserError::Audit(_) => Status::internal("audit ledger unavailable"),
            })?;
        tracing::info!(user_id = %out.user_id, hash = %hash_state(&out.user_id.to_string()), "user registered");
        Ok(Response::new(RegisterUserResponse {
            user_id: out.user_id.to_string(),
        }))
    }

    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let req = request.into_inner();
        let out = self
            .services
            .authenticate
            .execute(AuthenticateInput {
                email: req.email,
                password: req.password,
            })
            .await
            .map_err(|e| match e {
                AuthenticateError::Domain(d) => map_domain_err(&d),
                AuthenticateError::Repository(r) => map_repo_err(&r),
                AuthenticateError::Token(_) => Status::internal("token issuance failed"),
            })?;
        Ok(Response::new(AuthenticateResponse {
            access_token: out.tokens.access_token,
            refresh_token: out.tokens.refresh_token,
            expires_at: Some(ts(out.tokens.expires_at)),
        }))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        let req = request.into_inner();
        let out = self
            .services
            .refresh_token
            .execute(RefreshTokenInput {
                refresh_token: req.refresh_token,
            })
            .await
            .map_err(|e| match e {
                RefreshTokenError::Domain(d) => map_domain_err(&d),
                RefreshTokenError::Repository(r) => map_repo_err(&r),
                RefreshTokenError::Token(_) | RefreshTokenError::Revoked => {
                    Status::unauthenticated("invalid refresh token")
                }
            })?;
        Ok(Response::new(RefreshTokenResponse {
            access_token: out.tokens.access_token,
            refresh_token: out.tokens.refresh_token,
            expires_at: Some(ts(out.tokens.expires_at)),
        }))
    }

    async fn revoke_token(
        &self,
        request: Request<RevokeTokenRequest>,
    ) -> Result<Response<RevokeTokenResponse>, Status> {
        let req = request.into_inner();
        let out = self
            .services
            .revoke_token
            .execute(RevokeTokenInput {
                refresh_token: req.refresh_token,
            })
            .await
            .map_err(|e| match e {
                RevokeTokenError::Token(_) => Status::unauthenticated("invalid refresh token"),
            })?;
        Ok(Response::new(RevokeTokenResponse {
            revoked: out.revoked,
        }))
    }

    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResponse>, Status> {
        let req = request.into_inner();
        let user_id = EntityId::from_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("malformed user_id"))?;
        let out = self
            .services
            .get_user
            .execute(GetUserInput { user_id })
            .await
            .map_err(|e| match e {
                GetUserError::NotFound => Status::not_found("user not found"),
                GetUserError::Domain(d) => map_domain_err(&d),
                GetUserError::Repository(r) => map_repo_err(&r),
            })?;
        Ok(Response::new(GetUserResponse {
            user_id: out.user_id.to_string(),
            email: out.email,
            status: map_status(out.status),
            created_at: Some(ts(out.created_at)),
        }))
    }
}

/// Placeholder used to silence `unused` warnings on `Arc` when the service
/// bundle ships without wiring — kept public for forward-compat.
#[doc(hidden)]
pub fn _touch(_: &Arc<()>) {}
