//! Layer: application (Identity bounded context).
//! Ports: consumes `identity_domain::ports::{UserRepository, PasswordHasher}`
//! and `audit::AuditPort`. Exposes use-case handlers as public APIs.
//! MCP integration: Identity MCP tools/resources call these use cases.
//! Stack choice: canonical.
//!
//! Use cases orchestrate domain objects and ports. No HTTP, no DB details.
//! Tests use in-memory adapters (§3.2, §5 "mock ports only").

#![forbid(unsafe_code)]
#![deny(clippy::all)]

pub mod ports;
pub mod use_cases;

pub use ports::{IssuedTokens, RefreshClaims, TokenBlacklist, TokenError, TokenIssuer};
pub use use_cases::{
    authenticate::{Authenticate, AuthenticateError, AuthenticateInput, AuthenticateOutput},
    get_user::{GetUser, GetUserError, GetUserInput, GetUserOutput},
    refresh_token::{RefreshToken, RefreshTokenError, RefreshTokenInput, RefreshTokenOutput},
    register_user::{RegisterUser, RegisterUserError, RegisterUserInput, RegisterUserOutput},
    revoke_token::{RevokeToken, RevokeTokenError, RevokeTokenInput, RevokeTokenOutput},
};
