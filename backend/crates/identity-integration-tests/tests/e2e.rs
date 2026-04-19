//! Identity E2E. Stands up a real Postgres, applies both migration sets,
//! generates an ephemeral RSA keypair, and drives the full
//! register → authenticate → refresh → revoke flow through the application
//! use cases. Verifies that the audit ledger fills on writes.

use std::sync::Arc;

use audit::Actor;
use chrono::{Duration as ChronoDuration, Utc};
use identity_application::{
    Authenticate, AuthenticateInput, GetUser, GetUserInput, RefreshToken, RefreshTokenError,
    RefreshTokenInput, RegisterUser, RegisterUserInput, RevokeToken, RevokeTokenInput,
};
use identity_infrastructure::{
    Argon2idHasher, PostgresAuditLedger, PostgresTokenBlacklist, PostgresUserRepository,
    Rs256TokenIssuer,
};
use kernel::{EntityId, clock::SystemClock};
use rsa::{
    RsaPrivateKey,
    pkcs1::EncodeRsaPublicKey,
    pkcs8::{EncodePrivateKey, LineEnding},
    rand_core::OsRng,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PostgresImage;

const IDENTITY_MIGRATION: &str =
    include_str!("../../../migrations/identity/20260418_0001__init.sql");
const AUDIT_MIGRATION: &str = include_str!("../../../migrations/audit/20260418_0001__init.sql");

struct Fixture {
    register: RegisterUser,
    authenticate: Authenticate,
    refresh: RefreshToken,
    revoke: RevokeToken,
    get_user: GetUser,
    pool: PgPool,
}

impl Fixture {
    async fn audit_count(&self) -> i64 {
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*)::BIGINT FROM audit.events")
            .fetch_one(&self.pool)
            .await
            .expect("count");
        n
    }
}

async fn wire(pg_url: &str) -> Fixture {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(pg_url)
        .await
        .expect("connect");

    // CITEXT needs the extension; docker image does not enable it by default.
    // Multi-statement SQL: use raw_sql so we don't go through the prepare path.
    sqlx::raw_sql("CREATE EXTENSION IF NOT EXISTS citext")
        .execute(&pool)
        .await
        .expect("citext");
    sqlx::raw_sql(IDENTITY_MIGRATION)
        .execute(&pool)
        .await
        .expect("identity migration");
    sqlx::raw_sql(AUDIT_MIGRATION)
        .execute(&pool)
        .await
        .expect("audit migration");

    // Ephemeral RSA keypair — test-only.
    let mut rng = OsRng;
    let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("rsa");
    let public_key = private_key.to_public_key();
    let private_pem = private_key.to_pkcs8_pem(LineEnding::LF).expect("pkcs8");
    let public_pem = public_key.to_pkcs1_pem(LineEnding::LF).expect("pkcs1");

    let users = Arc::new(PostgresUserRepository::new(pool.clone()));
    let hasher = Arc::new(Argon2idHasher::owasp_default());
    let tokens = Arc::new(
        Rs256TokenIssuer::new(
            private_pem.as_bytes(),
            public_pem.as_bytes(),
            "test-issuer",
            "test-audience",
            ChronoDuration::minutes(15),
            ChronoDuration::days(30),
        )
        .expect("issuer"),
    );
    let blacklist = Arc::new(PostgresTokenBlacklist::new(pool.clone()));
    let audit_ledger = Arc::new(PostgresAuditLedger::new(pool.clone()));
    let clock = Arc::new(SystemClock);

    Fixture {
        register: RegisterUser::new(
            users.clone(),
            hasher.clone(),
            audit_ledger.clone(),
            clock.clone(),
        ),
        authenticate: Authenticate::new(
            users.clone(),
            hasher.clone(),
            tokens.clone(),
            clock.clone(),
        ),
        refresh: RefreshToken::new(
            users.clone(),
            tokens.clone(),
            blacklist.clone(),
            clock.clone(),
        ),
        revoke: RevokeToken::new(tokens.clone(), blacklist.clone()),
        get_user: GetUser::new(users),
        pool,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_identity_lifecycle() {
    let node = PostgresImage::default().start().await.expect("pg up");
    let host = node.get_host().await.expect("host");
    let port = node.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let fx = wire(&url).await;

    // 1. Register.
    let reg = fx
        .register
        .execute(RegisterUserInput {
            email: "alice@example.com".into(),
            password: "correct horse battery staple".into(),
            actor_id: EntityId::<Actor>::new(),
        })
        .await
        .expect("register");

    assert_eq!(fx.audit_count().await, 1, "register emits one audit event");

    // 2. Read-side.
    let profile = fx
        .get_user
        .execute(GetUserInput {
            user_id: reg.user_id,
        })
        .await
        .expect("get_user");
    assert_eq!(profile.email, "alice@example.com");

    // 3. Authenticate — must return a well-formed token pair.
    let authed = fx
        .authenticate
        .execute(AuthenticateInput {
            email: "alice@example.com".into(),
            password: "correct horse battery staple".into(),
        })
        .await
        .expect("authenticate");

    assert!(!authed.tokens.access_token.is_empty());
    assert!(!authed.tokens.refresh_token.is_empty());
    assert!(authed.tokens.expires_at > Utc::now());

    // 4. Refresh — rotates the token pair and revokes the old refresh jti.
    let refreshed = fx
        .refresh
        .execute(RefreshTokenInput {
            refresh_token: authed.tokens.refresh_token.clone(),
        })
        .await
        .expect("refresh");
    assert_ne!(refreshed.tokens.refresh_token, authed.tokens.refresh_token);

    // 5. Old refresh token is now revoked — reuse must fail with `Revoked`.
    let reuse = fx
        .refresh
        .execute(RefreshTokenInput {
            refresh_token: authed.tokens.refresh_token.clone(),
        })
        .await;
    assert!(matches!(reuse, Err(RefreshTokenError::Revoked)));

    // 6. Revoke the current refresh token explicitly.
    fx.revoke
        .execute(RevokeTokenInput {
            refresh_token: refreshed.tokens.refresh_token.clone(),
        })
        .await
        .expect("revoke");

    let reuse_after_revoke = fx
        .refresh
        .execute(RefreshTokenInput {
            refresh_token: refreshed.tokens.refresh_token,
        })
        .await;
    assert!(matches!(
        reuse_after_revoke,
        Err(RefreshTokenError::Revoked)
    ));

    // 7. Bad credentials path — timing-safe "invalid credentials" regardless of
    // whether the email is unknown or the password is wrong.
    for (email, password) in [
        ("alice@example.com", "wrong"),
        ("unknown@example.com", "whatever"),
    ] {
        let bad = fx
            .authenticate
            .execute(AuthenticateInput {
                email: email.into(),
                password: password.into(),
            })
            .await;
        assert!(bad.is_err(), "bad credentials must fail ({email})");
    }

    // 8. Duplicate registration rejected.
    let dup = fx
        .register
        .execute(RegisterUserInput {
            email: "alice@example.com".into(),
            password: "another password".into(),
            actor_id: EntityId::<Actor>::new(),
        })
        .await;
    assert!(dup.is_err(), "duplicate email must fail");
}
