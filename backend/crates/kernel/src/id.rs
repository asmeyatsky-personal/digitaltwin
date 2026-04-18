use serde::{Deserialize, Serialize};
use std::{fmt, marker::PhantomData, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum IdError {
    #[error("invalid id: {0}")]
    Invalid(String),
}

/// Typed identifier. `T` is a zero-sized marker so `EntityId<User>` is a
/// distinct type from `EntityId<Session>` at compile time — prevents passing
/// a user id where a session id is required.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId<T: ?Sized> {
    value: Uuid,
    #[serde(skip)]
    _marker: PhantomData<fn() -> T>,
}

impl<T: ?Sized> EntityId<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            value: Uuid::now_v7(),
            _marker: PhantomData,
        }
    }

    #[must_use]
    pub fn from_uuid(value: Uuid) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }

    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.value
    }
}

impl<T: ?Sized> Default for EntityId<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> Clone for EntityId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: ?Sized> Copy for EntityId<T> {}

impl<T: ?Sized> PartialEq for EntityId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl<T: ?Sized> Eq for EntityId<T> {}

impl<T: ?Sized> std::hash::Hash for EntityId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: ?Sized> fmt::Debug for EntityId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("EntityId").field(&self.value).finish()
    }
}

impl<T: ?Sized> fmt::Display for EntityId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl<T: ?Sized> FromStr for EntityId<T> {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self::from_uuid)
            .map_err(|e| IdError::Invalid(e.to_string()))
    }
}
