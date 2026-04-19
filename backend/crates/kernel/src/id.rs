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

#[cfg(test)]
mod tests {
    use super::*;

    struct Marker;

    #[test]
    fn new_produces_unique_ids() {
        let a = EntityId::<Marker>::new();
        let b = EntityId::<Marker>::new();
        assert_ne!(a, b);
    }

    #[test]
    fn round_trip_display_and_from_str() {
        let id = EntityId::<Marker>::new();
        let parsed = EntityId::<Marker>::from_str(&id.to_string()).expect("parses");
        assert_eq!(id, parsed);
        assert_eq!(id.as_uuid(), parsed.as_uuid());
    }

    #[test]
    fn invalid_input_returns_idelrror() {
        let e = EntityId::<Marker>::from_str("not-a-uuid").unwrap_err();
        assert!(matches!(e, IdError::Invalid(_)));
    }

    #[test]
    fn clone_and_debug_are_cheap() {
        let a = EntityId::<Marker>::new();
        let b = a;
        assert_eq!(a, b);
        let _ = format!("{a:?}");
    }

    #[test]
    fn default_new_are_distinct() {
        let a = EntityId::<Marker>::default();
        let b = EntityId::<Marker>::default();
        assert_ne!(a, b);
    }

    #[test]
    fn hash_and_eq_align() {
        use std::collections::HashSet;
        let a = EntityId::<Marker>::new();
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&a));
    }
}
