//! Argon2id password hasher. Parameters are OWASP 2024 minimums; increase via
//! env vars if hardware allows. Constant-time comparison is handled by
//! `argon2::PasswordHash::verify_password`.

use argon2::{
    Argon2, Params, PasswordHash, PasswordHasher as _, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use async_trait::async_trait;
use identity_domain::{
    DomainError, ports::PasswordHasher, values::PasswordHash as DomainPasswordHash,
};

pub struct Argon2idHasher {
    params: Params,
}

impl Argon2idHasher {
    /// # Errors
    /// Returns `DomainError::WeakPassword` if the caller-supplied parameters
    /// are below OWASP minimums. Default parameters are always safe.
    pub fn new(m_cost: u32, t_cost: u32, p_cost: u32) -> Result<Self, DomainError> {
        // OWASP 2024 minimums for Argon2id.
        if m_cost < 19_456 || t_cost < 2 || p_cost < 1 {
            return Err(DomainError::WeakPassword);
        }
        let params =
            Params::new(m_cost, t_cost, p_cost, None).map_err(|_| DomainError::WeakPassword)?;
        Ok(Self { params })
    }

    /// OWASP 2024 default: 19 MiB, t=2, p=1.
    ///
    /// # Panics
    /// Never — the parameters are compile-time constants known to satisfy
    /// `Argon2idHasher::new`. The `expect` is a static assertion.
    #[must_use]
    pub fn owasp_default() -> Self {
        Self::new(19_456, 2, 1).expect("OWASP defaults are valid")
    }
}

impl Default for Argon2idHasher {
    fn default() -> Self {
        Self::owasp_default()
    }
}

#[async_trait]
impl PasswordHasher for Argon2idHasher {
    async fn hash(&self, plaintext: &str) -> Result<DomainPasswordHash, DomainError> {
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            self.params.clone(),
        );
        let salt = SaltString::generate(&mut OsRng);
        let hash = argon2
            .hash_password(plaintext.as_bytes(), &salt)
            .map_err(|_| DomainError::WeakPassword)?
            .to_string();
        Ok(DomainPasswordHash::from_raw(hash))
    }

    async fn verify(
        &self,
        plaintext: &str,
        hash: &DomainPasswordHash,
    ) -> Result<bool, DomainError> {
        let parsed =
            PasswordHash::new(hash.as_str()).map_err(|_| DomainError::InvalidCredentials)?;
        let argon2 = Argon2::default();
        Ok(argon2
            .verify_password(plaintext.as_bytes(), &parsed)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_verify_round_trip() {
        let hasher = Argon2idHasher::owasp_default();
        let hash = hasher
            .hash("correct horse battery staple")
            .await
            .expect("hash");
        assert!(
            hasher
                .verify("correct horse battery staple", &hash)
                .await
                .expect("verify")
        );
        assert!(
            !hasher
                .verify("wrong password", &hash)
                .await
                .expect("verify")
        );
    }

    #[tokio::test]
    async fn rejects_subminimum_params() {
        assert_eq!(
            Argon2idHasher::new(1024, 1, 1).err(),
            Some(DomainError::WeakPassword)
        );
    }
}
